use hidapi::HidDevice;
use serde_json::{Value, json};

use crate::cli::{MicArgs, MicCommand};
use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, REPORT_LEN, op};

pub fn run(device_handle: &HidDevice, args: MicArgs, json: bool) -> Result<(), AppError> {
    let acknowledged = device::write(device_handle, &frame(&args.command))?;
    output::report_set(state(&args.command), acknowledged, json);
    Ok(())
}

fn frame(cmd: &MicCommand) -> [u8; REPORT_LEN] {
    match cmd {
        // 0x00 mutes, 0x01 unmutes — `protocol::parse_mic_muted` decodes the
        // reply with the same polarity.
        MicCommand::Mute => protocol::set_frame(op::MIC_MUTE_SET, &[0x00]),
        MicCommand::Unmute => protocol::set_frame(op::MIC_MUTE_SET, &[0x01]),
        MicCommand::NoiseCancel { toggle } => {
            protocol::set_frame(op::MIC_NOISE_CANCEL, &[toggle.as_byte()])
        }
    }
}

/// Keys match what `get mic` emits.
fn state(cmd: &MicCommand) -> Value {
    match cmd {
        MicCommand::Mute => json!({ "muted": true }),
        MicCommand::Unmute => json!({ "muted": false }),
        MicCommand::NoiseCancel { toggle } => json!({ "noise_cancel": toggle.as_byte() != 0 }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Toggle;
    use crate::protocol::assert_frame;

    // Golden frames captured from the Windows app.

    #[test]
    fn mute_frame() {
        assert_frame(
            &frame(&MicCommand::Mute),
            &[0x06, 0x40, 0x73, 0x01, 0x00, 0x00],
        );
    }

    #[test]
    fn unmute_frame() {
        assert_frame(
            &frame(&MicCommand::Unmute),
            &[0x06, 0x40, 0x73, 0x01, 0x00, 0x01],
        );
    }

    #[test]
    fn noise_cancel_frames() {
        assert_frame(
            &frame(&MicCommand::NoiseCancel { toggle: Toggle::On }),
            &[0x06, 0x40, 0x80, 0x01, 0x00, 0x01],
        );
        assert_frame(
            &frame(&MicCommand::NoiseCancel {
                toggle: Toggle::Off,
            }),
            &[0x06, 0x40, 0x80, 0x01, 0x00, 0x00],
        );
    }
}
