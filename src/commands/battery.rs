use hidapi::HidApi;
use serde_json::json;

use crate::device;
use crate::error::AppError;
use crate::output;

pub fn run(json: bool) -> Result<(), AppError> {
    let api = HidApi::new().map_err(|_| AppError::DeviceNotFound)?;
    let device = device::open(&api)?;

    let buf = device::query(&device, 0x0A)?;
    // Response: 07 C0 0A 0A 00 00 <level>
    let level = buf[6] as u32;

    output::print(&format!("{level}%"), json!({ "level": level }), json);
    Ok(())
}
