use serde_json::{Value, json};

use crate::error::AppError;

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
