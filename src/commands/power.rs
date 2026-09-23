use hidapi::HidDevice;
use serde_json::{Value, json};

use crate::cli::{AutoOffInterval, PowerArgs, PowerCommand, SavingCommand};
use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, REPORT_LEN, op};

/// The threshold byte the Windows app leaves in place when it disables power
/// saving (`06 40 15 02 00 00 14` in the capture). Sending 0x00 here instead
/// overwrites the stored threshold with a value the CLI itself rejects, so the
/// next `power saving on` from the headset or the vendor app comes back at 0%.
const SAVING_DISABLED_THRESHOLD: u8 = 0x14; // 20%

pub fn run(device_handle: &HidDevice, args: PowerArgs, json: bool) -> Result<(), AppError> {
    let acknowledged = device::write(device_handle, &frame(&args.command))?;
    output::report_set(state(&args.command), acknowledged, json);
    Ok(())
}

fn frame(cmd: &PowerCommand) -> [u8; REPORT_LEN] {
    match cmd {
        PowerCommand::AutoOff { interval } => {
            let enabled = !matches!(interval, AutoOffInterval::Off);
            protocol::set_frame(op::AUTO_OFF, &[u8::from(enabled), interval.code()])
        }
        PowerCommand::Saving { cmd } => match cmd {
            SavingCommand::Off => {
                protocol::set_frame(op::POWER_SAVING, &[0x00, SAVING_DISABLED_THRESHOLD])
            }
            SavingCommand::On { threshold } => {
                protocol::set_frame(op::POWER_SAVING, &[0x01, *threshold])
            }
        },
    }
}

/// Keys match what `get power ...` emits.
fn state(cmd: &PowerCommand) -> Value {
    match cmd {
        PowerCommand::AutoOff { interval } => match interval {
            AutoOffInterval::Off => json!({ "enabled": false }),
            other => {
                json!({ "enabled": true, "minutes": protocol::auto_off_minutes(other.code()) })
            }
        },
        PowerCommand::Saving { cmd } => match cmd {
            SavingCommand::Off => json!({ "enabled": false }),
            SavingCommand::On { threshold } => json!({ "enabled": true, "threshold": threshold }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::assert_frame;

    // Golden frames captured from the Windows app.

    #[test]
    fn auto_off_frames() {
        let cases = [
            (
                AutoOffInterval::Off,
                [0x06, 0x40, 0x8D, 0x02, 0x00, 0x00, 0x02],
            ),
            (
                AutoOffInterval::Min15,
                [0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x01],
            ),
            (
                AutoOffInterval::Min30,
                [0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x02],
            ),
            (
                AutoOffInterval::Min45,
                [0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x03],
            ),
            (
                AutoOffInterval::Min60,
                [0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x04],
            ),
        ];
        for (interval, expected) in cases {
            assert_frame(&frame(&PowerCommand::AutoOff { interval }), &expected);
        }
    }

    #[test]
    fn saving_on_frame_carries_the_threshold() {
        assert_frame(
            &frame(&PowerCommand::Saving {
                cmd: SavingCommand::On { threshold: 0x14 },
            }),
            &[0x06, 0x40, 0x15, 0x02, 0x00, 0x01, 0x14],
        );
    }

    #[test]
    fn saving_off_frame_preserves_the_default_threshold() {
        // Regression: this used to send 0x00 as the threshold, wiping the
        // stored value. The capture keeps 0x14 in place.
        assert_frame(
            &frame(&PowerCommand::Saving {
                cmd: SavingCommand::Off,
            }),
            &[0x06, 0x40, 0x15, 0x02, 0x00, 0x00, 0x14],
        );
    }

    #[test]
    fn auto_off_codes_round_trip_through_the_decoder() {
        let cases = [
            (AutoOffInterval::Min15, 15),
            (AutoOffInterval::Min30, 30),
            (AutoOffInterval::Min45, 45),
            (AutoOffInterval::Min60, 60),
        ];
        for (interval, minutes) in cases {
            assert_eq!(protocol::auto_off_minutes(interval.code()), minutes);
        }
    }
}
