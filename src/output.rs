use serde_json::{Value, json};

use crate::error::AppError;
use crate::protocol::{Anc, WirelessLink};

/// Print either a plain string or a JSON object depending on the `json` flag.
pub fn print(plain: &str, json_value: Value, json: bool) {
    if json {
        println!("{json_value}");
    } else {
        println!("{plain}");
    }
}

/// Report the outcome of a command that sets something.
///
/// Plain mode stays silent on success — a command that worked printing nothing
/// is the useful shell behaviour, and it is what this tool has always done.
/// `--json` gets one object describing the new state, so the globally-accepted
/// flag is no longer inert on every setter.
///
/// `acknowledged` is false when the headset never answered the write. That is
/// reported, not treated as a failure: whether every command is acknowledged
/// has not been established on hardware, so failing here could break commands
/// that work today.
pub fn report_set(state: Value, acknowledged: bool, json: bool) {
    if json {
        let mut value = state;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("ok".into(), Value::Bool(true));
            obj.insert("acknowledged".into(), Value::Bool(acknowledged));
        }
        println!("{value}");
    } else if !acknowledged {
        eprintln!("warning: the headset did not acknowledge the command (is it on and in range?)");
    }
}

/// Report a failure: always a human line on stderr, plus a machine-readable
/// object on stdout under `--json` so a consumer parsing stdout sees the error
/// instead of empty output.
pub fn report_error(err: &AppError, json: bool) {
    eprintln!("error: {err}");
    if json {
        println!(
            "{}",
            json!({ "error": { "kind": err.kind(), "message": err.to_string() } })
        );
    }
}

// ── Field rendering ───────────────────────────────────────────────────────────
//
// ANC is the one field with enough shape to be worth rendering in one place:
// `get anc` and `status` both have to spell out four variants, and a fifth
// spelling of any of them would be a silent inconsistency.

/// ANC state as a human-readable phrase.
pub fn anc_plain(anc: &Anc) -> String {
    match anc {
        Anc::Off => "off".to_string(),
        Anc::On => "on".to_string(),
        Anc::Transparency(level) => format!("transparency (level {level})"),
        Anc::Unknown(code) => format!("unknown (0x{code:02x})"),
    }
}

/// ANC state as its `--json` object.
///
/// `mode` stays a fixed token so a consumer can match on it; an unrecognised
/// byte goes in its own `raw` field rather than inside the token.
pub fn anc_json(anc: &Anc) -> Value {
    match anc {
        Anc::Off => json!({ "mode": "off" }),
        Anc::On => json!({ "mode": "on" }),
        Anc::Transparency(level) => json!({ "mode": "transparency", "level": level }),
        Anc::Unknown(code) => json!({ "mode": "unknown", "raw": code }),
    }
}

/// `up` / `down`, spelled the same by `status` and `info`.
pub fn wireless_link_plain(link: &WirelessLink) -> String {
    match link {
        WirelessLink::Up => "up".to_string(),
        WirelessLink::Down => "down".to_string(),
        WirelessLink::Unknown(code) => format!("unknown (0x{code:02x})"),
    }
}

pub fn wireless_link_json(link: &WirelessLink) -> Value {
    match link {
        WirelessLink::Up => json!({ "up": true }),
        WirelessLink::Down => json!({ "up": false }),
        WirelessLink::Unknown(code) => json!({ "up": null, "code": code }),
    }
}
