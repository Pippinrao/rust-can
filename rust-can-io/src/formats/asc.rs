//! ASC log reader support.

use std::fmt;
use std::io::{self, BufRead, Write};

use crate::event::{
    CanFdLogEvent, CanLogEvent, Channel, Direction, LinLogEvent, LogEvent, Payload, UnknownEvent,
};

/// Error returned when an ASC line cannot be parsed.
#[derive(Debug)]
pub enum AscParseError {
    /// Input/output error while reading from the source.
    Io(io::Error),
    /// A field was missing from a record.
    MissingField(&'static str),
    /// A field had an invalid value.
    InvalidField {
        /// Field name.
        field: &'static str,
        /// Field value.
        value: String,
    },
}

impl fmt::Display for AscParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "ASC I/O error: {error}"),
            Self::MissingField(field) => write!(f, "ASC record is missing {field}"),
            Self::InvalidField { field, value } => {
                write!(f, "ASC field {field} has invalid value {value:?}")
            }
        }
    }
}

impl std::error::Error for AscParseError {}

impl From<io::Error> for AscParseError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Streaming ASC reader.
pub struct AscReader<R> {
    reader: R,
}

/// Aggregate counts from a CAN/CAN FD ASC scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AscCanStats {
    /// Total CAN and CAN FD frames scanned.
    pub messages: usize,
    /// Classical CAN frames scanned.
    pub classic: usize,
    /// CAN FD frames scanned.
    pub fd: usize,
    /// Payload bytes decoded while scanning.
    pub payload_bytes: usize,
}

impl<R: BufRead> AscReader<R> {
    /// Creates an ASC reader from a buffered source.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Collects all decoded log events from the source.
    pub fn collect_events(self) -> Result<Vec<LogEvent>, AscParseError> {
        self.collect_events_limit(usize::MAX)
    }

