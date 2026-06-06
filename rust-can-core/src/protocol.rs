/// CAN protocol types supported by rust-can.
///
/// Maps to python-can's `CanProtocol` enum with extensions for CAN XL.
use serde::{Deserialize, Serialize};

/// The CAN protocol type supported by a bus instance.
///
/// Each variant represents a different CAN protocol level.
/// The protocol is set at bus initialization time and does not
/// change during the lifetime of a bus instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CanProtocol {
    /// Classical CAN 2.0 (ISO 11898-1:2003)
    /// - 11-bit (standard) or 29-bit (extended) identifiers
    /// - Up to 8 bytes data payload
    /// - Max bitrate: 1 Mbit/s
    Can20 = 0,

    /// CAN FD - Flexible Data-rate, ISO mode (ISO 11898-1:2015)
    /// - Up to 64 bytes data payload
    /// - Dual bitrate: arbitration phase up to 1 Mbit/s, data phase up to 8 Mbit/s
    /// - ISO standardized CRC
    CanFd = 1,

    /// CAN FD - Flexible Data-rate, Non-ISO mode (Bosch original spec)
    /// - Same as CanFd but with Bosch's original CRC scheme
    /// - Legacy mode for older Bosch controllers
    CanFdNonIso = 2,

    /// CAN XL - Extra Large (ISO 11898-1:2024 / CiA 610-1)
    /// - 11-bit priority ID + 32-bit acceptance ID
    /// - Up to 2048 bytes data payload
    /// - Dual bitrate: arbitration up to 1 Mbit/s, data up to 20 Mbit/s
    /// - New CRC scheme and SDT/SVC field
    CanXl = 3,
}

impl CanProtocol {
    /// Returns the maximum data payload size in bytes for this protocol.
    pub const fn max_data_length(&self) -> usize {
        match self {
            CanProtocol::Can20 => 8,
            CanProtocol::CanFd | CanProtocol::CanFdNonIso => 64,
            CanProtocol::CanXl => 2048,
        }
    }

    /// Returns whether this protocol supports CAN FD or higher.
    pub const fn is_fd_or_higher(&self) -> bool {
        matches!(self, CanProtocol::CanFd | CanProtocol::CanFdNonIso | CanProtocol::CanXl)
    }

    /// Returns whether this protocol is CAN XL.
    pub const fn is_xl(&self) -> bool {
        matches!(self, CanProtocol::CanXl)
    }
}

/// The operational state of a CAN bus controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BusState {
    /// Bus is fully operational and participating in CAN communication.
    Active = 0,

    /// Bus is in listen-only / passive mode (no ACKs, no error frames).
    Passive = 1,

    /// Bus controller is in error state (bus-off or warning).
    Error = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_capabilities_match_payload_limits() {
        assert_eq!(CanProtocol::Can20.max_data_length(), 8);
        assert_eq!(CanProtocol::CanFd.max_data_length(), 64);
        assert_eq!(CanProtocol::CanFdNonIso.max_data_length(), 64);
        assert_eq!(CanProtocol::CanXl.max_data_length(), 2048);
        assert!(!CanProtocol::Can20.is_fd_or_higher());
        assert!(CanProtocol::CanFd.is_fd_or_higher());
        assert!(CanProtocol::CanXl.is_xl());
    }
}
