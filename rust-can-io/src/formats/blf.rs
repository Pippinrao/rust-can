//! Vector BLF reader and writer support for CAN and CAN FD log objects.

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom, Write};

use flate2::read::ZlibDecoder;

use crate::event::{
    CanFdLogEvent, CanLogEvent, Channel, Direction, LogEvent, Payload, TimestampNanos,
};

const FILE_HEADER_STRUCT_SIZE: usize = 72;
const FILE_HEADER_SIZE: usize = 144;
const OBJ_HEADER_BASE_SIZE: usize = 16;
const OBJ_HEADER_V1_SIZE: usize = 16;
const OBJ_HEADER_V2_SIZE: usize = 24;
const LOG_CONTAINER_STRUCT_SIZE: usize = 16;
const CAN_MSG_STRUCT_SIZE: usize = 16;
const CAN_FD_MSG_STRUCT_SIZE: usize = 84;
const CAN_FD_MSG_64_STRUCT_SIZE: usize = 40;
const MAX_CONTAINER_SIZE: usize = 128 * 1024;

const CAN_MESSAGE: u32 = 1;
const LOG_CONTAINER: u32 = 10;
const CAN_MESSAGE2: u32 = 86;
const CAN_FD_MESSAGE: u32 = 100;
const CAN_FD_MESSAGE_64: u32 = 101;

const NO_COMPRESSION: u16 = 0;
const ZLIB_DEFLATE: u16 = 2;

const CAN_MSG_EXT: u32 = 0x8000_0000;
const REMOTE_FLAG: u8 = 0x80;
const DIR: u8 = 0x01;

const EDL: u8 = 0x01;
const BRS: u8 = 0x02;
const ESI: u8 = 0x04;

const FD64_BRS: u32 = 0x2000;
const FD64_ESI: u32 = 0x4000;

const TIME_TEN_MICS: u32 = 0x0000_0001;
const TIME_ONE_NANS: u32 = 0x0000_0002;

/// Error returned while parsing BLF data.
#[derive(Debug)]
pub enum BlfParseError {
    /// Underlying input/output failure.
    Io(io::Error),
    /// BLF file header signature is not `LOGG`.
    InvalidFileSignature,
    /// BLF object header signature is not `LOBJ`.
    InvalidObjectSignature,
    /// A field value was unsupported or inconsistent.
    InvalidField {
        /// Name of the invalid field.
        field: &'static str,
        /// Human-readable value or reason.
        value: String,
    },
    /// Input ended before the current object could be fully decoded.
    Truncated {
        /// Context where truncation was detected.
        context: &'static str,
    },
}

impl fmt::Display for BlfParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "BLF IO error: {error}"),
            Self::InvalidFileSignature => write!(f, "BLF file header signature must be LOGG"),
            Self::InvalidObjectSignature => write!(f, "BLF object header signature must be LOBJ"),
            Self::InvalidField { field, value } => write!(f, "invalid BLF {field}: {value}"),
            Self::Truncated { context } => write!(f, "truncated BLF input while reading {context}"),
        }
    }
}

impl std::error::Error for BlfParseError {}

impl From<io::Error> for BlfParseError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Reader for Vector BLF files containing CAN and CAN FD objects.
pub struct BlfReader<R> {
    reader: R,
    start_timestamp_nanos: TimestampNanos,
    tail: Vec<u8>,
}

/// Aggregate counts from a CAN/CAN FD BLF scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlfCanStats {
    /// Total CAN and CAN FD frames scanned.
    pub messages: usize,
    /// Classical CAN frames scanned.
    pub classic: usize,
    /// CAN FD frames scanned.
    pub fd: usize,
    /// Payload bytes validated while scanning.
    pub payload_bytes: usize,
}

/// Writer for Vector BLF files containing CAN and CAN FD objects.
pub struct BlfWriter<W> {
    writer: W,
    object_buffer: Vec<u8>,
    object_count: u32,
    start_timestamp_nanos: Option<TimestampNanos>,
    stop_timestamp_nanos: Option<TimestampNanos>,
    uncompressed_size: u64,
}

impl<W: Write + Seek> BlfWriter<W> {
    /// Create a writer and emit a header that will be updated by [`Self::finish`].
    pub fn new(mut writer: W) -> Self {
        write_file_header(&mut writer, FILE_HEADER_SIZE as u64, FILE_HEADER_SIZE as u64, 0)
            .expect("in-memory BLF header write should not fail");
        Self {
            writer,
            object_buffer: Vec::with_capacity(MAX_CONTAINER_SIZE),
            object_count: 0,
            start_timestamp_nanos: None,
            stop_timestamp_nanos: None,
            uncompressed_size: FILE_HEADER_SIZE as u64,
        }
    }

