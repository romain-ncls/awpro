use hidapi::HidApi;

use crate::cli::{MicArgs, MicCommand, Toggle};
use crate::device;
use crate::error::AppError;

pub fn run(args: MicArgs, _json: bool) -> Result<(), AppError> {
    let api = HidApi::new().map_err(|_| AppError::DeviceNotFound)?;
    let device = device::open(&api)?;

    let payload: &[u8] = match &args.command {
        MicCommand::Mute => &[0x06, 0x40, 0x73, 0x01, 0x00, 0x00],
        MicCommand::Unmute => &[0x06, 0x40, 0x73, 0x01, 0x00, 0x01],
        MicCommand::NoiseCancel { toggle } => match toggle {
            Toggle::On => &[0x06, 0x40, 0x80, 0x01, 0x00, 0x01],
            Toggle::Off => &[0x06, 0x40, 0x80, 0x01, 0x00, 0x00],
        },
    };

    device::write(&device, payload)
}
