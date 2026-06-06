/// Error types for rust-can.
use std::io;

/// Base error type for all CAN-related errors.
#[derive(Debug, thiserror::Error)]
pub enum CanError {
    /// The requested interface has no implementation.
    #[error("{message}")]
    InterfaceNotImplemented {
        /// Human-readable error message.
        message: String,
    },

    /// Bus or adapter initialization failed.
    #[error("{message}")]
    InitializationError {
        /// Human-readable error message.
        message: String,
    },

    /// A bus operation failed.
    #[error("{message}")]
    OperationError {
        /// Human-readable error message.
        message: String,
    },

    /// A bus operation timed out.
    #[error("{message}")]
    TimeoutError {
        /// Human-readable error message.
        message: String,
    },

    /// A feature is not supported by the selected bus or adapter.
    #[error("{feature} is not supported: {reason}")]
    NotSupported {
        /// Unsupported feature name.
        feature: String,
        /// Explanation of why the feature is unavailable.
        reason: String,
    },

    /// Wrapped input/output error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Internal invariant violation.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl CanError {
    /// Creates an interface-not-implemented error.
    pub fn interface_not_implemented(message: impl Into<String>) -> Self {
        CanError::InterfaceNotImplemented { message: message.into() }
    }

    /// Creates an initialization error.
    pub fn initialization(message: impl Into<String>) -> Self {
        CanError::InitializationError { message: message.into() }
    }

    /// Creates an operation error.
    pub fn operation(message: impl Into<String>) -> Self {
        CanError::OperationError { message: message.into() }
    }

    /// Creates a timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        CanError::TimeoutError { message: message.into() }
    }

    /// Creates a not-supported error.
    pub fn not_supported(feature: impl Into<String>, reason: impl Into<String>) -> Self {
        CanError::NotSupported { feature: feature.into(), reason: reason.into() }
    }
}

/// Convenient result alias for rust-can operations.
pub type Result<T> = std::result::Result<T, CanError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_format_user_visible_messages() {
        assert_eq!(
            CanError::interface_not_implemented("missing").to_string(),
            "missing"
        );
        assert_eq!(CanError::initialization("bad init").to_string(), "bad init");
        assert_eq!(CanError::operation("bad op").to_string(), "bad op");
        assert_eq!(CanError::timeout("too slow").to_string(), "too slow");
        assert_eq!(
            CanError::not_supported("fileno", "virtual").to_string(),
            "fileno is not supported: virtual"
        );
        assert!(CanError::Internal("state".to_string()).to_string().contains("state"));
    }

    #[test]
    fn io_errors_are_wrapped() {
        let error: CanError = io::Error::other("disk").into();
        assert!(error.to_string().contains("disk"));
    }
}