    /// Write one supported log event into the BLF object buffer.
    pub fn write_event(&mut self, event: &LogEvent) -> Result<(), BlfParseError> {
        match event {
            LogEvent::Can(frame) => self.add_object(
                CAN_MESSAGE,
                &encode_can_message(frame),
                frame.timestamp_ns,
            )?,
            LogEvent::CanFd(frame) => self.add_object(
                CAN_FD_MESSAGE,
                &encode_can_fd_message(frame),
                frame.timestamp_ns,
            )?,
            _ => {}
        }
        Ok(())
    }

    /// Flush pending data and update the BLF file header.
    pub fn finish(&mut self) -> Result<(), BlfParseError> {
        self.flush_container()?;
        let file_size = self.writer.stream_position()?;
        self.writer.seek(SeekFrom::Start(0))?;
        write_file_header(
            &mut self.writer,
            file_size,
            self.uncompressed_size,
            self.object_count,
        )?;
        self.writer.seek(SeekFrom::Start(file_size))?;
        Ok(())
    }

    fn add_object(
        &mut self,
        object_type: u32,
        payload: &[u8],
        timestamp_ns: TimestampNanos,
    ) -> Result<(), BlfParseError> {
        let start = *self.start_timestamp_nanos.get_or_insert(timestamp_ns);
        self.stop_timestamp_nanos = Some(timestamp_ns);
        let relative_timestamp = timestamp_ns.saturating_sub(start).max(0) as u64;
        let header_size = OBJ_HEADER_BASE_SIZE + OBJ_HEADER_V1_SIZE;
        let object_size = header_size + payload.len();

        write_base_header_to_vec(
            &mut self.object_buffer,
            header_size as u16,
            1,
            object_size as u32,
            object_type,
        );
        push_u32(&mut self.object_buffer, TIME_ONE_NANS);
        push_u16(&mut self.object_buffer, 0);
        push_u16(&mut self.object_buffer, 0);
        push_u64(&mut self.object_buffer, relative_timestamp);
        self.object_buffer.extend_from_slice(payload);
        let padding = payload.len() % 4;
        if padding != 0 {
            self.object_buffer
                .extend(std::iter::repeat_n(0_u8, padding));
        }

        self.object_count += 1;
        if self.object_buffer.len() >= MAX_CONTAINER_SIZE {
            self.flush_container()?;
        }
        Ok(())
    }

    fn flush_container(&mut self) -> Result<(), BlfParseError> {
        if self.object_buffer.is_empty() {
            return Ok(());
        }

        let object_size =
            OBJ_HEADER_BASE_SIZE + LOG_CONTAINER_STRUCT_SIZE + self.object_buffer.len();
        let mut header = Vec::with_capacity(OBJ_HEADER_BASE_SIZE + LOG_CONTAINER_STRUCT_SIZE);
        write_base_header_to_vec(
            &mut header,
            OBJ_HEADER_BASE_SIZE as u16,
            1,
            object_size as u32,
            LOG_CONTAINER,
        );
        push_u16(&mut header, NO_COMPRESSION);
        header.extend(std::iter::repeat_n(0_u8, 6));
        push_u32(&mut header, self.object_buffer.len() as u32);
        header.extend(std::iter::repeat_n(0_u8, 4));

        self.writer.write_all(&header)?;
        self.writer.write_all(&self.object_buffer)?;
        let padding = object_size % 4;
        if padding != 0 {
            self.writer
                .write_all(&std::iter::repeat_n(0_u8, padding).collect::<Vec<_>>())?;
        }

        self.uncompressed_size +=
            (OBJ_HEADER_BASE_SIZE + LOG_CONTAINER_STRUCT_SIZE + self.object_buffer.len()) as u64;
        self.object_buffer.clear();
        Ok(())
    }
}

impl<R: Read> BlfReader<R> {
    /// Create a BLF reader after validating and consuming the file header.
    pub fn new(mut reader: R) -> Result<Self, BlfParseError> {
        let mut header = [0_u8; FILE_HEADER_STRUCT_SIZE];
        reader.read_exact(&mut header)?;
        if &header[0..4] != b"LOGG" {
            return Err(BlfParseError::InvalidFileSignature);
        }

        let header_size = read_u32(&header, 4)? as usize;
        if header_size < FILE_HEADER_STRUCT_SIZE {
            return Err(BlfParseError::InvalidField {
                field: "file header size",
                value: header_size.to_string(),
            });
        }
        let mut remaining_header = vec![0_u8; header_size - FILE_HEADER_STRUCT_SIZE];
        reader.read_exact(&mut remaining_header)?;

        let start_timestamp_nanos = systemtime_to_timestamp_nanos(&header[40..56]);
        Ok(Self {
            reader,
            start_timestamp_nanos,
            tail: Vec::new(),
        })
    }

