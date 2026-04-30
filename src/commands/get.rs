use hidapi::HidApi;
use serde_json::json;

use crate::cli::{GetArgs, GetCommand, MicField, PowerField};
use crate::device;
use crate::error::AppError;
use crate::output;

pub fn run(args: GetArgs, json: bool) -> Result<(), AppError> {
    let api = HidApi::new().map_err(|_| AppError::DeviceNotFound)?;
    let device = device::open(&api)?;

    match args.command {
        GetCommand::Anc => get_anc(&device, json),
        GetCommand::Mic { field } => match field {
            None => get_mic_mute(&device, json),
            Some(MicField::NoiseCancel) => get_mic_nc(&device, json),
        },
        GetCommand::Sidetone => get_sidetone(&device, json),
        GetCommand::Power { field } => match field {
            PowerField::Saving => get_power_saving(&device, json),
            PowerField::AutoOff => get_power_auto_off(&device, json),
        },
        GetCommand::Battery => {
            let buf = device::query(&device, 0x0A)?;
            let level = buf[6] as u32;
            output::print(&format!("{level}%"), json!({ "level": level }), json);
            Ok(())
        }
    }
}

// ── ANC ───────────────────────────────────────────────────────────────────────

fn get_anc(device: &hidapi::HidDevice, json: bool) -> Result<(), AppError> {
    let buf = device::query(device, 0x77)?;
    // Response: 07 C0 77 02 00 <mode> <level>
    let mode = buf[5];
    let level = buf[6];

    match mode {
        0x00 => output::print("off", json!({ "mode": "off" }), json),
        0x01 => output::print("on", json!({ "mode": "on" }), json),
        0x02 => {
            let plain = format!("transparency (level {level})");
            output::print(
                &plain,
                json!({ "mode": "transparency", "level": level }),
                json,
            );
        }
        other => {
            let plain = format!("unknown (0x{other:02x})");
            output::print(
                &plain,
                json!({ "mode": format!("unknown (0x{other:02x})") }),
                json,
            );
        }
    }
    Ok(())
}

// ── Mic mute ──────────────────────────────────────────────────────────────────

fn get_mic_mute(device: &hidapi::HidDevice, json: bool) -> Result<(), AppError> {
    let buf = device::query(device, 0x76)?;
    // Response: 07 C0 76 01 00 <muted>
    let muted = buf[5] == 0;
    let plain = if muted { "muted" } else { "unmuted" };
    output::print(plain, json!({ "muted": muted }), json);
    Ok(())
}

// ── Mic noise cancel ──────────────────────────────────────────────────────────

fn get_mic_nc(device: &hidapi::HidDevice, json: bool) -> Result<(), AppError> {
    let buf = device::query(device, 0x80)?;
    // Response: 07 C0 80 01 00 <enabled>
    let enabled = buf[5] != 0;
    let plain = if enabled { "on" } else { "off" };
    output::print(plain, json!({ "noise_cancel": enabled }), json);
    Ok(())
}

// ── Sidetone ──────────────────────────────────────────────────────────────────

fn get_sidetone(device: &hidapi::HidDevice, json: bool) -> Result<(), AppError> {
    let buf = device::query(device, 0x80)?;
    // Response: 07 C0 80 ... <sidetone_level at byte 8>
    let level = buf[8];
    let plain = match level {
        0 => "off".to_string(),
        n => n.to_string(),
    };
    output::print(&plain, json!({ "sidetone": level }), json);
    Ok(())
}

// ── Power saving ──────────────────────────────────────────────────────────────

fn get_power_saving(device: &hidapi::HidDevice, json: bool) -> Result<(), AppError> {
    let buf = device::query(device, 0x15)?;
    // Response: 07 C0 15 02 00 <enabled> <threshold>
    let enabled = buf[5] != 0;
    let threshold = buf[6] as u32;

    if enabled {
        output::print(
            &format!("on (threshold: {threshold}%)"),
            json!({ "enabled": true, "threshold": threshold }),
            json,
        );
    } else {
        output::print("off", json!({ "enabled": false }), json);
    }
    Ok(())
}

// ── Power auto-off ────────────────────────────────────────────────────────────

fn get_power_auto_off(device: &hidapi::HidDevice, json: bool) -> Result<(), AppError> {
    // GET command for auto-off is not confirmed — timeout treated as error
    let buf = device::query(device, 0x8D)?;
    // Expected: 07 C0 8D 02 00 <enabled> <interval_code>
    let enabled = buf[5] != 0;
    let interval_code = buf[6];

    if enabled {
        let minutes: u32 = match interval_code {
            0x01 => 15,
            0x02 => 30,
            0x03 => 45,
            0x04 => 60,
            other => other as u32 * 15, // best-effort fallback
        };
        output::print(
            &format!("{minutes}"),
            json!({ "enabled": true, "minutes": minutes }),
            json,
        );
    } else {
        output::print("off", json!({ "enabled": false }), json);
    }
    Ok(())
}
