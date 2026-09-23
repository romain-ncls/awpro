use hidapi::HidDevice;
use serde_json::json;

use crate::cli::{GetArgs, GetCommand, MicField, PowerField};
use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, op};

pub fn run(device_handle: &HidDevice, args: GetArgs, json: bool) -> Result<(), AppError> {
    match args.command {
        GetCommand::Anc => get_anc(device_handle, json),
        GetCommand::Mic { field } => match field {
            None => get_mic_mute(device_handle, json),
            Some(MicField::NoiseCancel) => get_mic_nc(device_handle, json),
        },
        GetCommand::Sidetone => get_sidetone(device_handle, json),
        GetCommand::Power { field } => match field {
            PowerField::Saving => get_power_saving(device_handle, json),
            PowerField::AutoOff => get_power_auto_off(device_handle, json),
        },
        // Same query and same decoder as `awpro battery`; delegating keeps the
        // two from drifting apart.
        GetCommand::Battery => super::battery::run(device_handle, json),
    }
}

// ── ANC ───────────────────────────────────────────────────────────────────────

fn get_anc(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let reply = device::query(device_handle, op::ANC_GET)?;
    let anc = protocol::parse_anc(&reply);

    output::print(&output::anc_plain(&anc), output::anc_json(&anc), json);
    Ok(())
}

// ── Mic mute ──────────────────────────────────────────────────────────────────

fn get_mic_mute(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let reply = device::query(device_handle, op::MIC_MUTE_GET)?;
    let muted = protocol::parse_mic_muted(&reply);

    output::print(
        if muted { "muted" } else { "unmuted" },
        json!({ "muted": muted }),
        json,
    );
    Ok(())
}

// ── Mic noise cancel ──────────────────────────────────────────────────────────

fn get_mic_nc(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let reply = device::query(device_handle, op::MIC_NOISE_CANCEL)?;
    let enabled = protocol::parse_mic_noise_cancel(&reply);

    output::print(
        if enabled { "on" } else { "off" },
        json!({ "noise_cancel": enabled }),
        json,
    );
    Ok(())
}

// ── Sidetone ──────────────────────────────────────────────────────────────────

fn get_sidetone(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    // UNVERIFIED: queries the mic noise-cancel opcode and reads a byte outside
    // that reply's declared payload. See `protocol::parse_sidetone` for what is
    // and is not known about this.
    let reply = device::query(device_handle, op::MIC_NOISE_CANCEL)?;
    let level = protocol::parse_sidetone(&reply);

    let plain = match level {
        0 => "off".to_string(),
        n => n.to_string(),
    };
    output::print(&plain, json!({ "sidetone": level }), json);
    Ok(())
}

// ── Power saving ──────────────────────────────────────────────────────────────

fn get_power_saving(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let reply = device::query(device_handle, op::POWER_SAVING)?;
    let saving = protocol::parse_power_saving(&reply);

    if saving.enabled {
        output::print(
            &format!("on (threshold: {}%)", saving.threshold),
            json!({ "enabled": true, "threshold": saving.threshold }),
            json,
        );
    } else {
        output::print("off", json!({ "enabled": false }), json);
    }
    Ok(())
}

// ── Power auto-off ────────────────────────────────────────────────────────────

fn get_power_auto_off(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let reply = device::query(device_handle, op::AUTO_OFF)?;
    let auto_off = protocol::parse_auto_off(&reply);

    if auto_off.enabled {
        output::print(
            &auto_off.minutes.to_string(),
            json!({ "enabled": true, "minutes": auto_off.minutes }),
            json,
        );
    } else {
        output::print("off", json!({ "enabled": false }), json);
    }
    Ok(())
}