    /// Read the remaining BLF stream into decoded log events.
    pub fn collect_events(&mut self) -> Result<Vec<LogEvent>, BlfParseError> {
        let mut events = Vec::new();
        loop {
            let mut base = [0_u8; OBJ_HEADER_BASE_SIZE];
            match self.reader.read_exact(&mut base) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            }

            let header = parse_base_header(&base)?;
            let body_len = header.object_size.checked_sub(OBJ_HEADER_BASE_SIZE).ok_or(
                BlfParseError::InvalidField {
                    field: "object size",
                    value: header.object_size.to_string(),
                },
            )?;
            let mut body = vec![0_u8; body_len];
            self.reader.read_exact(&mut body)?;

            let padding = header.object_size % 4;
            if padding != 0 {
                let mut padding_buf = vec![0_u8; padding];
                self.reader.read_exact(&mut padding_buf)?;
            }

            if header.object_type == LOG_CONTAINER {
                let data = decode_container(&body)?;
                events.extend(self.parse_container(&data)?);
            }
        }
        Ok(events)
    }

    /// Scan all CAN and CAN FD objects without allocating per-frame events.
    pub fn scan_can_stats(&mut self) -> Result<BlfCanStats, BlfParseError> {
        let mut stats = BlfCanStats::default();
        loop {
            let mut base = [0_u8; OBJ_HEADER_BASE_SIZE];
            match self.reader.read_exact(&mut base) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            }

            let header = parse_base_header(&base)?;
            let body_len = header.object_size.checked_sub(OBJ_HEADER_BASE_SIZE).ok_or(
                BlfParseError::InvalidField {
                    field: "object size",
                    value: header.object_size.to_string(),
                },
            )?;
            let mut body = vec![0_u8; body_len];
            self.reader.read_exact(&mut body)?;

            let padding = header.object_size % 4;
            if padding != 0 {
                let mut padding_buf = [0_u8; 3];
                self.reader.read_exact(&mut padding_buf[..padding])?;
            }

            if header.object_type == LOG_CONTAINER {
                let data = decode_container(&body)?;
                self.scan_container(&data, &mut stats)?;
            }
        }
        Ok(stats)
    }

    fn parse_container(&mut self, data: &[u8]) -> Result<Vec<LogEvent>, BlfParseError> {
        let mut buffer;
        let parse_data = if self.tail.is_empty() {
            data
        } else {
            buffer = Vec::with_capacity(self.tail.len() + data.len());
            buffer.extend_from_slice(&self.tail);
            buffer.extend_from_slice(data);
            &buffer
        };

        let (events, consumed) = parse_objects(parse_data, self.start_timestamp_nanos)?;
        self.tail.clear();
        self.tail.extend_from_slice(&parse_data[consumed..]);
        Ok(events)
    }

    fn scan_container(
        &mut self,
        data: &[u8],
        stats: &mut BlfCanStats,
    ) -> Result<(), BlfParseError> {
        let mut buffer;
        let parse_data = if self.tail.is_empty() {
            data
        } else {
            buffer = Vec::with_capacity(self.tail.len() + data.len());
            buffer.extend_from_slice(&self.tail);
            buffer.extend_from_slice(data);
            &buffer
        };

        let consumed = scan_objects(parse_data, self.start_timestamp_nanos, stats)?;
        self.tail.clear();
        self.tail.extend_from_slice(&parse_data[consumed..]);
        Ok(())
    }
}

#[derive(Debug)]
struct BaseHeader {
    header_size: usize,
    header_version: u16,
    object_size: usize,
    object_type: u32,
}

#[derive(Debug)]
struct ObjectHeader {
    payload_offset: usize,
    next_offset: usize,
    object_type: u32,
    timestamp_nanos: TimestampNanos,
}

#[derive(Debug)]
struct ScanObjectHeader {
    payload_offset: usize,
    next_offset: usize,
    object_type: u32,
}

