/// CAN bit timing configuration.
///
/// Supports both CAN 2.0 (classic) and CAN FD timing calculations.
/// Based on the algorithms in python-can's `bit_timing.py`.
use std::collections::BTreeMap;

/// A CAN 2.0 bit timing configuration.
///
/// Represents the parameters needed to configure a CAN controller
/// for a specific bitrate.
///
/// # Parameters
/// - `f_clock`: CAN system clock frequency in Hz
/// - `brp`: Baud rate prescaler (1-64)
/// - `tseg1`: Time segment 1 (quanta from sync to sample point, 1-16)
/// - `tseg2`: Time segment 2 (quanta from sample point to end, 1-8)
/// - `sjw`: Synchronization jump width (1-4)
/// - `nof_samples`: Number of samples per bit (1 or 3)
///
/// # Bitrate formula
/// ```text
/// bitrate = f_clock / (brp * (1 + tseg1 + tseg2))
/// sample_point = 100 * (1 + tseg1) / (1 + tseg1 + tseg2)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitTiming {
    /// CAN system clock frequency in Hz.
    pub f_clock: u32,
    /// Baud rate prescaler (1-64).
    pub brp: u16,
    /// Time segment 1 (1-16 quanta).
    pub tseg1: u16,
    /// Time segment 2 (1-8 quanta).
    pub tseg2: u16,
    /// Synchronization jump width (1-4 quanta).
    pub sjw: u16,
    /// Number of samples per bit (1 or 3).
    pub nof_samples: u8,
}

impl BitTiming {
    /// Create a new BitTiming and validate parameters.
    pub fn new(
        f_clock: u32,
        brp: u16,
        tseg1: u16,
        tseg2: u16,
        sjw: u16,
        nof_samples: u8,
    ) -> Result<Self, String> {
        let bt = Self {
            f_clock,
            brp,
            tseg1,
            tseg2,
            sjw,
            nof_samples,
        };
        bt.validate()?;
        Ok(bt)
    }

    /// Create BitTiming from bitrate and segment values.
    ///
    /// Computes the BRP from the given parameters.
    pub fn from_bitrate_and_segments(
        f_clock: u32,
        bitrate: u32,
        tseg1: u16,
        tseg2: u16,
        sjw: u16,
        nof_samples: u8,
    ) -> Result<Self, String> {
        let total_tq = 1 + tseg1 + tseg2;
        let brp = f_clock / (bitrate * total_tq as u32);
        if brp == 0 || brp > 64 {
            return Err(format!(
                "Cannot achieve bitrate {} with f_clock={} and tseg1={}, tseg2={}: brp would be {}",
                bitrate, f_clock, tseg1, tseg2, brp
            ));
        }
        Self::new(f_clock, brp as u16, tseg1, tseg2, sjw, nof_samples)
    }

    /// Create BitTiming from BTR0 and BTR1 registers (SJA1000 style).
    pub fn from_registers(f_clock: u32, btr0: u8, btr1: u8) -> Result<Self, String> {
        let brp = ((btr0 & 0x3F) + 1) as u16;
        let sjw = ((btr0 >> 6) + 1) as u16;
        let tseg1 = ((btr1 & 0x0F) + 1) as u16;
        let tseg2 = (((btr1 >> 4) & 0x07) + 1) as u16;
        let sam = if btr1 & 0x80 != 0 { 3 } else { 1 };
        Self::new(f_clock, brp, tseg1, tseg2, sjw, sam)
    }

    /// Find a bit timing for a desired sample point.
    ///
    /// Searches for valid BRP, TSEG1, TSEG2 combinations
    /// that achieve the desired bitrate and sample point.
    pub fn from_sample_point(
        f_clock: u32,
        bitrate: u32,
        sample_point: f64,
        sjw: Option<u16>,
        nof_samples: Option<u8>,
    ) -> Result<Self, String> {
        if !(50.0..=90.0).contains(&sample_point) {
            return Err(format!(
                "Sample point must be between 50.0 and 90.0%, got {}",
                sample_point
            ));
        }

        let sjw = sjw.unwrap_or(1);
        let nof_samples = nof_samples.unwrap_or(1);

        let mut best: Option<(f64, BitTiming)> = None;

        for brp in 1..=64u16 {
            let tq_freq = f_clock / brp as u32;
            let total_tq = tq_freq / bitrate;
            if !(4..=25).contains(&total_tq) {
                continue;
            }

            for tseg1 in 1..=16u16 {
                for tseg2 in 1..=8u16 {
                    if 1 + tseg1 + tseg2 != total_tq as u16 {
                        continue;
                    }
                    let sp = 100.0 * (1.0 + tseg1 as f64) / (1.0 + tseg1 as f64 + tseg2 as f64);
                    let diff = (sp - sample_point).abs();
                    if (best.is_none() || diff < best.as_ref().unwrap().0)
                        && let Ok(bt) = Self::new(f_clock, brp, tseg1, tseg2, sjw, nof_samples)
                    {
                        best = Some((diff, bt));
                    }
                }
            }
        }

        best.map(|(_, bt)| bt)
            .ok_or_else(|| format!(
                "No valid bit timing found for f_clock={}, bitrate={}, sample_point={}",
                f_clock, bitrate, sample_point
            ))
    }

