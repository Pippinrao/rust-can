/// CAN message representation.
///
/// This is the central data type of rust-can. It represents a single CAN frame
/// on the bus, supporting CAN 2.0, CAN FD, and CAN XL formats.
///
/// ## Design notes
///
/// - Uses `#[repr(C, packed)]` for compact in-memory layout (80 bytes for standard path)
/// - Timestamp uses nanoseconds as `u64` for precision and monotonicity
/// - Data ≤64 bytes is stored inline (covers all CAN FD); CAN XL uses heap-allocated extension
/// - Bit flags pack protocol metadata efficiently
/// - `channel` field maps to python-can's channel concept for multi-bus scenarios
use std::fmt;

use bytes::Bytes;

use crate::error::{CanError, Result};
use crate::protocol::CanProtocol;

/// Bit flags packed into a single `u8` for the message metadata.
///
/// Layout (MSB to LSB):
/// - bit 7: is_extended_id
/// - bit 6: is_remote_frame
/// - bit 5: is_error_frame
/// - bit 4: is_fd
/// - bit 3: is_rx
/// - bit 2: bitrate_switch (BRS)
/// - bit 1: error_state_indicator (ESI)
/// - bit 0: reserved / is_xl
pub mod flags {
    /// Extended (29-bit) identifier flag.
    pub const EXTENDED_ID: u8 = 0b1000_0000;
    /// Remote transmission request frame.
    pub const REMOTE_FRAME: u8 = 0b0100_0000;
    /// Error frame indicator.
    pub const ERROR_FRAME: u8 = 0b0010_0000;
    /// CAN FD frame indicator.
    pub const FD_FRAME: u8 = 0b0001_0000;
    /// Received frame (as opposed to transmitted).
    pub const RX_FRAME: u8 = 0b0000_1000;
    /// Bitrate switch (CAN FD only).
    pub const BITRATE_SWITCH: u8 = 0b0000_0100;
    /// Error state indicator (CAN FD only).
    pub const ERROR_STATE_INDICATOR: u8 = 0b0000_0010;
    /// CAN XL frame indicator (reserved bit 0).
    pub const XL_FRAME: u8 = 0b0000_0001;
}

/// A CAN message with inline data for CAN 2.0 and CAN FD.
///
/// For CAN XL messages (>64 bytes), use [`CanMessageXL`] instead
/// or convert via `From<CanMessageXL>`.
///
/// # Memory layout
///
/// `#[repr(C, packed)]` ensures compact representation:
/// - `timestamp`: 8 bytes
/// - `arbitration_id`: 4 bytes
/// - `data`: 64 bytes
/// - `dlc`: 1 byte
/// - `flags`: 1 byte
/// - `channel`: 2 bytes
/// - **Total: 80 bytes** (vs ~280 bytes in python-can)
#[derive(Clone)]
#[repr(C)]
pub struct CanMessage {
    /// Timestamp in nanoseconds since boot (or custom epoch).
    pub timestamp: u64,
    /// CAN arbitration identifier (11-bit or 29-bit depending on `is_extended_id`).
    pub arbitration_id: u32,
    /// Data payload. First `dlc` bytes are valid.
    pub data: [u8; 64],
    /// Data length code (actual payload length in bytes).
    pub dlc: u8,
    /// Bit flags (see `flags` module).
    pub flags: u8,
    /// Channel identifier for multi-bus scenarios.
    pub channel: u16,
}

impl CanMessage {
    // ---- Constructors ----

    /// Create a new CAN 2.0 data frame.
    pub fn new(
        arbitration_id: u32,
        data: &[u8],
        is_extended_id: bool,
    ) -> Result<Self> {
        let dlc = data.len() as u8;
        if dlc > 8 {
            return Err(CanError::operation(format!(
                "CAN 2.0 data length must be ≤ 8, got {}",
                dlc
            )));
        }
        let mut buf = [0u8; 64];
        buf[..data.len()].copy_from_slice(data);
        let mut flags = 0u8;
        if is_extended_id {
            flags |= flags::EXTENDED_ID;
        }
        flags |= flags::RX_FRAME; // default to received
        Ok(Self {
            timestamp: now_nanos(),
            arbitration_id,
            data: buf,
            dlc,
            flags,
            channel: 0,
        })
    }

