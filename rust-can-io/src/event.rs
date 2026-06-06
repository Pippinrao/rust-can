//! Log-level events used by streaming readers and writers.

/// Timestamp represented as nanoseconds from the log's time origin.
pub type TimestampNanos = i64;

/// Frame direction recorded in a log file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Frame was received by the logging interface.
    Rx,
    /// Frame was transmitted by the logging interface.
    Tx,
}

/// Logical channel identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    /// Numeric CAN channel.
    Number(u16),
    /// Named or prefixed channel, such as `L11` for LIN records.
    Named(String),
}

/// Owned payload bytes for log events.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Payload {
    data: Vec<u8>,
}

impl Payload {
    /// Copies bytes into an owned payload.
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    /// Returns the payload as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Returns the payload length in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true when the payload is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl From<Vec<u8>> for Payload {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

/// Classical CAN record from a log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanLogEvent {
    /// Timestamp in nanoseconds.
    pub timestamp_ns: TimestampNanos,
    /// Logical channel.
    pub channel: Channel,
    /// Arbitration identifier.
    pub arbitration_id: u32,
    /// Receive or transmit direction.
    pub direction: Direction,
    /// Whether the arbitration ID is 29-bit extended.
    pub extended_id: bool,
    /// Whether this is a remote frame.
    pub remote_frame: bool,
    /// Data bytes.
    pub data: Payload,
}

/// CAN FD record from a log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanFdLogEvent {
    /// Timestamp in nanoseconds.
    pub timestamp_ns: TimestampNanos,
    /// Logical channel.
    pub channel: Channel,
    /// Arbitration identifier.
    pub arbitration_id: u32,
    /// Receive or transmit direction.
    pub direction: Direction,
    /// Whether the arbitration ID is 29-bit extended.
    pub extended_id: bool,
    /// Whether bitrate switch was enabled.
    pub bitrate_switch: bool,
    /// Whether the error state indicator bit was set.
    pub error_state_indicator: bool,
    /// CAN FD DLC code as stored in the log.
    pub dlc_code: u8,
    /// Data bytes.
    pub data: Payload,
}

/// LIN record from a log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinLogEvent {
    /// Timestamp in nanoseconds.
    pub timestamp_ns: TimestampNanos,
    /// Logical LIN channel.
    pub channel: Channel,
    /// LIN frame identifier.
    pub frame_id: u8,
    /// Receive or transmit direction.
    pub direction: Direction,
    /// Data bytes.
    pub data: Payload,
    /// Optional checksum byte.
    pub checksum: Option<u8>,
}

/// Metadata event such as trigger block, comments, or measurement markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataEvent {
    /// Optional timestamp in nanoseconds.
    pub timestamp_ns: Option<TimestampNanos>,
    /// Metadata kind.
    pub kind: String,
    /// Metadata text.
    pub text: String,
}

/// Raw event preserved for roundtrip when the format is known but not decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEvent {
    /// Optional timestamp in nanoseconds.
    pub timestamp_ns: Option<TimestampNanos>,
    /// Raw text or object bytes encoded as text for now.
    pub raw: String,
}

/// Unknown event reserved for future bus formats or vendor-specific records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEvent {
    /// Optional timestamp in nanoseconds.
    pub timestamp_ns: Option<TimestampNanos>,
    /// Best-effort event kind.
    pub kind: String,
    /// Original raw record.
    pub raw: String,
}

/// Streaming log event.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LogEvent {
    /// Classical CAN frame.
    Can(CanLogEvent),
    /// CAN FD frame.
    CanFd(CanFdLogEvent),
    /// LIN frame.
    Lin(LinLogEvent),
    /// Metadata record.
    Metadata(MetadataEvent),
    /// Raw record.
    Raw(RawEvent),
    /// Unknown record for future extension.
    Unknown(UnknownEvent),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_event_keeps_classic_fields() {
        let event = LogEvent::Can(CanLogEvent {
            timestamp_ns: 42,
            channel: Channel::Number(2),
            arbitration_id: 0x1d1,
            direction: Direction::Rx,
            extended_id: false,
            remote_frame: false,
            data: Payload::from_slice(&[0, 1, 2, 3]),
        });

        match event {
            LogEvent::Can(frame) => {
                assert_eq!(frame.timestamp_ns, 42);
                assert_eq!(frame.channel, Channel::Number(2));
                assert_eq!(frame.arbitration_id, 0x1d1);
                assert_eq!(frame.direction, Direction::Rx);
                assert_eq!(frame.data.as_slice(), &[0, 1, 2, 3]);
            }
            _ => panic!("expected classic CAN event"),
        }
    }

    #[test]
    fn canfd_event_distinguishes_dlc_code_and_payload_length() {
        let event = CanFdLogEvent {
            timestamp_ns: 0,
            channel: Channel::Number(6),
            arbitration_id: 0x637,
            direction: Direction::Rx,
            extended_id: false,
            bitrate_switch: false,
            error_state_indicator: false,
            dlc_code: 10,
            data: Payload::from_slice(&[0x20; 16]),
        };

        assert_eq!(event.dlc_code, 10);
        assert_eq!(event.data.len(), 16);
    }

    #[test]
    fn lin_event_is_not_forced_into_can_message() {
        let event = LogEvent::Lin(LinLogEvent {
            timestamp_ns: 30_000,
            channel: Channel::Named("L11".to_string()),
            frame_id: 0x01,
            direction: Direction::Rx,
            data: Payload::from_slice(&[0x00, 0x4f, 0x3f]),
            checksum: Some(0),
        });

        match event {
            LogEvent::Lin(frame) => {
                assert_eq!(frame.channel, Channel::Named("L11".to_string()));
                assert_eq!(frame.frame_id, 1);
                assert_eq!(frame.checksum, Some(0));
            }
            _ => panic!("expected LIN event"),
        }
    }

    #[test]
    fn unknown_event_preserves_raw_line_for_future_formats() {
        let event = LogEvent::Unknown(UnknownEvent {
            timestamp_ns: Some(1000),
            kind: "FlexRay".to_string(),
            raw: "0.001 FlexRay payload".to_string(),
        });

        match event {
            LogEvent::Unknown(unknown) => {
                assert_eq!(unknown.timestamp_ns, Some(1000));
                assert_eq!(unknown.kind, "FlexRay");
                assert_eq!(unknown.raw, "0.001 FlexRay payload");
            }
            _ => panic!("expected unknown event"),
        }
    }
}