fn decode_container(body: &[u8]) -> Result<Vec<u8>, BlfParseError> {
    if body.len() < LOG_CONTAINER_STRUCT_SIZE {
        return Err(BlfParseError::Truncated {
            context: "log container",
        });
    }
    let method = read_u16(body, 0)?;
    let container_data = &body[LOG_CONTAINER_STRUCT_SIZE..];
    match method {
        NO_COMPRESSION => Ok(container_data.to_vec()),
        ZLIB_DEFLATE => {
            let mut decoder = ZlibDecoder::new(container_data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(decompressed)
        }
        _ => Err(BlfParseError::InvalidField {
            field: "compression method",
            value: method.to_string(),
        }),
    }
}

fn parse_objects(
    data: &[u8],
    start_timestamp_nanos: TimestampNanos,
) -> Result<(Vec<LogEvent>, usize), BlfParseError> {
    let mut events = Vec::new();
    let mut offset = 0;

    while offset + OBJ_HEADER_BASE_SIZE <= data.len() {
        offset = match find_lobj(data, offset) {
            Some(found) => found,
            None => return Ok((events, offset)),
        };

        let base = parse_base_header(slice_at(data, offset, OBJ_HEADER_BASE_SIZE, "object header")?)?;
        if offset + base.object_size > data.len() || offset + base.header_size > data.len() {
            return Ok((events, offset));
        }

        let header = parse_object_header(data, offset, start_timestamp_nanos)?;
        if let Some(event) = parse_message_object(data, &header)? {
            events.push(event);
        }
        offset = header.next_offset;
    }

    Ok((events, offset))
}

fn scan_objects(
    data: &[u8],
    _start_timestamp_nanos: TimestampNanos,
    stats: &mut BlfCanStats,
) -> Result<usize, BlfParseError> {
    let mut offset = 0;

    while offset + OBJ_HEADER_BASE_SIZE <= data.len() {
        offset = match find_lobj(data, offset) {
            Some(found) => found,
            None => return Ok(offset),
        };

        let base = parse_base_header(slice_at(data, offset, OBJ_HEADER_BASE_SIZE, "object header")?)?;
        if offset + base.object_size > data.len() || offset + base.header_size > data.len() {
            return Ok(offset);
        }

        let header = scan_object_header(offset, &base)?;
        scan_message_object(data, &header, stats)?;
        offset = header.next_offset;
    }

    Ok(offset)
}

fn scan_message_object(
    data: &[u8],
    header: &ScanObjectHeader,
    stats: &mut BlfCanStats,
) -> Result<(), BlfParseError> {
    let payload = &data[header.payload_offset..header.next_offset];
    match header.object_type {
        CAN_MESSAGE | CAN_MESSAGE2 => scan_can_message(payload, stats),
        CAN_FD_MESSAGE => scan_can_fd_message(payload, stats),
        CAN_FD_MESSAGE_64 => scan_can_fd_64_message(data, header, stats),
        _ => Ok(()),
    }
}

fn scan_can_message(payload: &[u8], stats: &mut BlfCanStats) -> Result<(), BlfParseError> {
    if payload.len() < CAN_MSG_STRUCT_SIZE {
        return Err(BlfParseError::Truncated {
            context: "CAN message",
        });
    }
    let dlc_len = usize::from(read_u8(payload, 3)?).min(8);
    stats.messages += 1;
    stats.classic += 1;
    stats.payload_bytes += dlc_len;
    Ok(())
}

fn scan_can_fd_message(payload: &[u8], stats: &mut BlfCanStats) -> Result<(), BlfParseError> {
    if payload.len() < CAN_FD_MSG_STRUCT_SIZE {
        return Err(BlfParseError::Truncated {
            context: "CAN FD message",
        });
    }
    let valid_bytes = usize::from(read_u8(payload, 14)?).min(64);
    stats.messages += 1;
    stats.fd += 1;
    stats.payload_bytes += valid_bytes;
    Ok(())
}

fn scan_can_fd_64_message(
    data: &[u8],
    header: &ScanObjectHeader,
    stats: &mut BlfCanStats,
) -> Result<(), BlfParseError> {
    let payload = &data[header.payload_offset..header.next_offset];
    if payload.len() < CAN_FD_MSG_64_STRUCT_SIZE {
        return Err(BlfParseError::Truncated {
            context: "CAN FD 64 message",
        });
    }
    let valid_bytes = usize::from(read_u8(payload, 2)?);
    let ext_data_offset = usize::from(read_u8(payload, 35)?);
    let data_offset = if ext_data_offset == 0 {
        header.payload_offset + CAN_FD_MSG_64_STRUCT_SIZE
    } else {
        header.payload_offset + ext_data_offset
    };
    let data_len = valid_bytes.min(header.next_offset.saturating_sub(data_offset));
    if data_offset + data_len > data.len() {
        return Err(BlfParseError::Truncated {
            context: "CAN FD 64 message data",
        });
    }
    stats.messages += 1;
    stats.fd += 1;
    stats.payload_bytes += data_len;
    Ok(())
}

fn scan_object_header(offset: usize, base: &BaseHeader) -> Result<ScanObjectHeader, BlfParseError> {
    let next_offset = offset + base.object_size;
    let payload_offset = offset + base.header_size;
    if payload_offset > next_offset {
        return Err(BlfParseError::InvalidField {
            field: "object header size",
            value: base.header_size.to_string(),
        });
    }

    Ok(ScanObjectHeader {
        payload_offset,
        next_offset,
        object_type: base.object_type,
    })
}

fn parse_message_object(
    data: &[u8],
    header: &ObjectHeader,
) -> Result<Option<LogEvent>, BlfParseError> {
    let payload = &data[header.payload_offset..header.next_offset];
    let event = match header.object_type {
        CAN_MESSAGE | CAN_MESSAGE2 => Some(parse_can_message(payload, header.timestamp_nanos)?),
        CAN_FD_MESSAGE => Some(parse_can_fd_message(payload, header.timestamp_nanos)?),
        CAN_FD_MESSAGE_64 => Some(parse_can_fd_64_message(data, header)?),
        _ => None,
    };
    Ok(event)
}

fn parse_can_message(payload: &[u8], timestamp_nanos: TimestampNanos) -> Result<LogEvent, BlfParseError> {
    if payload.len() < CAN_MSG_STRUCT_SIZE {
        return Err(BlfParseError::Truncated {
            context: "CAN message",
        });
    }
    let channel = read_u16(payload, 0)?;
    let flags = read_u8(payload, 2)?;
    let dlc = read_u8(payload, 3)?;
    let arbitration_id = read_u32(payload, 4)?;
    let dlc_len = usize::from(dlc).min(8);
    let data = &payload[8..8 + dlc_len];

    Ok(LogEvent::Can(CanLogEvent {
        timestamp_ns: timestamp_nanos,
        channel: decode_channel(channel),
        arbitration_id: arbitration_id & !CAN_MSG_EXT,
        extended_id: arbitration_id & CAN_MSG_EXT != 0,
        remote_frame: flags & REMOTE_FLAG != 0,
        direction: decode_direction_from_flags(flags),
        data: Payload::from_slice(data),
    }))
}

fn parse_can_fd_message(
    payload: &[u8],
    timestamp_nanos: TimestampNanos,
) -> Result<LogEvent, BlfParseError> {
    if payload.len() < CAN_FD_MSG_STRUCT_SIZE {
        return Err(BlfParseError::Truncated {
            context: "CAN FD message",
        });
    }
    let channel = read_u16(payload, 0)?;
    let flags = read_u8(payload, 2)?;
    let dlc = read_u8(payload, 3)?;
    let arbitration_id = read_u32(payload, 4)?;
    let fd_flags = read_u8(payload, 13)?;
    let valid_bytes = usize::from(read_u8(payload, 14)?).min(64);
    let data = &payload[20..20 + valid_bytes];

    Ok(LogEvent::CanFd(CanFdLogEvent {
        timestamp_ns: timestamp_nanos,
        channel: decode_channel(channel),
        arbitration_id: arbitration_id & !CAN_MSG_EXT,
        extended_id: arbitration_id & CAN_MSG_EXT != 0,
        direction: decode_direction_from_flags(flags),
        dlc_code: dlc,
        data: Payload::from_slice(data),
        bitrate_switch: fd_flags & BRS != 0,
        error_state_indicator: fd_flags & ESI != 0,
    }))
}

fn parse_can_fd_64_message(data: &[u8], header: &ObjectHeader) -> Result<LogEvent, BlfParseError> {
    let payload = &data[header.payload_offset..header.next_offset];
    if payload.len() < CAN_FD_MSG_64_STRUCT_SIZE {
        return Err(BlfParseError::Truncated {
            context: "CAN FD 64 message",
        });
    }
    let channel = read_u8(payload, 0)?;
    let dlc = read_u8(payload, 1)?;
    let valid_bytes = usize::from(read_u8(payload, 2)?);
    let arbitration_id = read_u32(payload, 4)?;
    let fd_flags = read_u32(payload, 12)?;
    let direction = read_u8(payload, 34)?;
    let ext_data_offset = usize::from(read_u8(payload, 35)?);
    let data_offset = if ext_data_offset == 0 {
        header.payload_offset + CAN_FD_MSG_64_STRUCT_SIZE
    } else {
        header.payload_offset + ext_data_offset
    };
    let max_len = header.next_offset.saturating_sub(data_offset);
    let data_len = valid_bytes.min(max_len);
    let msg_data = &data[data_offset..data_offset + data_len];

    Ok(LogEvent::CanFd(CanFdLogEvent {
        timestamp_ns: header.timestamp_nanos,
        channel: decode_channel(u16::from(channel)),
        arbitration_id: arbitration_id & !CAN_MSG_EXT,
        extended_id: arbitration_id & CAN_MSG_EXT != 0,
        direction: if direction == 0 {
            Direction::Rx
        } else {
            Direction::Tx
        },
        dlc_code: dlc,
        data: Payload::from_slice(msg_data),
        bitrate_switch: fd_flags & FD64_BRS != 0,
        error_state_indicator: fd_flags & FD64_ESI != 0,
    }))
}

fn parse_object_header(
    data: &[u8],
    offset: usize,
    start_timestamp_nanos: TimestampNanos,
) -> Result<ObjectHeader, BlfParseError> {
    let base = parse_base_header(slice_at(data, offset, OBJ_HEADER_BASE_SIZE, "object header")?)?;
    let next_offset = offset + base.object_size;
    let header_start = offset + OBJ_HEADER_BASE_SIZE;
    let (timestamp_flags, timestamp) = match base.header_version {
        1 => {
            let header = slice_at(data, header_start, OBJ_HEADER_V1_SIZE, "object header v1")?;
            (read_u32(header, 0)?, read_u64(header, 8)?)
        }
        2 => {
            let header = slice_at(data, header_start, OBJ_HEADER_V2_SIZE, "object header v2")?;
            (read_u32(header, 0)?, read_u64(header, 8)?)
        }
        _ => {
            return Err(BlfParseError::InvalidField {
                field: "object header version",
                value: base.header_version.to_string(),
            });
        }
    };

    let payload_offset = offset + base.header_size;
    if payload_offset > next_offset {
        return Err(BlfParseError::InvalidField {
            field: "object header size",
            value: base.header_size.to_string(),
        });
    }

    Ok(ObjectHeader {
        payload_offset,
        next_offset,
        object_type: base.object_type,
        timestamp_nanos: start_timestamp_nanos + timestamp_to_nanos(timestamp_flags, timestamp),
    })
}

fn parse_base_header(data: &[u8]) -> Result<BaseHeader, BlfParseError> {
    if data.len() < OBJ_HEADER_BASE_SIZE {
        return Err(BlfParseError::Truncated {
            context: "object header",
        });
    }
    if &data[0..4] != b"LOBJ" {
        return Err(BlfParseError::InvalidObjectSignature);
    }
    Ok(BaseHeader {
        header_size: usize::from(read_u16(data, 4)?),
        header_version: read_u16(data, 6)?,
        object_size: read_u32(data, 8)? as usize,
        object_type: read_u32(data, 12)?,
    })
}

fn encode_can_message(frame: &CanLogEvent) -> [u8; CAN_MSG_STRUCT_SIZE] {
    let mut output = [0_u8; CAN_MSG_STRUCT_SIZE];
    write_u16_at(&mut output, 0, encode_channel(&frame.channel));
    output[2] = encode_direction_flag(frame.direction)
        | if frame.remote_frame { REMOTE_FLAG } else { 0 };
    output[3] = frame.data.len().min(8) as u8;
    write_u32_at(&mut output, 4, encode_arbitration_id(frame.arbitration_id, frame.extended_id));
    output[8..8 + frame.data.len().min(8)].copy_from_slice(&frame.data.as_slice()[..frame.data.len().min(8)]);
    output
}

fn encode_can_fd_message(frame: &CanFdLogEvent) -> [u8; CAN_FD_MSG_STRUCT_SIZE] {
    let mut output = [0_u8; CAN_FD_MSG_STRUCT_SIZE];
    let data_len = frame.data.len().min(64);
    write_u16_at(&mut output, 0, encode_channel(&frame.channel));
    output[2] = encode_direction_flag(frame.direction);
    output[3] = frame.dlc_code;
    write_u32_at(
        &mut output,
        4,
        encode_arbitration_id(frame.arbitration_id, frame.extended_id),
    );
    output[13] = EDL | if frame.bitrate_switch { BRS } else { 0 }
        | if frame.error_state_indicator { ESI } else { 0 };
    output[14] = data_len as u8;
    output[20..20 + data_len].copy_from_slice(&frame.data.as_slice()[..data_len]);
    output
}

fn write_file_header<W: Write>(
    writer: &mut W,
    file_size: u64,
    uncompressed_size: u64,
    object_count: u32,
) -> Result<(), BlfParseError> {
    let mut header = Vec::with_capacity(FILE_HEADER_SIZE);
    header.extend_from_slice(b"LOGG");
    push_u32(&mut header, FILE_HEADER_SIZE as u32);
    header.extend_from_slice(&[5, 0, 0, 0, 2, 6, 8, 1]);
    push_u64(&mut header, file_size);
    push_u64(&mut header, uncompressed_size);
    push_u32(&mut header, object_count);
    push_u32(&mut header, 0);
    header.extend(std::iter::repeat_n(0_u8, 32));
    header.extend(std::iter::repeat_n(0_u8, FILE_HEADER_SIZE - header.len()));
    writer.write_all(&header)?;
    Ok(())
}

fn write_base_header_to_vec(
    output: &mut Vec<u8>,
    header_size: u16,
    header_version: u16,
    object_size: u32,
    object_type: u32,
) {
    output.extend_from_slice(b"LOBJ");
    push_u16(output, header_size);
    push_u16(output, header_version);
    push_u32(output, object_size);
    push_u32(output, object_type);
}

fn encode_channel(channel: &Channel) -> u16 {
    match channel {
        Channel::Number(number) => number.saturating_add(1),
        Channel::Named(_) => 1,
    }
}

fn encode_arbitration_id(arbitration_id: u32, extended_id: bool) -> u32 {
    if extended_id {
        arbitration_id | CAN_MSG_EXT
    } else {
        arbitration_id
    }
}

fn encode_direction_flag(direction: Direction) -> u8 {
    match direction {
        Direction::Rx => 0,
        Direction::Tx => DIR,
    }
}

fn systemtime_to_timestamp_nanos(systemtime: &[u8]) -> TimestampNanos {
    let year = read_u16(systemtime, 0).unwrap_or(0);
    let month = read_u16(systemtime, 2).unwrap_or(0);
    let day = read_u16(systemtime, 6).unwrap_or(0);
    let hour = read_u16(systemtime, 8).unwrap_or(0);
    let minute = read_u16(systemtime, 10).unwrap_or(0);
    let second = read_u16(systemtime, 12).unwrap_or(0);
    let millisecond = read_u16(systemtime, 14).unwrap_or(0);
    if year == 0 || month == 0 || day == 0 {
        return 0;
    }
    let days = days_from_civil(i32::from(year), u32::from(month), u32::from(day));
    let seconds = days
        .saturating_mul(86_400)
        .saturating_add(i64::from(hour) * 3_600)
        .saturating_add(i64::from(minute) * 60)
        .saturating_add(i64::from(second));
    seconds
        .saturating_mul(1_000_000_000)
        .saturating_add(i64::from(millisecond) * 1_000_000)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as u32;
    let month_adjusted = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year =
        (153 * month_adjusted as u32 + 2) / 5 + day.saturating_sub(1);
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era) * 146_097 + i64::from(day_of_era) - 719_468
}