    /// Collects up to `limit` decoded log events from the source.
    pub fn collect_events_limit(self, limit: usize) -> Result<Vec<LogEvent>, AscParseError> {
        // Reuse a single `String` for the line buffer instead of
        // allocating a fresh one per iteration through `BufRead::lines`.
        // The same pattern is used by `scan_can_stats_limit` below.
        let mut events = Vec::new();
        let mut reader = self.reader;
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            if events.len() >= limit {
                break;
            }
            if let Some(event) = parse_line(&line)? {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Collects up to `limit` CAN and CAN FD frame events from the source.
    ///
    /// Non-CAN events such as LIN and metadata are still parsed for validation,
    /// but they do not count toward the returned frame limit.
    pub fn collect_can_events_limit(self, limit: usize) -> Result<Vec<LogEvent>, AscParseError> {
        let mut events = Vec::new();
        self.for_each_can_event_limit(limit, |event| events.push(event.clone()))?;
        Ok(events)
    }

    /// Parses up to `limit` CAN and CAN FD frame events, invoking `visitor` for each.
    pub fn for_each_can_event_limit<F>(
        self,
        limit: usize,
        mut visitor: F,
    ) -> Result<usize, AscParseError>
    where
        F: FnMut(&LogEvent),
    {
        let mut count = 0;
        for line in self.reader.lines() {
            if count >= limit {
                break;
            }
            if let Some(event @ (LogEvent::Can(_) | LogEvent::CanFd(_))) = parse_line(&line?)? {
                visitor(&event);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Counts up to `limit` CAN and CAN FD frame events without storing them.
    pub fn count_can_events_limit(self, limit: usize) -> Result<usize, AscParseError> {
        self.for_each_can_event_limit(limit, |_| {})
    }

    /// Scans up to `limit` CAN and CAN FD frames without allocating events.
    pub fn scan_can_stats_limit(self, limit: usize) -> Result<AscCanStats, AscParseError> {
        let mut stats = AscCanStats::default();
        let mut reader = self.reader;
        let mut line = String::new();
        while reader.read_line(&mut line)? != 0 {
            if stats.messages >= limit {
                break;
            }
            scan_can_stats_line(&line, &mut stats)?;
            line.clear();
        }
        Ok(stats)
    }
}

/// Streaming ASC writer.
pub struct AscWriter<W> {
    writer: W,
}

impl<W: Write> AscWriter<W> {
    /// Creates an ASC writer from a byte sink.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Writes one log event in canonical ASC form.
    pub fn write_event(&mut self, event: &LogEvent) -> Result<(), AscParseError> {
        match event {
            LogEvent::Can(frame) => self.write_can(frame),
            LogEvent::CanFd(frame) => self.write_canfd(frame),
            LogEvent::Lin(frame) => self.write_lin(frame),
            LogEvent::Metadata(metadata) => writeln!(self.writer, "{}", metadata.text).map_err(Into::into),
            LogEvent::Raw(raw) => writeln!(self.writer, "{}", raw.raw).map_err(Into::into),
            LogEvent::Unknown(unknown) => writeln!(self.writer, "{}", unknown.raw).map_err(Into::into),
        }
    }

    fn write_can(&mut self, frame: &CanLogEvent) -> Result<(), AscParseError> {
        let channel = format_channel(&frame.channel);
        let direction = format_direction(frame.direction);
        let id = format_arbitration_id(frame.arbitration_id, frame.extended_id);
        if frame.remote_frame {
            writeln!(
                self.writer,
                "{} {} {} {} r {:X}",
                format_timestamp(frame.timestamp_ns),
                channel,
                id,
                direction,
                frame.data.len()
            )?;
        } else {
            writeln!(
                self.writer,
                "{} {} {} {} d {:X} {}",
                format_timestamp(frame.timestamp_ns),
                channel,
                id,
                direction,
                frame.data.len(),
                format_payload(frame.data.as_slice())
            )?;
        }
        Ok(())
    }

    fn write_canfd(&mut self, frame: &CanFdLogEvent) -> Result<(), AscParseError> {
        writeln!(
            self.writer,
            "{} CANFD {} {} {} {} {} d {} {} {}",
            format_timestamp(frame.timestamp_ns),
            format_channel(&frame.channel),
            format_arbitration_id(frame.arbitration_id, frame.extended_id),
            format_direction(frame.direction),
            u8::from(frame.bitrate_switch),
            u8::from(frame.error_state_indicator),
            frame.dlc_code,
            frame.data.len(),
            format_payload(frame.data.as_slice())
        )?;
        Ok(())
    }

    fn write_lin(&mut self, frame: &LinLogEvent) -> Result<(), AscParseError> {
        write!(
            self.writer,
            "{} {} {:X} {} {} {}",
            format_timestamp(frame.timestamp_ns),
            format_channel(&frame.channel),
            frame.frame_id,
            format_direction(frame.direction),
            frame.data.len(),
            format_payload(frame.data.as_slice())
        )?;
        if let Some(checksum) = frame.checksum {
            write!(self.writer, " checksum = {checksum:02X}")?;
        }
        writeln!(self.writer)?;
        Ok(())
    }
}

/// Parses one ASC line into a log event.
pub fn parse_line(line: &str) -> Result<Option<LogEvent>, AscParseError> {
    #[cfg(feature = "profile")]
    prof_scope!("asc::parse_line");
    let trimmed = line.trim();
    if trimmed.is_empty() || is_metadata_line_without_event(trimmed) {
        return Ok(None);
    }

    // Stream the line as whitespace-separated tokens instead of
    // collecting them into a `Vec`. Each per-line parts allocation
    // (≈ 3 reallocations for 12+ tokens) is the second-largest
    // alloc source after the `BufRead::lines()` string churn.
    let mut parts = trimmed.split_whitespace();
    let Some(timestamp) = parts.next() else {
        return Ok(None);
    };

    let Some(timestamp_ns) = parse_timestamp_ns(timestamp) else {
        return Ok(None);
    };

    let Some(second) = parts.next() else {
        return Ok(None);
    };

    if second == "Start" {
        return Ok(None);
    }

    if second == "CANFD" {
        return parse_canfd(parts, timestamp_ns).map(Some);
    }

    if second.starts_with('L') {
        return parse_lin(parts, second, timestamp_ns).map(Some);
    }

    // Classic CAN: second token is the channel (decimal integer).
    // The next two tokens are the arbitration id and the direction;
    // we peek at the direction without consuming the iterator so the
    // inner parser can re-iterate from the arbitration id onward.
    if second.bytes().all(|byte| byte.is_ascii_digit()) {
        let mut peek = parts.by_ref().peekable();
        let Some(arbitration_id_str) = peek.next() else {
            return Ok(None);
        };
        if peek.peek().is_some_and(|dir| matches!(*dir, "Rx" | "Tx")) {
            return parse_classic_can(peek, second, arbitration_id_str, timestamp_ns)
                .map(Some);
        }
    }

    let kind = second.to_string();
    Ok(Some(LogEvent::Unknown(UnknownEvent {
        timestamp_ns: Some(timestamp_ns),
        kind,
        raw: trimmed.to_string(),
    })))
}

fn is_metadata_line_without_event(line: &str) -> bool {
    line.starts_with("date ")
        || line.starts_with("base ")
        || line.starts_with("internal ")
        || line.starts_with("//")
        || line.starts_with("Begin TriggerBlock")
        || line.starts_with("Begin Triggerblock")
        || line.starts_with("End TriggerBlock")
}

fn format_timestamp(timestamp_ns: i64) -> String {
    format!("{:.6}", timestamp_ns as f64 / 1_000_000_000.0)
}

fn format_direction(direction: Direction) -> &'static str {
    match direction {
        Direction::Rx => "Rx",
        Direction::Tx => "Tx",
    }
}

fn format_channel(channel: &Channel) -> String {
    match channel {
        Channel::Number(number) => number.saturating_add(1).to_string(),
        Channel::Named(name) => name.clone(),
    }
}

fn format_arbitration_id(arbitration_id: u32, extended_id: bool) -> String {
    if extended_id {
        format!("{arbitration_id:X}x")
    } else {
        format!("{arbitration_id:X}")
    }
}

fn format_payload(data: &[u8]) -> String {
    data.iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_timestamp_ns(token: &str) -> Option<i64> {
    let seconds = token.parse::<f64>().ok()?;
    Some((seconds * 1_000_000_000.0).round() as i64)
}

fn parse_direction(token: &str) -> Result<Direction, AscParseError> {
    match token {
        "Rx" => Ok(Direction::Rx),
        "Tx" => Ok(Direction::Tx),
        _ => Err(AscParseError::InvalidField {
            field: "direction",
            value: token.to_string(),
        }),
    }
}

fn parse_hex_u32(field: &'static str, token: &str) -> Result<(u32, bool), AscParseError> {
    let extended = token.ends_with('x') || token.ends_with('X');
    let value_token = token.trim_end_matches(['x', 'X']);
    u32::from_str_radix(value_token, 16)
        .map(|value| (value, extended))
        .map_err(|_| AscParseError::InvalidField {
            field,
            value: token.to_string(),
        })
}

fn parse_hex_byte(field: &'static str, token: &str) -> Result<u8, AscParseError> {
    u8::from_str_radix(token, 16).map_err(|_| AscParseError::InvalidField {
        field,
        value: token.to_string(),
    })
}

fn parse_decimal_u16(field: &'static str, token: &str) -> Result<u16, AscParseError> {
    token.parse::<u16>().map_err(|_| AscParseError::InvalidField {
        field,
        value: token.to_string(),
    })
}

fn parse_asc_channel(token: &str) -> Result<Channel, AscParseError> {
    parse_decimal_u16("channel", token).map(|channel| Channel::Number(channel.saturating_sub(1)))
}

fn parse_payload<'a, I>(mut tokens: I, len: usize) -> Result<Payload, AscParseError>
where
    I: Iterator<Item = &'a str>,
{
    #[cfg(feature = "profile")]
    prof_scope!("asc::parse_payload");
    // `SmallVec<[u8; 8]>` keeps classical-CAN payloads (≤ 8 bytes)
    // on the stack with no heap allocation. For longer payloads it
    // spills to the heap.
    let mut bytes: smallvec::SmallVec<[u8; 8]> =
        smallvec::SmallVec::with_capacity(len);
    for _ in 0..len {
        let token = tokens.next().ok_or(AscParseError::MissingField("data"))?;
        bytes.push(parse_hex_byte("data", token)?);
    }
    if bytes.len() != len {
        return Err(AscParseError::MissingField("data"));
    }
    Ok(Payload::from_smallvec(bytes))
}

fn scan_can_stats_line(line: &str, stats: &mut AscCanStats) -> Result<(), AscParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() || is_metadata_line_without_event(trimmed) {
        return Ok(());
    }

    let mut parts = trimmed.split_whitespace();
    let Some(timestamp) = parts.next() else {
        return Ok(());
    };
    if parse_timestamp_ns(timestamp).is_none() {
        return Ok(());
    }

    let Some(second) = parts.next() else {
        return Ok(());
    };
    if second == "Start" || second.starts_with('L') {
        return Ok(());
    }

    if second == "CANFD" {
        scan_canfd_stats(parts, stats)
    } else if second.bytes().all(|byte| byte.is_ascii_digit()) {
        scan_classic_can_stats(second, parts, stats)
    } else {
        Ok(())
    }
}

fn scan_classic_can_stats<'a>(
    channel: &str,
    mut parts: impl Iterator<Item = &'a str>,
    stats: &mut AscCanStats,
) -> Result<(), AscParseError> {
    let _channel = parse_asc_channel(channel)?;
    let arbitration_id = parts
        .next()
        .ok_or(AscParseError::MissingField("arbitration_id"))?;
    let _ = parse_hex_u32("arbitration_id", arbitration_id)?;
    let direction = parts.next().ok_or(AscParseError::MissingField("direction"))?;
    let _ = parse_direction(direction)?;
    let frame_kind = parts.next().ok_or(AscParseError::MissingField("frame kind"))?;
    let dlc = parts.next().ok_or(AscParseError::MissingField("dlc"))?;
    let dlc = usize::from(parse_hex_byte("dlc", dlc)?);
    if !frame_kind.eq_ignore_ascii_case("r") {
        scan_payload_tokens(parts, dlc.min(8))?;
        stats.payload_bytes += dlc.min(8);
    }
    stats.messages += 1;
    stats.classic += 1;
    Ok(())
}

fn scan_canfd_stats<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    stats: &mut AscCanStats,
) -> Result<(), AscParseError> {
    let channel = parts.next().ok_or(AscParseError::MissingField("channel"))?;
    let _channel = parse_asc_channel(channel)?;
    let arbitration_id = parts
        .next()
        .ok_or(AscParseError::MissingField("arbitration_id"))?;
    let _ = parse_hex_u32("arbitration_id", arbitration_id)?;
    let direction = parts.next().ok_or(AscParseError::MissingField("direction"))?;
    let _ = parse_direction(direction)?;
    let brs = parts.next().ok_or(AscParseError::MissingField("brs"))?;
    let esi = parts.next().ok_or(AscParseError::MissingField("esi"))?;
    if !matches!(brs, "0" | "1") {
        return Err(AscParseError::InvalidField {
            field: "brs",
            value: brs.to_string(),
        });
    }
    if !matches!(esi, "0" | "1") {
        return Err(AscParseError::InvalidField {
            field: "esi",
            value: esi.to_string(),
        });
    }
    let frame_kind = parts.next().ok_or(AscParseError::MissingField("frame kind"))?;
    if !frame_kind.eq_ignore_ascii_case("d") {
        return Err(AscParseError::InvalidField {
            field: "frame kind",
            value: frame_kind.to_string(),
        });
    }
    let dlc = parts.next().ok_or(AscParseError::MissingField("dlc"))?;
    let _dlc = dlc.parse::<u8>().map_err(|_| AscParseError::InvalidField {
        field: "dlc",
        value: dlc.to_string(),
    })?;
    let data_len = parts
        .next()
        .ok_or(AscParseError::MissingField("data length"))?
        .parse::<usize>()
        .map_err(|_| AscParseError::InvalidField {
            field: "data length",
            value: dlc.to_string(),
        })?;
    scan_payload_tokens(parts, data_len)?;
    stats.messages += 1;
    stats.fd += 1;
    stats.payload_bytes += data_len;
    Ok(())
}

fn scan_payload_tokens<'a>(
    mut tokens: impl Iterator<Item = &'a str>,
    len: usize,
) -> Result<(), AscParseError> {
    for _ in 0..len {
        let token = tokens.next().ok_or(AscParseError::MissingField("data"))?;
        let _ = parse_hex_byte("data", token)?;
    }
    Ok(())
}

fn parse_classic_can<'a, I>(
    mut parts: I,
    channel_str: &'a str,
    arbitration_id_str: &'a str,
    timestamp_ns: i64,
) -> Result<LogEvent, AscParseError>
where
    I: Iterator<Item = &'a str>,
{
    #[cfg(feature = "profile")]
    prof_scope!("asc::parse_classic_can");
    let channel = parse_asc_channel(channel_str)?;
    let (arbitration_id, extended_id) =
        parse_hex_u32("arbitration_id", arbitration_id_str)?;
    let direction = parse_direction(
        parts.next().ok_or(AscParseError::MissingField("direction"))?,
    )?;
    let frame_kind = parts.next().ok_or(AscParseError::MissingField("frame kind"))?;
    let dlc =
        parse_hex_byte("dlc", parts.next().ok_or(AscParseError::MissingField("dlc"))?)?
            as usize;
    let remote_frame = frame_kind.eq_ignore_ascii_case("r");
    let data = if remote_frame {
        Payload::default()
    } else {
        parse_payload(parts, dlc.min(8))?
    };

    Ok(LogEvent::Can(CanLogEvent {
        timestamp_ns,
        channel,
        arbitration_id,
        direction,
        extended_id,
        remote_frame,
        data,
    }))
}

