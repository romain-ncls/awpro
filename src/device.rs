use std::thread;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::error::AppError;
use crate::protocol::{self, REPLY_LEN, REPORT_ID_IN, REPORT_LEN, Reply};

pub const VENDOR_ID: u16 = 0x413c;
pub const PRODUCT_ID: u16 = 0xa529;

const POLL_RETRIES: usize = 20;
const POLL_DELAY_MS: u64 = 20;

/// Open the dongle, distinguishing "not plugged in" from "plugged in but we
/// are not allowed to touch it".
///
/// Enumeration succeeds without read/write access to the hidraw node, so a
/// device that is listed and still refuses to open is a permissions problem —
/// reporting that as "device not found" sends people hunting a hardware fault
/// when they are missing a udev rule.
pub fn open(api: &HidApi) -> Result<HidDevice, AppError> {
    match api.open(VENDOR_ID, PRODUCT_ID) {
        Ok(device) => Ok(device),
        Err(e) => {
            let present = api
                .device_list()
                .any(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID);
            if present {
                Err(AppError::Open(e.to_string()))
            } else {
                Err(AppError::DeviceNotFound)
            }
        }
    }
}

/// Send a SET frame and wait briefly for the device's acknowledgement.
///
/// Returns whether an acknowledgement arrived. A missing ACK is deliberately
/// *not* an error: nothing in the captured traffic proves every command is
/// acknowledged, so callers surface it instead of failing on it. It matters
/// because `send_feature_report` succeeds as long as the *dongle* is there —
/// with the headset asleep or out of range the write returns Ok and the
/// setting never lands.
///
/// Draining the ACK also keeps it from contaminating the next query's poll.
pub fn write(device: &HidDevice, frame: &[u8; REPORT_LEN]) -> Result<bool, AppError> {
    device
        .send_feature_report(frame)
        .map_err(|e| AppError::HidWrite(e.to_string()))?;
    Ok(drain(device))
}

/// Read and discard any pending feature report the device queued as an
/// acknowledgement, reporting whether one turned up.
///
/// Polls up to POLL_RETRIES times so that a slow wireless ACK (which may
/// arrive after the initial sleep) is still consumed rather than left to
/// contaminate the next query's poll loop.
fn drain(device: &HidDevice) -> bool {
    for i in 0..POLL_RETRIES {
        if i > 0 {
            thread::sleep(Duration::from_millis(POLL_DELAY_MS));
        }
        let mut buf = [0u8; REPLY_LEN];
        buf[0] = REPORT_ID_IN;
        // A populated report (non-zero header byte) is the ACK. An all-zero
        // buffer means the device has not written anything yet.
        if device.get_feature_report(&mut buf).is_ok() && buf[1] != 0 {
            return true;
        }
    }
    false
}

/// Issue a GET query and poll for the matching response.
///
/// Returns the full reply buffer (index 0 = report ID 0x07) on success.
///
/// The declared payload length in byte 3 is deliberately not used to bound the
/// reply: no decoder has ever been validated against it, and at least one
/// (`protocol::parse_sidetone`) reads outside it on purpose. Adding a bound
/// here would turn working queries into timeouts with no way to retest them.
pub fn query(device: &HidDevice, func: u8) -> Result<Reply, AppError> {
    let req = protocol::get_frame(func);
    device
        .send_feature_report(&req)
        .map_err(|e| AppError::HidWrite(e.to_string()))?;

    let mut consecutive_errors: usize = 0;
    for i in 0..POLL_RETRIES {
        // Sleep after the first attempt so a fast device response is caught
        // immediately rather than always waiting at least POLL_DELAY_MS.
        if i > 0 {
            thread::sleep(Duration::from_millis(POLL_DELAY_MS));
        }
        // Start each poll from a clean buffer so a short read cannot leave
        // bytes from the previous iteration behind, then set byte 0: the
        // hidraw backend reads the report number out of it to pick which
        // report to fetch, so it must be re-set every time.
        let mut buf = [0u8; REPLY_LEN];
        buf[0] = REPORT_ID_IN;
        match device.get_feature_report(&mut buf) {
            Ok(_) => {
                consecutive_errors = 0;
                if protocol::reply_matches(&buf, func) {
                    return Ok(buf);
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors >= 3 {
                    return Err(AppError::HidRead(e.to_string()));
                }
            }
        }
    }
    Err(AppError::Timeout)
}