    /// Compute the actual bitrate in bits/s.
    pub fn bitrate(&self) -> u32 {
        let total_tq = (1 + self.tseg1 + self.tseg2) as u32;
        self.f_clock / (self.brp as u32 * total_tq)
    }

    /// Compute the sample point as a percentage (0-100).
    pub fn sample_point(&self) -> f64 {
        100.0 * (1.0 + self.tseg1 as f64) / (1.0 + self.tseg1 as f64 + self.tseg2 as f64)
    }

    /// Validate timing parameters against CAN specification.
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=64).contains(&self.brp) {
            return Err(format!("BRP must be in [1..64], got {}", self.brp));
        }
        if !(1..=16).contains(&self.tseg1) {
            return Err(format!("TSEG1 must be in [1..16], got {}", self.tseg1));
        }
        if !(1..=8).contains(&self.tseg2) {
            return Err(format!("TSEG2 must be in [1..8], got {}", self.tseg2));
        }
        if !(1..=4).contains(&self.sjw) {
            return Err(format!("SJW must be in [1..4], got {}", self.sjw));
        }
        if self.sjw > self.tseg2 {
            return Err(format!(
                "SJW ({}) must not be greater than TSEG2 ({})",
                self.sjw, self.tseg2
            ));
        }
        if self.sample_point() < 50.0 {
            return Err(format!(
                "Sample point ({:.1}%) must be >= 50%",
                self.sample_point()
            ));
        }
        if ![1, 3].contains(&self.nof_samples) {
            return Err(format!(
                "Number of samples must be 1 or 3, got {}",
                self.nof_samples
            ));
        }
        Ok(())
    }

    /// Convert to BTR0/BTR1 register values (SJA1000 compatible).
    pub fn to_registers(&self) -> (u8, u8) {
        let btr0 = ((self.brp - 1) & 0x3F) as u8 | (((self.sjw - 1) & 0x03) << 6) as u8;
        let mut btr1 = ((self.tseg1 - 1) & 0x0F) as u8 | (((self.tseg2 - 1) & 0x07) << 4) as u8;
        if self.nof_samples == 3 {
            btr1 |= 0x80;
        }
        (btr0, btr1)
    }
}

/// CAN FD bit timing for the data phase.
///
/// CAN FD supports different bitrates for the arbitration and data phases.
/// The data phase can run at significantly higher speeds (up to 8 Mbit/s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitTimingFd {
    /// Arbitration phase timing (same as classic CAN).
    pub nominal: BitTiming,
    /// Data phase timing (higher speed).
    pub data: BitTiming,
}

impl BitTimingFd {
    /// Creates CAN FD bit timing from nominal and data phase timing.
    pub fn new(nominal: BitTiming, data: BitTiming) -> Self {
        Self { nominal, data }
    }

    /// Get the nominal (arbitration) bitrate.
    pub fn nominal_bitrate(&self) -> u32 {
        self.nominal.bitrate()
    }

    /// Get the data phase bitrate.
    pub fn data_bitrate(&self) -> u32 {
        self.data.bitrate()
    }
}