fn parse_canfd<'a, I>(mut parts: I, timestamp_ns: i64) -> Result<LogEvent, AscParseError>
where
    I: Iterator<Item = &'a str>,
{
    #[cfg(feature = "profile")]
    prof_scope!("asc::parse_canfd");
    let channel = parse_asc_channel(parts.next().ok_or(AscParseError::MissingField("channel"))?)?;
    let (arbitration_id, extended_id) = parse_hex_u32(
        "arbitration_id",
        parts.next().ok_or(AscParseError::MissingField("arbitration_id"))?,
    )?;
    let direction =
        parse_direction(parts.next().ok_or(AscParseError::MissingField("direction"))?)?;
    let bitrate_switch_str = parts.next().ok_or(AscParseError::MissingField("brs"))?;
    let error_state_indicator_str = parts.next().ok_or(AscParseError::MissingField("esi"))?;
    // One token between `esi` and `dlc` is reserved (the frame-kind
    // marker "d" carried over from the classic CAN line format).
    let _ = parts.next();
    let dlc_str = parts.next().ok_or(AscParseError::MissingField("dlc"))?;
    let dlc_code = dlc_str.parse::<u8>().map_err(|_| AscParseError::InvalidField {
        field: "dlc",
        value: dlc_str.to_string(),
    })?;
    let data_len_str = parts
        .next()
        .ok_or(AscParseError::MissingField("data length"))?;
    let data_len = data_len_str
        .parse::<usize>()
        .map_err(|_| AscParseError::InvalidField {
            field: "data length",
            value: data_len_str.to_string(),
        })?;
    let data = parse_payload(parts, data_len)?;

    Ok(LogEvent::CanFd(CanFdLogEvent {
        timestamp_ns,
        channel,
        arbitration_id,
        direction,
        extended_id,
        bitrate_switch: bitrate_switch_str == "1",
        error_state_indicator: error_state_indicator_str == "1",
        dlc_code,
        data,
    }))
}

