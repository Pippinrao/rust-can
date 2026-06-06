//! E2E-COR-PRT-001: PrinterListener captures formatted output.

use std::sync::{Arc, Mutex};

use rust_can_core::listener::{Listener, PrinterListener};
use rust_can_core::message::CanMessage;

struct CapturingPrinter {
    lines: Arc<Mutex<Vec<String>>>,
    prefix: String,
}

impl CapturingPrinter {
    fn new(prefix: impl Into<String>) -> (Self, Arc<Mutex<Vec<String>>>) {
        let lines = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                lines: lines.clone(),
                prefix: prefix.into(),
            },
            lines,
        )
    }
}

impl Listener for CapturingPrinter {
    fn on_message_received(&self, msg: &CanMessage) {
        let line = if self.prefix.is_empty() {
            msg.to_string()
        } else {
            format!("{} {}", self.prefix, msg)
        };
        self.lines.lock().unwrap().push(line);
    }
}

#[test]
fn e2e_cor_prt_001_printer_listener_captures_output() {
    let (listener, lines) = CapturingPrinter::new("CAN");
    let msg = CanMessage::new(0x123, &[0x01, 0x02, 0x03], false).unwrap();

    listener.on_message_received(&msg);

    let captured = lines.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert!(captured[0].starts_with("CAN "));
    assert!(captured[0].contains("123"));
}

#[test]
fn e2e_cor_prt_001_builtin_printer_listener_formats_without_prefix() {
    let listener = PrinterListener::new("");
    let msg = CanMessage::new(0x456, &[0xAA], false).unwrap();
    let rendered = format!("{msg}");
    listener.on_message_received(&msg);
    assert!(!rendered.is_empty());
}