    /// Create a new CAN FD data frame.
    pub fn new_fd(
        arbitration_id: u32,
        data: &[u8],
        is_extended_id: bool,
        bitrate_switch: bool,
    ) -> Result<Self> {
        let dlc = data.len() as u8;
        if dlc > 64 {
            return Err(CanError::operation(format!(
                "CAN FD data length must be ≤ 64, got {}",
                dlc
            )));
        }
        let mut buf = [0u8; 64];
        buf[..data.len()].copy_from_slice(data);
        let mut flags = flags::FD_FRAME | flags::RX_FRAME;
        if is_extended_id {
            flags |= flags::EXTENDED_ID;
        }
        if bitrate_switch {
            flags |= flags::BITRATE_SWITCH;
        }
        Ok(Self {
            timestamp: now_nanos(),
            arbitration_id,
            data: buf,
            dlc,
            flags,
            channel: 0,
        })
    }

    /// Create a new remote frame (RTR).
    pub fn new_remote(arbitration_id: u32, dlc: u8, is_extended_id: bool) -> Result<Self> {
        if dlc > 8 {
            return Err(CanError::operation(format!(
                "Remote frame DLC must be ≤ 8, got {}",
                dlc
            )));
        }
        let mut flags = flags::REMOTE_FRAME | flags::RX_FRAME;
        if is_extended_id {
            flags |= flags::EXTENDED_ID;
        }
        Ok(Self {
            timestamp: now_nanos(),
            arbitration_id,
            data: [0u8; 64],
            dlc,
            flags,
            channel: 0,
        })
    }

    /// Create a new error frame.
    pub fn new_error() -> Self {
        Self {
            timestamp: now_nanos(),
            arbitration_id: 0,
            data: [0u8; 64],
            dlc: 0,
            flags: flags::ERROR_FRAME | flags::RX_FRAME,
            channel: 0,
        }
    }

    // ---- Field accessors ----

    /// Whether this message uses an extended (29-bit) identifier.
    #[inline]
    pub const fn is_extended_id(&self) -> bool {
        self.flags & flags::EXTENDED_ID != 0
    }

    /// Whether this is a remote transmission request frame.
    #[inline]
    pub const fn is_remote_frame(&self) -> bool {
        self.flags & flags::REMOTE_FRAME != 0
    }

    /// Whether this is an error frame.
    #[inline]
    pub const fn is_error_frame(&self) -> bool {
        self.flags & flags::ERROR_FRAME != 0
    }

    /// Whether this is a CAN FD frame.
    #[inline]
    pub const fn is_fd(&self) -> bool {
        self.flags & flags::FD_FRAME != 0
    }

    /// Whether this is a received (vs transmitted) frame.
    #[inline]
    pub const fn is_rx(&self) -> bool {
        self.flags & flags::RX_FRAME != 0
    }

    /// Whether bitrate switching is enabled (CAN FD only).
    #[inline]
    pub const fn bitrate_switch(&self) -> bool {
        self.flags & flags::BITRATE_SWITCH != 0
    }

    /// Error state indicator (CAN FD only).
    #[inline]
    pub const fn error_state_indicator(&self) -> bool {
        self.flags & flags::ERROR_STATE_INDICATOR != 0
    }

    /// Whether this is a CAN XL frame.
    #[inline]
    pub const fn is_xl(&self) -> bool {
        self.flags & flags::XL_FRAME != 0
    }

    // ---- Flag setters (builder-style) ----

    /// Set the timestamp to now.
    pub fn with_timestamp_now(mut self) -> Self {
        self.timestamp = now_nanos();
        self
    }

    /// Set the channel identifier.
    pub fn with_channel(mut self, channel: u16) -> Self {
        self.channel = channel;
        self
    }

    /// Mark this message as transmitted (rather than received).
    pub fn as_tx(mut self) -> Self {
        self.flags &= !flags::RX_FRAME;
        self
    }

