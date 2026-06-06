/// Adapter configuration.
///
/// A flexible key-value configuration structure that can represent
/// any adapter's settings without coupling to specific types.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A flexible configuration structure for CAN adapters.
///
/// Uses a flat key-value map for maximum flexibility.
/// Supports JSON, YAML, and TOML deserialization via serde.
///
/// # Standard keys
///
/// | Key | Type | Description |
/// |-----|------|-------------|
/// | `interface` | string | Backend name (e.g., "virtual", "socketcan") |
/// | `channel` | string | Channel identifier (e.g., "can0", "COM3") |
/// | `bitrate` | int | Bitrate in bits/s |
/// | `data_bitrate` | int | CAN FD data phase bitrate |
/// | `f_clock` | int | CAN controller clock frequency in Hz |
/// | `sjw` | int | Synchronization jump width |
/// | `tseg1` | int | Time segment 1 |
/// | `tseg2` | int | Time segment 2 |
/// | `nof_samples` | int | Number of samples (1 or 3) |
/// | `receive_own_messages` | bool | Whether to loop back sent messages |
/// | `is_fd` | bool | Enable CAN FD mode |
/// | `is_xl` | bool | Enable CAN XL mode |
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// The backend interface name.
    pub interface: Option<String>,

    /// The channel / device identifier.
    pub channel: Option<String>,

    /// Key-value pairs for all configuration options.
    #[serde(flatten)]
    pub options: HashMap<String, ConfigValue>,
}

/// A configuration value that can be one of several types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    /// String option.
    String(String),
    /// Integer option.
    Int(i64),
    /// Floating-point option.
    Float(f64),
    /// Boolean option.
    Bool(bool),
}

impl ConfigValue {
    /// Try to get the value as a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }
}

impl AdapterConfig {
    /// Create a new empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration with just an interface name.
    pub fn with_interface(interface: impl Into<String>) -> Self {
        let mut config = Self::new();
        config.interface = Some(interface.into());
        config
    }

    /// Create a configuration with interface and channel.
    pub fn with_interface_and_channel(
        interface: impl Into<String>,
        channel: impl Into<String>,
    ) -> Self {
        let mut config = Self::with_interface(interface);
        config.channel = Some(channel.into());
        config
    }

    /// Set a string option.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.options
            .insert(key.into(), ConfigValue::String(value.into()));
    }

    /// Get a string option.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.options.get(key).and_then(|v| v.as_str())
    }

    /// Set an integer option.
    pub fn set_int(&mut self, key: impl Into<String>, value: i64) {
        self.options.insert(key.into(), ConfigValue::Int(value));
    }

    /// Get an integer option.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.options.get(key).and_then(|v| match v {
            ConfigValue::Int(i) => Some(*i),
            _ => None,
        })
    }

    /// Set a boolean option.
    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.options.insert(key.into(), ConfigValue::Bool(value));
    }

    /// Get a boolean option.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.options.get(key).and_then(|v| match v {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builders_and_typed_accessors_roundtrip_values() {
        let mut config = AdapterConfig::with_interface_and_channel("virtual", "vcan0");
        config.set("mode", "loopback");
        config.set_int("bitrate", 500_000);
        config.set_bool("receive_own_messages", true);
        config
            .options
            .insert("sample_point".to_string(), ConfigValue::Float(87.5));

        assert_eq!(config.interface.as_deref(), Some("virtual"));
        assert_eq!(config.channel.as_deref(), Some("vcan0"));
        assert_eq!(config.get("mode"), Some("loopback"));
        assert_eq!(config.get_int("bitrate"), Some(500_000));
        assert_eq!(config.get_bool("receive_own_messages"), Some(true));
        assert_eq!(config.get("bitrate"), None);
    }

    #[test]
    fn config_deserializes_flat_json_options() {
        let config: AdapterConfig = serde_json::from_str(
            r#"{
                "interface": "virtual",
                "channel": "bench",
                "receive_own_messages": true,
                "rx_queue_size": 8,
                "label": "test"
            }"#,
        )
        .unwrap();

        assert_eq!(config.interface.as_deref(), Some("virtual"));
        assert_eq!(config.channel.as_deref(), Some("bench"));
        assert_eq!(config.get_bool("receive_own_messages"), Some(true));
        assert_eq!(config.get_int("rx_queue_size"), Some(8));
        assert_eq!(config.get("label"), Some("test"));
    }
}
