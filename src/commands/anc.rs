use hidapi::HidDevice;
use serde_json::{Value, json};

use crate::cli::{AncArgs, AncCommand};
use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, REPORT_LEN, op};

pub fn run(device_handle: &HidDevice, args: AncArgs, json: bool) -> Result<(), AppError> {
    let acknowledged = device::write(device_handle, &frame(&args.command))?;
    output::report_set(state(&args.command), acknowledged, json);
    Ok(())
}

fn frame(cmd: &AncCommand) -> [u8; REPORT_LEN] {
    // Params are `<mode> <level>`. The trailing 0x03 on off/on is what the
    // Windows app sends; it is the default transparency level riding along and
    // is kept byte-for-byte rather than guessed at.
    match cmd {
        AncCommand::Off => protocol::set_frame(op::ANC_SET, &[0x00, 0x03]),
        AncCommand::On => protocol::set_frame(op::ANC_SET, &[0x01, 0x03]),
        AncCommand::Transparency { level } => protocol::set_frame(op::ANC_SET, &[0x02, *level]),
    }
}

/// Keys match what `get anc` emits, so both directions parse the same.
fn state(cmd: &AncCommand) -> Value {
    match cmd {
        AncCommand::Off => json!({ "mode": "off" }),
        AncCommand::On => json!({ "mode": "on" }),
        AncCommand::Transparency { level } => json!({ "mode": "transparency", "level": level }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::assert_frame;

    // Golden frames captured from the Windows app. They cannot be re-derived
    // without the headset, so these assertions are the only record of them.

    #[test]
    fn anc_off_frame() {
        assert_frame(
            &frame(&AncCommand::Off),
            &[0x06, 0x40, 0x75, 0x02, 0x00, 0x00, 0x03],
        );
    }

    #[test]
    fn anc_on_frame() {
        assert_frame(
            &frame(&AncCommand::On),
            &[0x06, 0x40, 0x75, 0x02, 0x00, 0x01, 0x03],
        );
    }

    #[test]
    fn transparency_frame_carries_the_requested_level() {
        assert_frame(
            &frame(&AncCommand::Transparency { level: 5 }),
            &[0x06, 0x40, 0x75, 0x02, 0x00, 0x02, 0x05],
        );
    }
}
