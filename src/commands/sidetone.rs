use hidapi::HidDevice;
use serde_json::json;

use crate::cli::{SidetoneArgs, SidetoneLevel};
use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, REPORT_LEN, op};

pub fn run(device_handle: &HidDevice, args: SidetoneArgs, json: bool) -> Result<(), AppError> {
    let acknowledged = device::write(device_handle, &frame(&args.level))?;
    output::report_set(
        json!({ "sidetone": args.level.as_byte() }),
        acknowledged,
        json,
    );
    Ok(())
}

fn frame(level: &SidetoneLevel) -> [u8; REPORT_LEN] {
    // Level 0 is off; there is no separate on/off parameter on this opcode.
    protocol::set_frame(op::SIDETONE_SET, &[level.as_byte()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::assert_frame;

    // Golden frames captured from the Windows app. 0x8A is the only setter in
    // the tree with no confirmed getter — see `protocol::parse_sidetone`.

    #[test]
    fn sidetone_off_frame() {
        assert_frame(
            &frame(&SidetoneLevel::Off),
            &[0x06, 0x40, 0x8A, 0x01, 0x00, 0x00],
        );
    }

    #[test]
    fn sidetone_level_frame() {
        assert_frame(
            &frame(&SidetoneLevel::L3),
            &[0x06, 0x40, 0x8A, 0x01, 0x00, 0x03],
        );
        assert_frame(
            &frame(&SidetoneLevel::L5),
            &[0x06, 0x40, 0x8A, 0x01, 0x00, 0x05],
        );
    }
}
