//! `awpro watch` — follow state changes the headset pushes on its own.
//!
//! Every other subcommand asks a question and gets one answer. This one listens
//! to report 0x08, which the headset sends unasked whenever something changes —
//! including changes made with its own buttons, which polling can only catch by
//! luck. See `context/PROTOCOL-FINDINGS.md`.
//!
//! Not everything is covered: sidetone, power saving and auto power-off produce
//! no notification at all, confirmed by setting each one while listening. Only
//! the fields in [`protocol::field`] ever arrive.

use hidapi::HidDevice;
use serde_json::{Value, json};

use crate::error::AppError;
use crate::protocol::{self, Notification, field};

/// Width of the label column, matching `status` so the two read alike.
const LABEL_WIDTH: usize = 18;

/// A notification rendered for printing: a label, a plain value, and the JSON
/// object the same change would appear as in `status`.
struct Event {
    label: &'static str,
    plain: String,
    json: Value,
}

/// Describe a notification, or `None` for a field with no known meaning.
///
/// The value byte uses the same encoding as the first payload byte of the
/// matching query, so each arm decodes exactly like its `parse_*` counterpart
/// and the two can never disagree. [`field::ANC`] is the exception worth
/// remembering: it carries only the mode, never the transparency level, so no
/// level is reported here rather than a guessed one.
fn describe(notification: &Notification) -> Option<Event> {
    let value = notification.value;
    Some(match notification.field {
        field::BATTERY => Event {
            label: "battery",
            plain: format!("{value}%"),
            json: json!({ "battery": { "level": value } }),
        },
        field::MIC_MUTED => Event {
            label: "mic",
            // 0 means muted, matching `protocol::parse_mic_muted`.
            plain: if value == 0 { "muted" } else { "unmuted" }.to_string(),
            json: json!({ "mic": { "muted": value == 0 } }),
        },
        field::MIC_NOISE_CANCEL => Event {
            label: "mic noise-cancel",
            plain: if value != 0 { "on" } else { "off" }.to_string(),
            json: json!({ "mic": { "noise_cancel": value != 0 } }),
        },
        field::ANC => {
            let mode = match value {
                0x00 => "off",
                0x01 => "on",
                0x02 => "transparency",
                _ => "unknown",
            };
            Event {
                label: "anc",
                plain: mode.to_string(),
                json: json!({ "anc": { "mode": mode } }),
            }
        }
        field::WIRELESS_LINK => {
            let up = value == 0xCC;
            Event {
                label: "wireless link",
                plain: if up { "up" } else { "down" }.to_string(),
                json: json!({ "wireless_link": { "up": up } }),
            }
        }
        field::TRANSPORT => {
            let dongle = value != 0;
            let name = if dongle { "dongle" } else { "cable" };
            Event {
                label: "transport",
                plain: name.to_string(),
                json: json!({ "transport": name }),
            }
        }
        _ => return None,
    })
}

/// Print every notification until interrupted.
///
/// Unknown fields and malformed reports are skipped rather than reported: the
/// headset shares this endpoint with its consumer-control collection, so a
/// report that is not a notification is normal traffic, not an error.
pub fn run(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let mut buf = [0u8; protocol::REPLY_LEN];
    loop {
        let read = device_handle
            .read(&mut buf)
            .map_err(|e| AppError::HidRead(e.to_string()))?;
        let Some(event) = protocol::parse_notification(&buf[..read])
            .as_ref()
            .and_then(describe)
        else {
            continue;
        };
        if json {
            println!("{}", event.json);
        } else {
            println!("{:<LABEL_WIDTH$}{}", event.label, event.plain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn describe_hex(raw: &str) -> Option<Event> {
        let bytes: Vec<u8> = (0..raw.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).expect("hex digits"))
            .collect();
        describe(&protocol::parse_notification(&bytes).expect("a valid notification"))
    }

    #[test]
    fn describes_every_notification_recorded_from_the_headset() {
        // Verbatim reports from context/awcc-wireshark/notify-*.log.
        for (raw, label, plain) in [
            ("08c009030001cc0f", "wireless link", "up"),
            ("08c009030001dd1e", "wireless link", "down"),
            ("08c00903000264a4", "battery", "100%"),
            ("08c00903000303c2", "transport", "dongle"),
            ("08c00903000300c1", "transport", "cable"),
            ("08c00903000400c6", "mic", "muted"),
            ("08c00903000401c7", "mic", "unmuted"),
            ("08c00903000500c7", "anc", "off"),
            ("08c00903000501c6", "anc", "on"),
            ("08c00903000502c5", "anc", "transparency"),
            ("08c00903000801cb", "mic noise-cancel", "on"),
            ("08c00903000800ca", "mic noise-cancel", "off"),
        ] {
            let event = describe_hex(raw).unwrap_or_else(|| panic!("{raw} was not described"));
            assert_eq!((event.label, event.plain.as_str()), (label, plain), "{raw}");
        }
    }

    #[test]
    fn mic_mute_agrees_with_the_polled_decoder() {
        // A notification and a query must never disagree about what 0 means.
        let mut reply = [0u8; protocol::REPLY_LEN];
        reply[..6].copy_from_slice(&[0x07, 0xC0, 0x76, 0x01, 0x00, 0x00]);
        assert!(protocol::parse_mic_muted(&reply));
        assert_eq!(describe_hex("08c00903000400c6").unwrap().plain, "muted");
    }

    #[test]
    fn json_uses_the_same_keys_as_status() {
        assert_eq!(
            describe_hex("08c00903000264a4").unwrap().json,
            json!({ "battery": { "level": 100 } })
        );
        assert_eq!(
            describe_hex("08c00903000502c5").unwrap().json,
            json!({ "anc": { "mode": "transparency" } })
        );
    }

    #[test]
    fn a_field_with_no_known_meaning_is_skipped() {
        // Fields 0x06 and 0x07 appear in every announce burst with constant
        // values and no identified meaning. Printing a bare number would
        // invite someone to read one into them.
        assert!(
            describe(&Notification {
                field: 0x06,
                value: 0x02
            })
            .is_none()
        );
        assert!(
            describe(&Notification {
                field: 0x07,
                value: 0x01
            })
            .is_none()
        );
    }
}