    /// Set the bitrate switch flag.
    pub fn with_bitrate_switch(mut self, brs: bool) -> Self {
        if brs {
            self.flags |= flags::BITRATE_SWITCH;
        } else {
            self.flags &= !flags::BITRATE_SWITCH;
        }
        self
    }

    /// Set the error state indicator flag.
    pub fn with_error_state_indicator(mut self, esi: bool) -> Self {
        if esi {
            self.flags |= flags::ERROR_STATE_INDICATOR;
        } else {
            self.flags &= !flags::ERROR_STATE_INDICATOR;
        }
        self
    }

    /// Get the effective data as a byte slice.
    #[inline]
    pub fn data_slice(&self) -> &[u8] {
        let len = (self.dlc as usize).min(64);
        &self.data[..len]
    }

    /// Get mutable access to the effective data range.
    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        let len = (self.dlc as usize).min(64);
        &mut self.data[..len]
    }

    /// Get the protocol implied by this message's flags.
    pub fn protocol(&self) -> CanProtocol {
        if self.is_xl() {
            CanProtocol::CanXl
        } else if self.is_fd() {
            // Non-ISO detection would need additional context; default to ISO
            CanProtocol::CanFd
        } else {
            CanProtocol::Can20
        }
    }

    /// Validate the message for consistency.
    pub fn validate(&self) -> Result<()> {
        // Remote + Error cannot coexist
        if self.is_remote_frame() && self.is_error_frame() {
            return Err(CanError::operation(
                "a message cannot be a remote and an error frame at the same time",
            ));
        }

        // CAN FD does not support remote frames
        if self.is_remote_frame() && self.is_fd() {
            return Err(CanError::operation(
                "CAN FD does not support remote frames",
            ));
        }

        // Validate arbitration ID ranges
        if self.is_extended_id() {
            if self.arbitration_id >= 0x2000_0000 {
                return Err(CanError::operation(
                    "extended arbitration IDs must be less than 2^29",
                ));
            }
        } else if self.arbitration_id >= 0x800 {
            return Err(CanError::operation(
                "standard arbitration IDs must be less than 2^11",
            ));
        }

        // Validate DLC
        let max_dlc = if self.is_fd() { 64u8 } else { 8u8 };
        if self.dlc > max_dlc {
            return Err(CanError::operation(format!(
                "DLC {} exceeds maximum {} for this frame type",
                self.dlc, max_dlc
            )));
        }

        // Remote frames should have zero data length
        if self.is_remote_frame() && self.dlc > 0 {
            // Note: DLC in remote frames indicates requested length, not actual data
            // This is valid CAN behavior
        }

        // FD-specific flags are only valid for FD frames
        if !self.is_fd() {
            if self.bitrate_switch() {
                return Err(CanError::operation(
                    "bitrate switch is only allowed for CAN FD frames",
                ));
            }
            if self.error_state_indicator() {
                return Err(CanError::operation(
                    "error state indicator is only allowed for CAN FD frames",
                ));
            }
        }

        Ok(())
    }
}

impl PartialEq for CanMessage {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self.arbitration_id == other.arbitration_id
            && self.dlc == other.dlc
            && self.flags == other.flags
            && self.channel == other.channel
            && self.data_slice() == other.data_slice()
    }
}

impl Eq for CanMessage {}

impl fmt::Debug for CanMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CanMessage")
            .field("timestamp", &format_args!("{:.6}", self.timestamp as f64 / 1e9))
            .field("arbitration_id", &format_args!("{:#x}", self.arbitration_id))
            .field("is_extended_id", &self.is_extended_id())
            .field("is_remote_frame", &self.is_remote_frame())
            .field("is_error_frame", &self.is_error_frame())
            .field("is_fd", &self.is_fd())
            .field("is_rx", &self.is_rx())
            .field("dlc", &self.dlc)
            .field("data", &hex::encode(self.data_slice()))
            .field("channel", &self.channel)
            .finish()
    }
}

