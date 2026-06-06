//! Shared benchmark helpers for rust-can.

/// A measured throughput result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Throughput {
    /// Number of processed items.
    pub items: u64,
    /// Elapsed time in nanoseconds.
    pub elapsed_ns: u128,
}

impl Throughput {
    /// Creates a throughput measurement.
    pub const fn new(items: u64, elapsed_ns: u128) -> Self {
        Self { items, elapsed_ns }
    }

    /// Returns processed items per second.
    pub fn items_per_second(&self) -> f64 {
        if self.elapsed_ns == 0 {
            0.0
        } else {
            self.items as f64 / (self.elapsed_ns as f64 / 1_000_000_000.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Throughput;

    #[test]
    fn computes_items_per_second() {
        let throughput = Throughput::new(100, 1_000_000_000);
        assert_eq!(throughput.items_per_second(), 100.0);
    }
}
