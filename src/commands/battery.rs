use hidapi::HidDevice;
use serde_json::json;

use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, op};

pub fn run(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let reply = device::query(device_handle, op::BATTERY)?;
    let level = protocol::parse_battery(&reply);
    // The same reply carries the charging flag; `status` reads both out of it
    // too, and the two must keep emitting the same object.
    let charging = protocol::parse_charging(&reply);

    let plain = if charging {
        format!("{level}% (charging)")
    } else {
        format!("{level}%")
    };
    output::print(
        &plain,
        json!({ "level": level, "charging": charging }),
        json,
    );
    Ok(())
}