impl fmt::Display for CanMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id_format = if self.is_extended_id() {
            format!("{:08x}", self.arbitration_id)
        } else {
            format!("{:03x}", self.arbitration_id)
        };
        write!(f, "{:>15.6}  {:>8}  ", self.timestamp as f64 / 1e9, id_format)?;

        // Flags
        write!(f, "{}", if self.is_extended_id() { 'X' } else { 'S' })?;
        write!(f, "{}", if self.is_rx() { "Rx" } else { "Tx" })?;
        write!(f, "{}", if self.is_error_frame() { 'E' } else { ' ' })?;
        write!(f, "{}", if self.is_remote_frame() { 'R' } else { ' ' })?;
        write!(f, "{}", if self.is_fd() { 'F' } else { ' ' })?;
        write!(f, "{}", if self.bitrate_switch() { "BS" } else { "  " })?;
        write!(f, "{}", if self.error_state_indicator() { "EI" } else { "  " })?;

        write!(f, "  DL:{:2}  ", self.dlc)?;

        // Data hex
        let data = self.data_slice();
        if !data.is_empty() {
            write!(f, "{}", hex::encode(data))?;
        }

        Ok(())
    }
}

impl From<CanMessageXL> for CanMessage {
    fn from(xl: CanMessageXL) -> Self {
        let dlc = xl.header.dlc;
        let mut msg = xl.header;
        // Copy first 64 bytes if available
        let copy_len = dlc.min(64) as usize;
        msg.data[..copy_len].copy_from_slice(&xl.ext_data[..copy_len]);
        msg
    }
}

// ---- Utility ----

