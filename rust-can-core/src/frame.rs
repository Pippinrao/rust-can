/// Low-level CAN frame representation.
///
/// While [`CanMessage`](crate::message::CanMessage) is the high-level API,
/// [`CanFrame`] represents raw frames as they appear on the wire or
/// as received from hardware adapters.
use bytes::Bytes;

use crate::message::CanMessage;

/// A raw CAN frame as it would appear on the wire or from hardware.
///
/// This is the primary unit of exchange between adapters and the bus layer.
/// Adapters produce and consume `CanFrame`s; the bus layer converts between
/// `CanFrame` and `CanMessage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanFrame {
    /// CAN identifier (raw, without mask).
    pub can_id: u32,
    /// Data payload.
    pub data: Bytes,
    /// Frame flags (see [`FrameFlags`]).
    pub flags: FrameFlags,
    /// Timestamp in nanoseconds.
    pub timestamp: u64,
    /// Channel/interface identifier.
    pub channel: u16,
}

/// Bit flags for frame metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameFlags {
    /// Whether this is an extended (29-bit) ID frame.
    pub extended: bool,
    /// Whether this is a remote transmission request.
    pub remote: bool,
    /// Whether this is an error frame.
    pub error: bool,
    /// Whether this is a CAN FD frame.
    pub fd: bool,
    /// Bitrate switch (CAN FD only).
    pub brs: bool,
    /// Error state indicator (CAN FD only).
    pub esi: bool,
    /// Whether this is a CAN XL frame.
    pub xl: bool,
}

impl FrameFlags {
    /// Create from a packed flags byte (same layout as `CanMessage.flags`).
    pub fn from_byte(byte: u8) -> Self {
        use crate::message::flags;
        Self {
            extended: byte & flags::EXTENDED_ID != 0,
            remote: byte & flags::REMOTE_FRAME != 0,
            error: byte & flags::ERROR_FRAME != 0,
            fd: byte & flags::FD_FRAME != 0,
            brs: byte & flags::BITRATE_SWITCH != 0,
            esi: byte & flags::ERROR_STATE_INDICATOR != 0,
            xl: byte & flags::XL_FRAME != 0,
        }
    }

    /// Pack into a single byte (same layout as `CanMessage.flags`).
    pub fn to_byte(&self) -> u8 {
        use crate::message::flags;
        let mut b = 0u8;
        if self.extended { b |= flags::EXTENDED_ID; }
        if self.remote { b |= flags::REMOTE_FRAME; }
        if self.error { b |= flags::ERROR_FRAME; }
        if self.fd { b |= flags::FD_FRAME; }
        if self.brs { b |= flags::BITRATE_SWITCH; }
        if self.esi { b |= flags::ERROR_STATE_INDICATOR; }
        if self.xl { b |= flags::XL_FRAME; }
        b
    }
}

impl CanFrame {
    /// Create a new CAN 2.0 data frame.
    pub fn new_data(can_id: u32, data: impl Into<Bytes>, extended: bool) -> Self {
        Self {
            can_id,
            data: data.into(),
            flags: FrameFlags {
                extended,
                ..Default::default()
            },
            timestamp: 0,
            channel: 0,
        }
    }

    /// Create a new CAN FD data frame.
    pub fn new_fd(can_id: u32, data: impl Into<Bytes>, extended: bool, brs: bool) -> Self {
        Self {
            can_id,
            data: data.into(),
            flags: FrameFlags {
                extended,
                fd: true,
                brs,
                ..Default::default()
            },
            timestamp: 0,
            channel: 0,
        }
    }

    /// Create a new CAN XL data frame.
    pub fn new_xl(can_id: u32, data: impl Into<Bytes>) -> Self {
        Self {
            can_id,
            data: data.into(),
            flags: FrameFlags {
                extended: true,
                fd: true,
                xl: true,
                ..Default::default()
            },
            timestamp: 0,
            channel: 0,
        }
    }

    /// Create a new remote frame.
    pub fn new_remote(can_id: u32, _dlc: u8, extended: bool) -> Self {
        Self {
            can_id,
            data: Bytes::new(),
            flags: FrameFlags {
                extended,
                remote: true,
                ..Default::default()
            },
            timestamp: 0,
            channel: 0,
        }
    }

    /// Set the timestamp.
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }

    /// Set the channel.
    pub fn with_channel(mut self, channel: u16) -> Self {
        self.channel = channel;
        self
    }
}

// ---- Conversion between CanFrame and CanMessage ----

impl From<CanFrame> for CanMessage {
    fn from(frame: CanFrame) -> Self {
        let mut data = [0u8; 64];
        let dlc = frame.data.len().min(64) as u8;
        data[..dlc as usize].copy_from_slice(&frame.data[..dlc as usize]);

        let flags = frame.flags.to_byte() | crate::message::flags::RX_FRAME;

        CanMessage {
            timestamp: frame.timestamp,
            arbitration_id: frame.can_id,
            data,
            dlc,
            flags,
            channel: frame.channel,
        }
    }
}

impl From<CanMessage> for CanFrame {
    fn from(msg: CanMessage) -> Self {
        let dlc = msg.dlc.min(64) as usize;
        Self {
            can_id: msg.arbitration_id,
            data: Bytes::copy_from_slice(&msg.data[..dlc]),
            flags: FrameFlags::from_byte(msg.flags),
            timestamp: msg.timestamp,
            channel: msg.channel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_to_message_roundtrip() {
        let frame = CanFrame::new_data(0x123, Bytes::from(vec![1, 2, 3]), false)
            .with_timestamp(1000);
        let msg: CanMessage = frame.clone().into();
        let frame2: CanFrame = msg.into();
        assert_eq!(frame.can_id, frame2.can_id);
        assert_eq!(frame.data, frame2.data);
        assert_eq!(frame.timestamp, frame2.timestamp);
    }

    #[test]
    fn frame_flags_pack_and_unpack_all_bits() {
        let flags = FrameFlags {
            extended: true,
            remote: true,
            error: true,
            fd: true,
            brs: true,
            esi: true,
            xl: true,
        };
        assert_eq!(FrameFlags::from_byte(flags.to_byte()), flags);
    }

    #[test]
    fn fd_xl_remote_and_channel_builders_set_expected_fields() {
        let fd = CanFrame::new_fd(0x18ff_50e5, Bytes::from_static(&[0xAA; 16]), true, true)
            .with_channel(2);
        assert!(fd.flags.extended);
        assert!(fd.flags.fd);
        assert!(fd.flags.brs);
        assert_eq!(fd.channel, 2);

        let xl = CanFrame::new_xl(0x123, Bytes::from_static(&[0x55; 128]));
        assert!(xl.flags.extended);
        assert!(xl.flags.fd);
        assert!(xl.flags.xl);

        let remote = CanFrame::new_remote(0x321, 8, false);
        assert!(remote.flags.remote);
        assert!(remote.data.is_empty());
    }

    #[test]
    fn conversion_truncates_wire_frame_payload_to_inline_message_capacity() {
        let frame = CanFrame::new_fd(0x123, Bytes::from(vec![0x11; 128]), false, false);
        let msg: CanMessage = frame.into();
        assert_eq!(msg.dlc, 64);
        assert_eq!(msg.data_slice(), &[0x11; 64]);
    }
}
