/// Adapter registry for plugin-based adapter discovery.
use std::sync::LazyLock;

use parking_lot::RwLock;

/// Information about a CAN adapter.
#[derive(Debug, Clone)]
pub struct AdapterInfo {
    /// Stable adapter name.
    pub name: String,
    /// Human-readable adapter description.
    pub description: String,
    /// Adapter crate version.
    pub version: String,
    /// Whether CAN FD is supported.
    pub supports_fd: bool,
    /// Whether CAN XL is supported.
    pub supports_xl: bool,
    /// Whether hardware filters are supported.
    pub supports_hw_filters: bool,
    /// Supported platform names.
    pub platforms: Vec<String>,
}

impl AdapterInfo {
    /// Creates adapter metadata with default capabilities.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            supports_fd: false,
            supports_xl: false,
            supports_hw_filters: false,
            platforms: Vec::new(),
        }
    }

    /// Sets CAN FD support metadata.
    pub fn with_fd_support(mut self, supports: bool) -> Self {
        self.supports_fd = supports;
        self
    }

    /// Sets CAN XL support metadata.
    pub fn with_xl_support(mut self, supports: bool) -> Self {
        self.supports_xl = supports;
        self
    }

    /// Sets hardware-filter support metadata.
    pub fn with_hw_filters(mut self, supports: bool) -> Self {
        self.supports_hw_filters = supports;
        self
    }
}

/// The global registry of all available adapters.
pub static ADAPTER_REGISTRY: LazyLock<RwLock<Vec<AdapterInfo>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// List all registered adapter names.
pub fn list_adapters() -> Vec<String> {
    ADAPTER_REGISTRY.read().iter().map(|info| info.name.clone()).collect()
}

/// Find an adapter by name.
pub fn find_adapter(name: &str) -> Option<AdapterInfo> {
    ADAPTER_REGISTRY.read().iter().find(|info| info.name == name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_info_builders_set_capabilities() {
        let info = AdapterInfo::new("mock", "Mock adapter")
            .with_fd_support(true)
            .with_xl_support(true)
            .with_hw_filters(true);

        assert_eq!(info.name, "mock");
        assert_eq!(info.description, "Mock adapter");
        assert!(info.supports_fd);
        assert!(info.supports_xl);
        assert!(info.supports_hw_filters);
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn registry_lists_and_finds_inserted_adapter() {
        let name = format!("unit-registry-{}", std::process::id());
        ADAPTER_REGISTRY
            .write()
            .push(AdapterInfo::new(name.clone(), "unit test"));

        assert!(list_adapters().contains(&name));
        assert_eq!(find_adapter(&name).unwrap().description, "unit test");
        assert!(find_adapter("definitely-not-registered").is_none());
    }
}
