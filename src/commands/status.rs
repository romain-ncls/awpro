//! `awpro status` — every readable setting in one snapshot.

use hidapi::HidDevice;
use serde_json::{Value, json};

use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, Anc, AutoOff, PowerSaving, Reply, op};

/// Width of the label column in plain output: the longest label plus a gap.
const LABEL_WIDTH: usize = 18;

/// Queries that may fail before a device with nothing to say is written off.
/// See [`ReadState::query`].
const SILENT_QUERY_LIMIT: usize = 2;

pub fn run(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let status = read(device_handle)?;
    output::print(&status.plain(), status.json(), json);
    Ok(())
}

// ── The snapshot ──────────────────────────────────────────────────────────────

/// Everything the headset can be asked for, read in one go.
///
/// Every field is optional because every field is separately fallible: the
/// opcode table is reverse-engineered, so one query falling silent must not
/// take the other six down with it. `errors` names each field that could not
/// be read and the [`AppError::kind`] tag saying why.
#[derive(Debug, Default)]
struct Status {
    battery: Option<u8>,
    anc: Option<Anc>,
    mic_muted: Option<bool>,
    noise_cancel: Option<bool>,
    sidetone: Option<u8>,
    power_saving: Option<PowerSaving>,
    auto_off: Option<AutoOff>,
    errors: Vec<(&'static str, &'static str)>,
}

impl Status {
    /// One `(label, value)` pair per field, in display order.
    ///
    /// Labels are spelled like the `get` subcommand that reads the same field,
    /// so the table doubles as an index of what can be queried on its own.
    fn rows(&self) -> Vec<(&'static str, String)> {
        vec![
            self.row("battery", "battery", self.battery.map(|l| format!("{l}%"))),
            self.row("anc", "anc", self.anc.as_ref().map(output::anc_plain)),
            self.row(
                "mic",
                "mic.muted",
                self.mic_muted
                    .map(|muted| if muted { "muted" } else { "unmuted" }.to_string()),
            ),
            self.row(
                "mic noise-cancel",
                "mic.noise_cancel",
                self.noise_cancel.map(on_off),
            ),
            self.row("sidetone", "sidetone", self.sidetone.map(sidetone_plain)),
            self.row(
                "power saving",
                "power.saving",
                self.power_saving.as_ref().map(saving_plain),
            ),
            self.row(
                "power auto-off",
                "power.auto_off",
                self.auto_off.as_ref().map(auto_off_plain),
            ),
        ]
    }

    /// Pair `label` with `value`, or — when the field is missing — with the
    /// reason the query keyed `field` gave for not producing it.
    fn row(
        &self,
        label: &'static str,
        field: &str,
        value: Option<String>,
    ) -> (&'static str, String) {
        let text = value.unwrap_or_else(|| match self.error_for(field) {
            Some(kind) => format!("unavailable ({kind})"),
            None => "unavailable".to_string(),
        });
        (label, text)
    }

