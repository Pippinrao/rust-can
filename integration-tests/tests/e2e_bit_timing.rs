//! E2E-COR-BTM-001: BitTiming calculation and register roundtrip.

use rust_can_core::bit_timing::BitTiming;

#[test]
fn e2e_cor_btm_001_bit_timing_from_bitrate_and_register_roundtrip() {
    let timing = BitTiming::from_bitrate_and_segments(8_000_000, 500_000, 13, 2, 1, 1)
        .expect("500 kbit/s timing should be valid");

    assert_eq!(timing.bitrate(), 500_000);
    assert!(timing.sample_point() >= 50.0);

    let (btr0, btr1) = timing.to_registers();
    let roundtrip = BitTiming::from_registers(timing.f_clock, btr0, btr1)
        .expect("register roundtrip should succeed");

    assert_eq!(roundtrip.brp, timing.brp);
    assert_eq!(roundtrip.tseg1, timing.tseg1);
    assert_eq!(roundtrip.tseg2, timing.tseg2);
    assert_eq!(roundtrip.sjw, timing.sjw);
    assert_eq!(roundtrip.nof_samples, timing.nof_samples);
    assert_eq!(roundtrip.bitrate(), timing.bitrate());
}

#[test]
fn e2e_cor_btm_001_sample_point_search_matches_target() {
    let timing = BitTiming::from_sample_point(8_000_000, 1_000_000, 75.0, Some(1), Some(1))
        .expect("1 Mbit/s sample point search should succeed");

    assert_eq!(timing.bitrate(), 1_000_000);
    assert!((timing.sample_point() - 75.0).abs() < 5.0);
}
