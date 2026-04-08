use hidapi::HidApi;

use crate::cli::{AncArgs, AncCommand};
use crate::device;
use crate::error::AppError;

pub fn run(args: AncArgs, _json: bool) -> Result<(), AppError> {
    let api = HidApi::new().map_err(|_| AppError::DeviceNotFound)?;
    let device = device::open(&api)?;

    let payload: &[u8] = match &args.command {
        AncCommand::Off => &[0x06, 0x40, 0x75, 0x02, 0x00, 0x00, 0x03],
        AncCommand::On => &[0x06, 0x40, 0x75, 0x02, 0x00, 0x01, 0x03],
        AncCommand::Transparency { level } => {
            let lvl = level.unwrap_or(0x03);
            // build in a local buffer so the lifetime is fine
            return run_transparency(&device, lvl);
        }
    };

    device::write(&device, payload)
}

fn run_transparency(device: &hidapi::HidDevice, level: u8) -> Result<(), AppError> {
    let payload = [0x06, 0x40, 0x75, 0x02, 0x00, 0x02, level];
    device::write(device, &payload)
}
