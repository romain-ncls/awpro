use hidapi::HidDevice;
use serde_json::json;

use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, op};

pub fn run(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let reply = device::query(device_handle, op::BATTERY)?;
    let level = protocol::parse_battery(&reply);

    output::print(&format!("{level}%"), json!({ "level": level }), json);
    Ok(())
}
