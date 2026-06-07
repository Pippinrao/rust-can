//! Log-level events used by streaming readers and writers.

use smallvec::SmallVec;

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

/// Inline capacity for the small-payload buffer in [`Payload`].
///
/// `8` covers every classical CAN frame (max DLC 8 bytes) and the vast
/// majority of CAN FD frames; the smallvec spills to the heap on larger
/// payloads. The constant is kept `pub` so downstream code can document
/// the threshold in their own error messages if needed.
pub const PAYLOAD_INLINE_CAPACITY: usize = 8;

/// Owned payload bytes for log events.
///
/// The backing storage is a `SmallVec<[u8; 8]>`: up to eight bytes stay
/// inline (no heap allocation), anything larger is heap-allocated.
/// Classical CAN and most CAN FD frames therefore never touch the
/// allocator.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Payload {
    data: SmallVec<[u8; PAYLOAD_INLINE_CAPACITY]>,
}

impl Payload {
    /// Copies bytes into an owned payload.
    #[inline]
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: SmallVec::from_slice(data),
        }
    }

    /// Returns the payload as a byte slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Returns the payload length in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true when the payload is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// True if the payload is stored on the stack (no heap allocation).
    #[inline]
    pub fn is_inline(&self) -> bool {
        self.data.len() <= self.data.inline_size()
    }

    /// Consume the payload and return its bytes as a `Vec<u8>`. Useful
    /// when handing the bytes to an API that requires owned storage.
    #[inline]
    pub fn into_bytes(self) -> SmallVec<[u8; PAYLOAD_INLINE_CAPACITY]> {
        self.data
    }
}

impl From<Vec<u8>> for Payload {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self {
            data: SmallVec::from_vec(data),
        }
    }
}

impl Payload {
    /// Construct from a `SmallVec` already populated by the caller. Used
    /// by hot paths in the ASC parser to skip a `Vec` round-trip when
    /// the smallvec would never spill to the heap.
    #[inline]
    pub fn from_smallvec(data: SmallVec<[u8; PAYLOAD_INLINE_CAPACITY]>) -> Self {
        Self { data }
    }
}

impl<'a> From<&'a [u8]> for Payload {
    #[inline]
    fn from(data: &'a [u8]) -> Self {
        Self::from_slice(data)
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

    #[test]
    fn payload_inlines_up_to_eight_bytes() {
        // Classical CAN (≤8 bytes) stays on the stack with no
        // heap allocation; `is_inline` reports true.
        let p = Payload::from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(p.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(p.len(), 8);
        assert!(!p.is_empty());
        assert!(p.is_inline());
    }

    #[test]
    fn payload_spills_to_heap_above_eight_bytes() {
        let p = Payload::from_slice(&[0u8; 64]);
        assert_eq!(p.len(), 64);
        assert!(!p.is_inline());
        // `into_bytes` returns the underlying SmallVec; the
        // surrounding `Payload` is dropped at end of test, so the
        // owned bytes are reclaimed cleanly.
        let bytes = p.into_bytes();
        assert_eq!(bytes.len(), 64);
    }

    #[test]
    fn empty_payload_reports_empty() {
        let p = Payload::from_slice(&[]);
        assert!(p.is_empty());
        // `from_slice` of an empty slice still keeps the inline
        // storage with zero length, so `is_inline` is true.
        assert!(p.is_inline());
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn payload_from_vec_uses_smallvec_storage() {
        let v: Vec<u8> = vec![1, 2, 3];
        let p = Payload::from(v);
        assert_eq!(p.as_slice(), &[1, 2, 3]);
        assert!(p.is_inline());
    }

    #[test]
    fn channel_named_carries_string() {
        let ch = Channel::Named("L11".to_string());
        match ch {
            Channel::Named(s) => assert_eq!(s, "L11"),
            _ => panic!("expected Named"),
        }
    }
}