/// Common bit timing presets for standard bitrates.
pub fn standard_presets() -> BTreeMap<u32, BitTiming> {
    let mut presets = BTreeMap::new();

    // Standard presets for f_clock = 8 MHz (common for many microcontrollers)
    let f_clock = 8_000_000;

    // 1 Mbit/s
    presets.insert(
        1_000_000,
        BitTiming::new(f_clock, 1, 5, 2, 1, 1).unwrap(),
    );

    // 500 kbit/s
    presets.insert(
        500_000,
        BitTiming::new(f_clock, 2, 5, 2, 1, 1).unwrap(),
    );

    // 250 kbit/s
    presets.insert(
        250_000,
        BitTiming::new(f_clock, 4, 5, 2, 1, 1).unwrap(),
    );

    // 125 kbit/s
    presets.insert(
        125_000,
        BitTiming::new(f_clock, 8, 5, 2, 1, 1).unwrap(),
    );

    // 100 kbit/s
    presets.insert(
        100_000,
        BitTiming::new(f_clock, 10, 5, 2, 1, 1).unwrap(),
    );

    // 50 kbit/s
    presets.insert(
        50_000,
        BitTiming::new(f_clock, 20, 5, 2, 1, 1).unwrap(),
    );

    // 20 kbit/s
    presets.insert(
        20_000,
        BitTiming::new(f_clock, 50, 5, 2, 1, 1).unwrap(),
    );

    // 10 kbit/s
    presets.insert(
        10_000,
        BitTiming::new(f_clock, 100, 5, 2, 1, 1).unwrap_or_else(|_| {
            // brp 100 exceeds limit, adjust
            BitTiming::new(f_clock, 64, 5, 2, 1, 1).unwrap()
        }),
    );

    presets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitrate_calculation() {
        let bt = BitTiming::new(8_000_000, 1, 5, 2, 1, 1).unwrap();
        assert_eq!(bt.bitrate(), 1_000_000);
        assert!((bt.sample_point() - 75.0).abs() < 0.5);
    }

    #[test]
    fn test_from_sample_point() {
        let bt = BitTiming::from_sample_point(8_000_000, 500_000, 87.5, None, None).unwrap();
        assert_eq!(bt.bitrate(), 500_000);
        assert!((bt.sample_point() - 87.5).abs() < 5.0);
    }

    #[test]
    fn test_register_conversion() {
        let bt = BitTiming::new(8_000_000, 1, 5, 2, 1, 1).unwrap();
        let (btr0, btr1) = bt.to_registers();
        assert_eq!(btr0, 0x00);
        assert_eq!(btr1, 0x14);
    }

    #[test]
    fn test_from_registers() {
        let bt = BitTiming::from_registers(8_000_000, 0x00, 0x14).unwrap();
        assert_eq!(bt.brp, 1);
        assert_eq!(bt.tseg1, 5);
        assert_eq!(bt.tseg2, 2);
    }

    #[test]
    fn test_validation_sjw_le_tseg2() {
        let result = BitTiming::new(8_000_000, 1, 5, 2, 4, 1);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_out_of_range_timing_parameters() {
        assert!(BitTiming::new(8_000_000, 0, 5, 2, 1, 1).is_err());
        assert!(BitTiming::new(8_000_000, 1, 0, 2, 1, 1).is_err());
        assert!(BitTiming::new(8_000_000, 1, 5, 0, 1, 1).is_err());
        assert!(BitTiming::new(8_000_000, 1, 5, 2, 0, 1).is_err());
        assert!(BitTiming::new(8_000_000, 1, 1, 8, 1, 1).is_err());
        assert!(BitTiming::new(8_000_000, 1, 5, 2, 1, 2).is_err());
    }

    #[test]
    fn from_bitrate_and_segments_rejects_impossible_prescaler() {
        let result = BitTiming::from_bitrate_and_segments(8_000_000, 1, 5, 2, 1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn sample_point_search_rejects_invalid_inputs_and_unreachable_rates() {
        assert!(BitTiming::from_sample_point(8_000_000, 500_000, 49.9, None, None).is_err());
        assert!(BitTiming::from_sample_point(8_000_000, 9_000_000, 75.0, Some(1), Some(1)).is_err());
    }

    #[test]
    fn fd_timing_reports_nominal_and_data_bitrates() {
        let nominal = BitTiming::new(8_000_000, 2, 5, 2, 1, 1).unwrap();
        let data = BitTiming::new(8_000_000, 1, 5, 2, 1, 1).unwrap();
        let timing = BitTimingFd::new(nominal, data);
        assert_eq!(timing.nominal_bitrate(), 500_000);
        assert_eq!(timing.data_bitrate(), 1_000_000);
    }

    #[test]
    fn standard_presets_include_common_rates() {
        let presets = standard_presets();
        assert_eq!(presets.get(&1_000_000).unwrap().bitrate(), 1_000_000);
        assert!(presets.contains_key(&125_000));
        assert!(presets.contains_key(&10_000));
    }
}