fn timestamp_to_nanos(flags: u32, timestamp: u64) -> TimestampNanos {
    match flags {
        TIME_TEN_MICS => (timestamp as TimestampNanos) * 10_000,
        TIME_ONE_NANS => timestamp as TimestampNanos,
        _ => timestamp as TimestampNanos,
    }
}

fn find_lobj(data: &[u8], offset: usize) -> Option<usize> {
    if data.get(offset..offset + 4) == Some(b"LOBJ") {
        return Some(offset);
    }
    let max = offset.saturating_add(8).min(data.len());
    data[offset..max]
        .windows(4)
        .position(|window| window == b"LOBJ")
        .map(|relative| offset + relative)
}

fn decode_channel(channel: u16) -> Channel {
    Channel::Number(channel.saturating_sub(1))
}

fn decode_direction_from_flags(flags: u8) -> Direction {
    if flags & DIR == 0 {
        Direction::Rx
    } else {
        Direction::Tx
    }
}

fn slice_at<'a>(
    data: &'a [u8],
    offset: usize,
    len: usize,
    context: &'static str,
) -> Result<&'a [u8], BlfParseError> {
    data.get(offset..offset + len)
        .ok_or(BlfParseError::Truncated { context })
}

fn read_u8(data: &[u8], offset: usize) -> Result<u8, BlfParseError> {
    data.get(offset)
        .copied()
        .ok_or(BlfParseError::Truncated { context: "u8" })
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, BlfParseError> {
    let bytes = slice_at(data, offset, 2, "u16")?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, BlfParseError> {
    let bytes = slice_at(data, offset, 4, "u32")?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, BlfParseError> {
    let bytes = slice_at(data, offset, 8, "u64")?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn write_u16_at(output: &mut [u8], offset: usize, value: u16) {
    output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32_at(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufReader, Cursor};
    use std::path::Path;

    use crate::event::LogEvent;

    use crate::event::{CanFdLogEvent, CanLogEvent, Channel, Direction, Payload};

    use super::{BlfCanStats, BlfReader, BlfWriter, systemtime_to_timestamp_nanos};

    #[test]
    fn reads_python_can_generated_real_blf_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("data")
            .join("generated")
            .join("real_can_canfd_10000.blf");
        let file = File::open(path).expect("real BLF fixture must exist");
        let mut reader = BlfReader::new(BufReader::new(file)).expect("valid BLF header");
        let events = reader.collect_events().expect("valid BLF body");

        let can = events
            .iter()
            .filter(|event| matches!(event, LogEvent::Can(_)))
            .count();
        let canfd = events
            .iter()
            .filter(|event| matches!(event, LogEvent::CanFd(_)))
            .count();

        assert_eq!(events.len(), 10_000);
        assert_eq!(can, 1_486);
        assert_eq!(canfd, 8_514);
    }

    #[test]
    fn writer_roundtrips_can_and_canfd_events() {
        let source = vec![
            LogEvent::Can(CanLogEvent {
                timestamp_ns: 0,
                channel: Channel::Number(0),
                arbitration_id: 0x123,
                direction: Direction::Rx,
                extended_id: false,
                remote_frame: false,
                data: Payload::from_slice(&[1, 2, 3, 4]),
            }),
            LogEvent::CanFd(CanFdLogEvent {
                timestamp_ns: 1_000,
                channel: Channel::Number(1),
                arbitration_id: 0x18ff_50e5,
                direction: Direction::Tx,
                extended_id: true,
                bitrate_switch: true,
                error_state_indicator: false,
                dlc_code: 15,
                data: Payload::from_slice(&[0xAA; 64]),
            }),
        ];

        let mut output = Cursor::new(Vec::new());
        {
            let mut writer = BlfWriter::new(&mut output);
            for event in &source {
                writer.write_event(event).expect("event should be encodable");
            }
            writer.finish().expect("writer should finish");
        }

        output.set_position(0);
        let mut reader = BlfReader::new(output).expect("written BLF header should be valid");
        let decoded = reader.collect_events().expect("written BLF body should be valid");

        assert_eq!(decoded, source);
    }

    #[test]
    fn systemtime_header_converts_to_unix_timestamp_nanos() {
        let mut systemtime = [0_u8; 16];
        systemtime[0..2].copy_from_slice(&1970_u16.to_le_bytes());
        systemtime[2..4].copy_from_slice(&1_u16.to_le_bytes());
        systemtime[6..8].copy_from_slice(&1_u16.to_le_bytes());
        assert_eq!(systemtime_to_timestamp_nanos(&systemtime), 0);

        systemtime[0..2].copy_from_slice(&2026_u16.to_le_bytes());
        systemtime[2..4].copy_from_slice(&6_u16.to_le_bytes());
        systemtime[6..8].copy_from_slice(&5_u16.to_le_bytes());
        systemtime[8..10].copy_from_slice(&12_u16.to_le_bytes());
        systemtime[10..12].copy_from_slice(&34_u16.to_le_bytes());
        systemtime[12..14].copy_from_slice(&56_u16.to_le_bytes());
        systemtime[14..16].copy_from_slice(&789_u16.to_le_bytes());
        assert_eq!(
            systemtime_to_timestamp_nanos(&systemtime),
            1_780_662_896_789_000_000
        );
    }

    #[test]
    fn scan_can_stats_matches_collect_events_for_real_blf_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("data")
            .join("generated")
            .join("real_can_canfd_10000.blf");
        let file = File::open(&path).expect("real BLF fixture must exist");
        let mut reader = BlfReader::new(BufReader::new(file)).expect("valid BLF header");
        let stats = reader.scan_can_stats().expect("scan should parse BLF body");

        let file = File::open(path).expect("real BLF fixture must exist");
        let mut reader = BlfReader::new(BufReader::new(file)).expect("valid BLF header");
        let events = reader.collect_events().expect("collect should parse BLF body");
        let classic = events
            .iter()
            .filter(|event| matches!(event, LogEvent::Can(_)))
            .count();
        let fd = events
            .iter()
            .filter(|event| matches!(event, LogEvent::CanFd(_)))
            .count();
        let payload_bytes = events
            .iter()
            .map(|event| match event {
                LogEvent::Can(frame) => frame.data.len(),
                LogEvent::CanFd(frame) => frame.data.len(),
                _ => 0,
            })
            .sum();

        assert_eq!(
            stats,
            BlfCanStats {
                messages: events.len(),
                classic,
                fd,
                payload_bytes,
            }
        );
    }
}