fn parse_lin<'a, I>(mut parts: I, channel_str: &'a str, timestamp_ns: i64) -> Result<LogEvent, AscParseError>
where
    I: Iterator<Item = &'a str>,
{
    // `channel_str` is the L11/L* token from the line; the iterator
    // resumes at the frame id.
    let channel = channel_str.to_string();
    let frame_id = parse_hex_byte(
        "frame_id",
        parts.next().ok_or(AscParseError::MissingField("frame_id"))?,
    )?;
    let direction =
        parse_direction(parts.next().ok_or(AscParseError::MissingField("direction"))?)?;
    let data_len_str = parts
        .next()
        .ok_or(AscParseError::MissingField("data length"))?;
    let data_len = data_len_str
        .parse::<usize>()
        .map_err(|_| AscParseError::InvalidField {
            field: "data length",
            value: data_len_str.to_string(),
        })?;
    // Collect remaining tokens: LIN frames optionally end with
    // `checksum = XX`; the data is everything before that marker.
    // LIN frames are rare, so a single small `Vec` is fine here.
    let remaining: Vec<&str> = parts.collect();
    let checksum_index = remaining.iter().position(|token| *token == "checksum");
    let data_end = checksum_index.unwrap_or(remaining.len());
    let data = parse_payload(remaining[..data_end].iter().copied(), data_len)?;
    let checksum = match checksum_index {
        Some(idx) => Some(parse_hex_byte(
            "checksum",
            remaining
                .get(idx + 2)
                .ok_or(AscParseError::MissingField("checksum"))?,
        )?),
        None => None,
    };

    Ok(LogEvent::Lin(LinLogEvent {
        timestamp_ns,
        channel: Channel::Named(channel),
        frame_id,
        direction,
        data,
        checksum,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{CanFdLogEvent, CanLogEvent, Channel, Direction, LinLogEvent, LogEvent, Payload};
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    #[test]
    fn parses_classic_can_data_line() {
        let event = parse_line("0.000080 2 1D1 Rx d 8 00 00 00 00 F8 00 82 D9")
            .expect("line should parse")
            .expect("line should be an event");

        match event {
            LogEvent::Can(frame) => {
                assert_eq!(frame.timestamp_ns, 80_000);
                assert_eq!(frame.channel, Channel::Number(1));
                assert_eq!(frame.arbitration_id, 0x1d1);
                assert_eq!(frame.direction, Direction::Rx);
                assert!(!frame.extended_id);
                assert_eq!(frame.data.as_slice(), &[0, 0, 0, 0, 0xf8, 0, 0x82, 0xd9]);
            }
            _ => panic!("expected CAN event"),
        }
    }

    #[test]
    fn parses_current_canfd_dialect_with_decimal_dlc_code() {
        let event = parse_line(
            "0.000000 CANFD 6 637 Rx 0 0 d 10 16 00 20 00 00 00 00 00 00 00 00 00 00 00 00 00 00",
        )
        .expect("line should parse")
        .expect("line should be an event");

        match event {
            LogEvent::CanFd(frame) => {
                assert_eq!(frame.channel, Channel::Number(5));
                assert_eq!(frame.arbitration_id, 0x637);
                assert_eq!(frame.dlc_code, 10);
                assert_eq!(frame.data.len(), 16);
                assert_eq!(frame.data.as_slice()[1], 0x20);
            }
            _ => panic!("expected CAN FD event"),
        }
    }

    #[test]
    fn parses_lin_line_with_checksum() {
        let event = parse_line("0.000030 L11 1 Rx 8 00 4F 3F FF FF C0 FE FE checksum = 00")
            .expect("line should parse")
            .expect("line should be an event");

        match event {
            LogEvent::Lin(frame) => {
                assert_eq!(frame.timestamp_ns, 30_000);
                assert_eq!(frame.channel, Channel::Named("L11".to_string()));
                assert_eq!(frame.frame_id, 1);
                assert_eq!(frame.direction, Direction::Rx);
                assert_eq!(frame.checksum, Some(0));
                assert_eq!(frame.data.as_slice(), &[0, 0x4f, 0x3f, 0xff, 0xff, 0xc0, 0xfe, 0xfe]);
            }
            _ => panic!("expected LIN event"),
        }
    }

    #[test]
    fn keeps_unknown_event_for_future_formats() {
        let event = parse_line("0.001000 FlexRay 1 2 3")
            .expect("line should parse")
            .expect("line should be an event");

        match event {
            LogEvent::Unknown(unknown) => {
                assert_eq!(unknown.timestamp_ns, Some(1_000_000));
                assert_eq!(unknown.kind, "FlexRay");
                assert_eq!(unknown.raw, "0.001000 FlexRay 1 2 3");
            }
            _ => panic!("expected unknown event"),
        }
    }

    #[test]
    fn streams_events_without_returning_header_lines() {
        let input = concat!(
            "date Fri May 29 09:08:31.774 AM 2026\n",
            "base hex timestamps absolute\n",
            "internal events logged\n",
            "Begin TriggerBlock Fri May 29 09:08:31.774 AM 2026\n",
            "0.000000 Start of measurement\n",
            "0.000080 2 1D1 Rx d 8 00 00 00 00 F8 00 82 D9\n",
            "0.000030 L11 1 Rx 8 00 4F 3F FF FF C0 FE FE checksum = 00\n",
        );
        let reader = AscReader::new(Cursor::new(input));
        let events = reader.collect_events().expect("reader should succeed");

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], LogEvent::Can(_)));
        assert!(matches!(events[1], LogEvent::Lin(_)));
    }

    #[test]
    fn collect_events_limit_stops_after_requested_count() {
        let input = concat!(
            "0.100000 1 123 Rx d 1 aa\n",
            "0.200000 1 124 Rx d 1 bb\n",
        );
        let reader = AscReader::new(Cursor::new(input));
        let events = reader
            .collect_events_limit(1)
            .expect("reader should succeed");
        assert_eq!(events.len(), 1);
        match &events[0] {
            LogEvent::Can(frame) => assert_eq!(frame.arbitration_id, 0x123),
            _ => panic!("expected CAN event"),
        }
    }

    #[test]
    fn collect_can_events_limit_ignores_lin_for_python_can_comparable_counts() {
        let input = concat!(
            "0.100000 L11 1 Rx 1 aa checksum = 00\n",
            "0.200000 1 123 Rx d 1 bb\n",
            "0.300000 1 124 Rx d 1 cc\n",
        );
        let reader = AscReader::new(Cursor::new(input));
        let events = reader
            .collect_can_events_limit(1)
            .expect("reader should succeed");
        assert_eq!(events.len(), 1);
        match &events[0] {
            LogEvent::Can(frame) => assert_eq!(frame.arbitration_id, 0x123),
            _ => panic!("expected CAN event"),
        }
    }

    #[test]
    fn count_can_events_limit_matches_comparable_frame_count_without_storage() {
        let input = concat!(
            "0.100000 L11 1 Rx 1 aa checksum = 00\n",
            "0.200000 1 123 Rx d 1 bb\n",
            "0.300000 CANFD 1 124 Rx 1 0 d 9 12 01 02 03 04 05 06 07 08 09 0A 0B 0C\n",
        );
        let reader = AscReader::new(Cursor::new(input));
        assert_eq!(reader.count_can_events_limit(10).unwrap(), 2);
    }

    #[test]
    fn numeric_can_channels_are_internal_zero_based_and_written_one_based() {
        let event = LogEvent::Can(CanLogEvent {
            timestamp_ns: 0,
            channel: Channel::Number(0),
            arbitration_id: 0x123,
            direction: Direction::Rx,
            extended_id: false,
            remote_frame: false,
            data: Payload::from_slice(&[0xAA]),
        });
        let mut output = Vec::new();
        AscWriter::new(&mut output).write_event(&event).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(" 1 123 Rx "));
        let parsed = AscReader::new(Cursor::new(text)).collect_events().unwrap();
        assert_eq!(parsed, vec![event]);
    }

    #[test]
    fn scan_can_stats_limit_counts_and_validates_payload_without_event_storage() {
        let input = concat!(
            "0.100000 L11 1 Rx 1 aa checksum = 00\n",
            "0.200000 1 123 Rx d 2 aa bb\n",
            "0.300000 CANFD 1 124 Rx 1 0 d 9 12 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F 10 11 12\n",
            "0.400000 1 125 Rx d 1 cc\n",
        );
        let stats = AscReader::new(Cursor::new(input))
            .scan_can_stats_limit(2)
            .expect("scan should parse valid CAN/CANFD");
        assert_eq!(
            stats,
            AscCanStats {
                messages: 2,
                classic: 1,
                fd: 1,
                payload_bytes: 14,
            }
        );
    }

    #[test]
    fn scan_can_stats_limit_reports_bad_payload_tokens() {
        let input = "0.200000 1 123 Rx d 2 aa zz\n";
        let error = AscReader::new(Cursor::new(input))
            .scan_can_stats_limit(10)
            .expect_err("invalid hex payload should fail");
        assert!(matches!(error, AscParseError::InvalidField { field: "data", .. }));
    }

    #[test]
    fn parses_smallest_real_asc_corpus_file() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let Some(path) = smallest_asc_file(&workspace.join("data/extracted")) else {
            eprintln!(
                "skip parses_smallest_real_asc_corpus_file: data/extracted contains no ASC files"
            );
            return;
        };
        let file = File::open(&path).expect("real ASC file should open");
        let events = AscReader::new(BufReader::new(file))
            .collect_events()
            .expect("real ASC file should parse");
        let classic = events.iter().filter(|event| matches!(event, LogEvent::Can(_))).count();
        let canfd = events.iter().filter(|event| matches!(event, LogEvent::CanFd(_))).count();
        let lin = events.iter().filter(|event| matches!(event, LogEvent::Lin(_))).count();

        assert_eq!(classic, 14);
        assert_eq!(canfd, 63);
        assert_eq!(lin, 2);
        assert_eq!(events.len(), 79);
    }

    #[test]
    fn writer_roundtrips_can_canfd_and_lin_events() {
        let events = vec![
            LogEvent::Can(CanLogEvent {
                timestamp_ns: 80_000,
                channel: Channel::Number(2),
                arbitration_id: 0x1d1,
                direction: Direction::Rx,
                extended_id: false,
                remote_frame: false,
                data: Payload::from_slice(&[0, 0, 0, 0, 0xf8, 0, 0x82, 0xd9]),
            }),
            LogEvent::CanFd(CanFdLogEvent {
                timestamp_ns: 0,
                channel: Channel::Number(6),
                arbitration_id: 0x637,
                direction: Direction::Rx,
                extended_id: false,
                bitrate_switch: false,
                error_state_indicator: false,
                dlc_code: 10,
                data: Payload::from_slice(&[0x20; 16]),
            }),
            LogEvent::Lin(LinLogEvent {
                timestamp_ns: 30_000,
                channel: Channel::Named("L11".to_string()),
                frame_id: 1,
                direction: Direction::Rx,
                data: Payload::from_slice(&[0, 0x4f, 0x3f, 0xff, 0xff, 0xc0, 0xfe, 0xfe]),
                checksum: Some(0),
            }),
        ];

        let mut output = Vec::new();
        {
            let mut writer = AscWriter::new(&mut output);
            for event in &events {
                writer.write_event(event).expect("event should write");
            }
        }
        let text = String::from_utf8(output).expect("ASC output should be utf8");
        let parsed = AscReader::new(Cursor::new(text))
            .collect_events()
            .expect("written ASC should parse");

        assert_eq!(parsed, events);
    }

    fn smallest_asc_file(root: &Path) -> Option<PathBuf> {
        let mut files = Vec::new();
        collect_asc_files(root, &mut files);
        files.into_iter().min_by_key(|path| path.metadata().map(|meta| meta.len()).unwrap_or(u64::MAX))
    }

    fn collect_asc_files(dir: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_asc_files(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "asc") {
                files.push(path);
            }
        }
    }
}
