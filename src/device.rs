use std::thread;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::error::AppError;

pub const VENDOR_ID: u16 = 0x413c;
pub const PRODUCT_ID: u16 = 0xa529;

const REPORT_LEN: usize = 62;
const POLL_RETRIES: usize = 50;
const POLL_DELAY_MS: u64 = 100;

pub fn open(api: &HidApi) -> Result<HidDevice, AppError> {
    api.open(VENDOR_ID, PRODUCT_ID)
        .map_err(|_| AppError::DeviceNotFound)
}

/// Send a 62-byte feature report (Report ID 6).
/// `payload` holds only the meaningful bytes; the rest are zeroed.
pub fn write(device: &HidDevice, payload: &[u8]) -> Result<(), AppError> {
    debug_assert!(payload.len() <= REPORT_LEN, "payload exceeds report length");
    let mut buf = [0u8; REPORT_LEN];
    buf[..payload.len()].copy_from_slice(payload);
    device
        .send_feature_report(&buf)
        .map_err(|e| AppError::HidWrite(e.to_string()))?;
    Ok(())
}

/// Issue a GET query (Report ID 6, byte[1] = 0xC0) and poll for the response
/// (Report ID 7, matched on buf[1] == 0xC0 && buf[2] == cmd).
///
/// Returns the full 62-byte response buffer on success.
pub fn query(device: &HidDevice, cmd: u8) -> Result<[u8; REPORT_LEN], AppError> {
    let mut req = [0u8; REPORT_LEN];
    req[0] = 0x06;
    req[1] = 0xC0;
    req[2] = cmd;
    device
        .send_feature_report(&req)
        .map_err(|e| AppError::HidWrite(e.to_string()))?;

    let mut buf = [0u8; REPORT_LEN];
    for _ in 0..POLL_RETRIES {
        thread::sleep(Duration::from_millis(POLL_DELAY_MS));
        match device.get_feature_report(&mut buf) {
            Ok(_) => {
                if buf[1] == 0xC0 && buf[2] == cmd {
                    return Ok(buf);
                }
            }
            Err(_) => {}
        }
    }
    Err(AppError::Timeout)
}