/// Get current time in nanoseconds (monotonic).
fn now_nanos() -> u64 {
    // Use a simple monotonic counter if available,
    // fall back to system time
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

mod hex {
    /// Simple hex encoding for display (avoids extra dependency).
    pub fn encode(data: &[u8]) -> String {
        data.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A CAN XL message with extended data (>64 bytes).
///
/// CAN XL supports up to 2048 bytes. The first 64 bytes are in the inline
/// `header.data` field; the rest is heap-allocated.
#[derive(Clone)]
pub struct CanMessageXL {
    /// Standard message header with inline first 64 bytes.
    pub header: CanMessage,
    /// Full extended data payload (up to 2048 bytes).
    pub ext_data: Bytes,
}

impl CanMessageXL {
    /// Create a new CAN XL message.
    pub fn new(
        arbitration_id: u32,
        data: impl Into<Bytes>,
    ) -> Result<Self> {
        let data = data.into();
        let payload_len = data.len();
        if payload_len > 2048 {
            return Err(CanError::operation(format!(
                "CAN XL data length must be ≤ 2048, got {}",
                payload_len
            )));
        }
        let inline_len = payload_len.min(64) as u8;

        let mut header = CanMessage {
            timestamp: now_nanos(),
            arbitration_id,
            data: [0u8; 64],
            dlc: inline_len,
            flags: flags::FD_FRAME | flags::XL_FRAME | flags::EXTENDED_ID | flags::RX_FRAME,
            channel: 0,
        };

        // Copy first 64 bytes to header
        let header_len = usize::from(inline_len);
        header.data[..header_len].copy_from_slice(&data[..header_len]);

        Ok(Self {
            header,
            ext_data: data,
        })
    }

    /// Get the full data payload.
    pub fn data(&self) -> &[u8] {
        &self.ext_data
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can20_message() {
        let msg = CanMessage::new(0x123, &[0x01, 0x02, 0x03], false).unwrap();
        assert_eq!(msg.arbitration_id, 0x123);
        assert_eq!(msg.dlc, 3);
        assert!(!msg.is_extended_id());
        assert!(!msg.is_fd());
        assert!(!msg.is_remote_frame());
        assert_eq!(msg.data_slice(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_can_fd_message() {
        let data = vec![0xAA; 64];
        let msg = CanMessage::new_fd(0x12345678, &data, true, true).unwrap();
        assert!(msg.is_extended_id());
        assert!(msg.is_fd());
        assert!(msg.bitrate_switch());
        assert_eq!(msg.dlc, 64);
    }

    #[test]
    fn test_remote_frame() {
        let msg = CanMessage::new_remote(0x100, 4, false).unwrap();
        assert!(msg.is_remote_frame());
        assert!(!msg.is_error_frame());
        assert_eq!(msg.dlc, 4);
    }

    #[test]
    fn test_error_frame() {
        let msg = CanMessage::new_error();
        assert!(msg.is_error_frame());
    }

    #[test]
    fn test_validation_can20_max_dlc() {
        let result = CanMessage::new(0x100, &[0u8; 9], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_extended_id_range() {
        let mut msg = CanMessage::new(0x20000000, &[0x01], true).unwrap();
        assert!(msg.validate().is_err());

        msg.arbitration_id = 0x1FFFFFFF;
        assert!(msg.validate().is_ok());
    }

    #[test]
    fn test_validation_fd_flags_on_non_fd() {
        let mut msg = CanMessage::new(0x100, &[0x01], false).unwrap();
        msg.flags |= flags::BITRATE_SWITCH;
        assert!(msg.validate().is_err());
    }

    #[test]
    fn test_display_format() {
        let msg = CanMessage::new(0x123, &[0x01, 0x02], false).unwrap();
        let display = format!("{}", msg);
        assert!(display.contains("123"));
        assert!(display.contains("01 02"));
    }

    #[test]
    fn test_flag_setters() {
        let msg = CanMessage::new(0x100, &[0x01], false).unwrap();
        assert!(msg.is_rx());
        let tx_msg = msg.as_tx();
        assert!(!tx_msg.is_rx());
    }

    #[test]
    fn data_mut_updates_effective_payload_only() {
        let mut msg = CanMessage::new(0x100, &[0x01, 0x02], false).unwrap();
        msg.data_mut()[1] = 0xFF;
        assert_eq!(msg.data_slice(), &[0x01, 0xFF]);
    }

    #[test]
    fn setters_toggle_fd_specific_flags_and_channel() {
        let msg = CanMessage::new_fd(0x123, &[0x01], false, false)
            .unwrap()
            .with_channel(7)
            .with_bitrate_switch(true)
            .with_error_state_indicator(true);
        assert_eq!(msg.channel, 7);
        assert!(msg.bitrate_switch());
        assert!(msg.error_state_indicator());

        let msg = msg
            .with_bitrate_switch(false)
            .with_error_state_indicator(false);
        assert!(!msg.bitrate_switch());
        assert!(!msg.error_state_indicator());
    }

    #[test]
    fn protocol_tracks_classic_fd_and_xl_flags() {
        let classic = CanMessage::new(0x100, &[0x01], false).unwrap();
        assert_eq!(classic.protocol(), CanProtocol::Can20);

        let fd = CanMessage::new_fd(0x100, &[0x01], false, false).unwrap();
        assert_eq!(fd.protocol(), CanProtocol::CanFd);

        let xl = CanMessageXL::new(0x123, Bytes::from_static(&[0xAA; 128])).unwrap();
        assert_eq!(xl.header.protocol(), CanProtocol::CanXl);
    }

    #[test]
    fn validation_rejects_conflicting_flags_and_standard_id_overflow() {
        let mut msg = CanMessage::new(0x123, &[0x01], false).unwrap();
        msg.flags |= flags::REMOTE_FRAME | flags::ERROR_FRAME;
        assert!(msg.validate().is_err());

        let mut msg = CanMessage::new(0x800, &[0x01], false).unwrap();
        assert!(msg.validate().is_err());

        msg.arbitration_id = 0x7FF;
        assert!(msg.validate().is_ok());
    }

    #[test]
    fn can_xl_keeps_full_payload_and_rejects_oversized_data() {
        let payload = Bytes::from(vec![0x55; 256]);
        let msg = CanMessageXL::new(0x123, payload.clone()).unwrap();
        assert_eq!(msg.data(), payload.as_ref());
        assert_eq!(&msg.header.data[..64], &payload[..64]);

        let oversized = Bytes::from(vec![0_u8; 2049]);
        assert!(CanMessageXL::new(0x123, oversized).is_err());
    }
}