    fn error_for(&self, field: &str) -> Option<&'static str> {
        self.errors
            .iter()
            .find(|(name, _)| *name == field)
            .map(|(_, kind)| *kind)
    }

    /// The aligned table printed without `--json`.
    fn plain(&self) -> String {
        self.rows()
            .iter()
            .map(|(label, value)| format!("{label:<LABEL_WIDTH$}{value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The object printed under `--json`.
    ///
    /// Each field carries the same shape its `get` subcommand emits, so one
    /// decoder serves both. A field that could not be read is `null` and is
    /// named under `errors` alongside its error kind.
    fn json(&self) -> Value {
        let mut value = json!({
            "battery": self.battery.map(|level| json!({ "level": level })),
            "anc": self.anc.as_ref().map(output::anc_json),
            "mic": {
                "muted": self.mic_muted,
                "noise_cancel": self.noise_cancel,
            },
            "sidetone": self.sidetone,
            "power": {
                "saving": self.power_saving.as_ref().map(saving_json),
                "auto_off": self.auto_off.as_ref().map(auto_off_json),
            },
        });

        if !self.errors.is_empty() {
            value["errors"] = self
                .errors
                .iter()
                .map(|(field, kind)| ((*field).to_string(), Value::from(*kind)))
                .collect::<serde_json::Map<_, _>>()
                .into();
        }
        value
    }
}

// ── Reading ───────────────────────────────────────────────────────────────────

/// Query every readable field, tolerating one that will not answer.
///
/// A failed query is recorded against the fields it would have filled and the
/// rest carry on. If nothing at all answered the first error is returned
/// instead, so a sleeping headset still exits with its usual code rather than
/// printing seven `unavailable` rows and calling that success.
fn read(device_handle: &HidDevice) -> Result<Status, AppError> {
    let mut state = ReadState::default();
    let mut status = Status::default();

    if let Some(reply) = state.query(&["battery"], || device::query(device_handle, op::BATTERY)) {
        status.battery = Some(protocol::parse_battery(&reply));
    }
    if let Some(reply) = state.query(&["anc"], || device::query(device_handle, op::ANC_GET)) {
        status.anc = Some(protocol::parse_anc(&reply));
    }
    if let Some(reply) = state.query(&["mic.muted"], || {
        device::query(device_handle, op::MIC_MUTE_GET)
    }) {
        status.mic_muted = Some(protocol::parse_mic_muted(&reply));
    }
    // One reply feeds two fields: the sidetone level is the last byte of the
    // noise-cancel payload, there being no sidetone getter. See
    // `protocol::parse_sidetone`.
    if let Some(reply) = state.query(&["mic.noise_cancel", "sidetone"], || {
        device::query(device_handle, op::MIC_NOISE_CANCEL)
    }) {
        status.noise_cancel = Some(protocol::parse_mic_noise_cancel(&reply));
        status.sidetone = Some(protocol::parse_sidetone(&reply));
    }
    if let Some(reply) = state.query(&["power.saving"], || {
        device::query(device_handle, op::POWER_SAVING)
    }) {
        status.power_saving = Some(protocol::parse_power_saving(&reply));
    }
    if let Some(reply) = state.query(&["power.auto_off"], || {
        device::query(device_handle, op::AUTO_OFF)
    }) {
        status.auto_off = Some(protocol::parse_auto_off(&reply));
    }

    status.errors = std::mem::take(&mut state.errors);
    match state.first_error {
        Some(err) if state.answered == 0 => Err(err),
        _ => Ok(status),
    }
}

/// Bookkeeping shared by the queries [`read`] issues.
#[derive(Default)]
struct ReadState {
    errors: Vec<(&'static str, &'static str)>,
    first_error: Option<AppError>,
    answered: usize,
    failed: usize,
}

impl ReadState {
    /// Run one query, attributing any failure to every field it would fill.
    ///
    /// Once [`SILENT_QUERY_LIMIT`] queries have failed with nothing having
    /// answered, the remaining ones are skipped without touching the device: a
    /// headset that is asleep or out of range fails all of them, each failure
    /// costs a full poll window, and [`read`] is going to return the error
    /// rather than the snapshot anyway. A single answer clears that: from then
    /// on every field is worth asking for, however many others stay quiet.
    fn query<F>(&mut self, fields: &[&'static str], fetch: F) -> Option<Reply>
    where
        F: FnOnce() -> Result<Reply, AppError>,
    {
        if self.answered == 0 && self.failed >= SILENT_QUERY_LIMIT {
            return None;
        }
        match fetch() {
            Ok(reply) => {
                self.answered += 1;
                Some(reply)
            }
            Err(err) => {
                self.failed += 1;
                for field in fields {
                    self.errors.push((field, err.kind()));
                }
                if self.first_error.is_none() {
                    self.first_error = Some(err);
                }
                None
            }
        }
    }
}

// ── Per-field rendering ───────────────────────────────────────────────────────

fn on_off(enabled: bool) -> String {
    if enabled { "on" } else { "off" }.to_string()
}

fn sidetone_plain(level: u8) -> String {
    match level {
        0 => "off".to_string(),
        n => n.to_string(),
    }
}

fn saving_plain(saving: &PowerSaving) -> String {
    if saving.enabled {
        format!("on (threshold: {}%)", saving.threshold)
    } else {
        "off".to_string()
    }
}

fn saving_json(saving: &PowerSaving) -> Value {
    if saving.enabled {
        json!({ "enabled": true, "threshold": saving.threshold })
    } else {
        json!({ "enabled": false })
    }
}

fn auto_off_plain(auto_off: &AutoOff) -> String {
    if auto_off.enabled {
        format!("{} minutes", auto_off.minutes)
    } else {
        "off".to_string()
    }
}

fn auto_off_json(auto_off: &AutoOff) -> Value {
    if auto_off.enabled {
        json!({ "enabled": true, "minutes": auto_off.minutes })
    } else {
        json!({ "enabled": false })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::REPLY_LEN;

    /// A snapshot in which every query answered.
    fn full() -> Status {
        Status {
            battery: Some(87),
            anc: Some(Anc::Transparency(3)),
            mic_muted: Some(false),
            noise_cancel: Some(true),
            sidetone: Some(4),
            power_saving: Some(PowerSaving {
                enabled: true,
                threshold: 20,
            }),
            auto_off: Some(AutoOff {
                enabled: true,
                minutes: 30,
            }),
            errors: Vec::new(),
        }
    }

    #[test]
    fn rows_describe_every_field_the_device_can_be_asked_for() {
        assert_eq!(
            full().rows(),
            vec![
                ("battery", "87%".to_string()),
                ("anc", "transparency (level 3)".to_string()),
                ("mic", "unmuted".to_string()),
                ("mic noise-cancel", "on".to_string()),
                ("sidetone", "4".to_string()),
                ("power saving", "on (threshold: 20%)".to_string()),
                ("power auto-off", "30 minutes".to_string()),
            ]
        );
    }

    #[test]
    fn rows_name_the_failure_for_a_field_that_could_not_be_read() {
        let status = Status {
            anc: None,
            errors: vec![("anc", "timeout")],
            ..full()
        };

        assert_eq!(
            status.rows()[1],
            ("anc", "unavailable (timeout)".to_string())
        );
    }

    #[test]
    fn plain_starts_every_value_in_the_same_column() {
        let status = full();
        let text = status.plain();

        let starts: Vec<usize> = text
            .lines()
            .zip(status.rows())
            .map(|(line, (_, value))| {
                assert!(
                    line.ends_with(&value),
                    "line {line:?} does not end in its value"
                );
                line.len() - value.len()
            })
            .collect();

        assert_eq!(starts.len(), status.rows().len());
        assert!(
            starts.windows(2).all(|w| w[0] == w[1]),
            "values are not aligned: {starts:?}"
        );
    }

    #[test]
    fn json_mirrors_the_objects_the_get_subcommands_emit() {
        assert_eq!(
            full().json(),
            json!({
                "battery": { "level": 87 },
                "anc": { "mode": "transparency", "level": 3 },
                "mic": { "muted": false, "noise_cancel": true },
                "sidetone": 4,
                "power": {
                    "saving": { "enabled": true, "threshold": 20 },
                    "auto_off": { "enabled": true, "minutes": 30 }
                }
            })
        );
    }

    #[test]
    fn json_nulls_an_unreadable_field_and_names_it_under_errors() {
        let status = Status {
            anc: None,
            sidetone: None,
            noise_cancel: None,
            errors: vec![
                ("anc", "timeout"),
                ("mic.noise_cancel", "timeout"),
                ("sidetone", "timeout"),
            ],
            ..full()
        };
        let value = status.json();

        assert_eq!(value["anc"], Value::Null);
        assert_eq!(value["sidetone"], Value::Null);
        assert_eq!(value["mic"]["noise_cancel"], Value::Null);
        assert_eq!(value["mic"]["muted"], Value::Bool(false));
        assert_eq!(
            value["errors"],
            json!({ "anc": "timeout", "mic.noise_cancel": "timeout", "sidetone": "timeout" })
        );
    }

    #[test]
    fn a_device_that_answers_nothing_stops_being_asked() {
        let mut state = ReadState::default();
        let mut attempts = 0;

        for _ in 0..6 {
            state.query(&["field"], || {
                attempts += 1;
                Err(AppError::Timeout)
            });
        }

        assert_eq!(attempts, SILENT_QUERY_LIMIT);
    }

    #[test]
    fn one_answer_keeps_every_later_field_worth_asking_for() {
        let mut state = ReadState::default();
        let mut attempts = 0;

        // Fail, answer, then fail for the rest: the single answer proves the
        // device is awake, so no later query may be skipped.
        for i in 0..6 {
            state.query(&["field"], || {
                attempts += 1;
                if i == 1 {
                    Ok([0u8; REPLY_LEN])
                } else {
                    Err(AppError::Timeout)
                }
            });
        }

        assert_eq!(attempts, 6);
    }
}
