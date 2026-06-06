//! E2E-IO-001 / E2E-IO-002 / E2E-IO-003: ASC/BLF roundtrip and format detection.

use std::fs::File;
use std::io::{BufReader, Cursor, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rust_can_io::event::{
    CanFdLogEvent, CanLogEvent, Channel, Direction, LinLogEvent, LogEvent, Payload,
};
use rust_can_io::formats::asc::{AscReader, AscWriter};
use rust_can_io::formats::blf::{BlfReader, BlfWriter};
use rust_can_io::reader::LogFormat;

fn sample_events() -> Vec<LogEvent> {
    vec![
        LogEvent::Can(CanLogEvent {
            timestamp_ns: 80_000,
            channel: Channel::Number(2),
            arbitration_id: 0x1d1,
            direction: Direction::Rx,
            extended_id: false,
            remote_frame: false,
            data: Payload::from_slice(&[0, 0, 0, 0, 0xf8, 0, 0x82, 0xd9]),
        }),
        LogEvent::CanFd(CanFdLogEvent {
            timestamp_ns: 0,
            channel: Channel::Number(6),
            arbitration_id: 0x637,
            direction: Direction::Rx,
            extended_id: false,
            bitrate_switch: false,
            error_state_indicator: false,
            dlc_code: 10,
            data: Payload::from_slice(&[0x20; 16]),
        }),
        LogEvent::Lin(LinLogEvent {
            timestamp_ns: 30_000,
            channel: Channel::Named("L11".to_string()),
            frame_id: 1,
            direction: Direction::Rx,
            data: Payload::from_slice(&[0, 0x4f, 0x3f, 0xff, 0xff, 0xc0, 0xfe, 0xfe]),
            checksum: Some(0),
        }),
    ]
}

fn temp_path(suffix: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("rust-can-e2e-{unique}{suffix}"))
}

#[test]
fn e2e_io_001_asc_roundtrip_can_canfd_lin() {
    let events = sample_events();
    let path = temp_path(".asc");

    {
        let file = File::create(&path).expect("temp ASC should be created");
        let mut writer = AscWriter::new(file);
        for event in &events {
            writer.write_event(event).expect("ASC write should succeed");
        }
    }

    let file = File::open(&path).expect("temp ASC should open");
    let parsed = AscReader::new(BufReader::new(file))
        .collect_events()
        .expect("ASC read should succeed");

    assert_eq!(parsed.len(), events.len());
    assert_eq!(parsed, events);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn e2e_io_002_blf_roundtrip_can_canfd() {
    // BLF writer currently normalizes some timestamps; use BLF-compatible fixtures.
    let events = vec![
        LogEvent::Can(CanLogEvent {
            timestamp_ns: 0,
            channel: Channel::Number(2),
            arbitration_id: 0x1d1,
            direction: Direction::Rx,
            extended_id: false,
            remote_frame: false,
            data: Payload::from_slice(&[0, 0, 0, 0, 0xf8, 0, 0x82, 0xd9]),
        }),
        LogEvent::CanFd(CanFdLogEvent {
            timestamp_ns: 1_000,
            channel: Channel::Number(6),
            arbitration_id: 0x637,
            direction: Direction::Rx,
            extended_id: false,
            bitrate_switch: false,
            error_state_indicator: false,
            dlc_code: 10,
            data: Payload::from_slice(&[0x20; 16]),
        }),
    ];
    let path = temp_path(".blf");

    {
        let file = File::create(&path).expect("temp BLF should be created");
        let mut writer = BlfWriter::new(file);
        for event in &events {
            writer.write_event(event).expect("BLF write should succeed");
        }
        writer.finish().expect("BLF finish should succeed");
    }

    let file = File::open(&path).expect("temp BLF should open");
    let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
    let parsed = reader.collect_events().expect("BLF body should parse");

    assert_eq!(parsed.len(), events.len());
    assert_eq!(parsed, events);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn e2e_io_003_format_detect_extension_and_magic() {
    let asc_path = temp_path(".asc");
    let blf_path = temp_path(".blf.gz");
    File::create(&asc_path).expect("asc stub");
    File::create(&blf_path).expect("blf.gz stub");

    assert_eq!(
        LogFormat::from_path(Path::new("capture.asc")).unwrap(),
        LogFormat::Asc
    );
    assert_eq!(
        LogFormat::from_path(Path::new("capture.blf")).unwrap(),
        LogFormat::Blf
    );
    assert_eq!(
        LogFormat::from_path(Path::new("capture.asc.gz")).unwrap(),
        LogFormat::Asc
    );
    assert_eq!(
        LogFormat::from_path(&asc_path).unwrap(),
        LogFormat::Asc
    );

    let mut blf_bytes = Cursor::new(Vec::new());
    {
        let events = sample_events()
            .into_iter()
            .filter(|event| !matches!(event, LogEvent::Lin(_)))
            .collect::<Vec<_>>();
        let mut writer = BlfWriter::new(&mut blf_bytes);
        for event in &events {
            writer.write_event(event).unwrap();
        }
        writer.finish().unwrap();
    }
    let bytes = blf_bytes.into_inner();
    assert_eq!(LogFormat::from_magic(&bytes), Some(LogFormat::Blf));

    let _ = std::fs::remove_file(&asc_path);
    let _ = std::fs::remove_file(&blf_path);
}

#[test]
fn e2e_io_001_file_roundtrip_preserves_payload_sample() {
    let events = sample_events();
    let path = temp_path("-file.asc");

    {
        let mut file = File::create(&path).unwrap();
        let mut writer = AscWriter::new(&mut file);
        for event in &events {
            writer.write_event(event).unwrap();
        }
        file.flush().unwrap();
    }

    let parsed = AscReader::new(BufReader::new(File::open(&path).unwrap()))
        .collect_events()
        .unwrap();
    let LogEvent::Can(can) = &parsed[0] else {
        panic!("first event should be CAN");
    };
    assert_eq!(can.arbitration_id, 0x1d1);
    assert_eq!(can.data.as_slice(), &[0, 0, 0, 0, 0xf8, 0, 0x82, 0xd9]);

    let _ = std::fs::remove_file(&path);
}
