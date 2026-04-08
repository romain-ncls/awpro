use hidapi::HidApi;

use crate::cli::{AutoOffInterval, PowerArgs, PowerCommand, SavingCommand};
use crate::device;
use crate::error::AppError;

pub fn run(args: PowerArgs, _json: bool) -> Result<(), AppError> {
    let api = HidApi::new().map_err(|_| AppError::DeviceNotFound)?;
    let device = device::open(&api)?;

    match args.command {
        PowerCommand::AutoOff { interval } => {
            let payload: &[u8] = match interval {
                AutoOffInterval::Off => &[0x06, 0x40, 0x8D, 0x02, 0x00, 0x00, 0x02],
                AutoOffInterval::Min15 => &[0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x01],
                AutoOffInterval::Min30 => &[0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x02],
                AutoOffInterval::Min45 => &[0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x03],
                AutoOffInterval::Min60 => &[0x06, 0x40, 0x8D, 0x02, 0x00, 0x01, 0x04],
            };
            device::write(&device, payload)
        }
        PowerCommand::Saving { cmd } => match cmd {
            SavingCommand::Off => {
                device::write(&device, &[0x06, 0x40, 0x15, 0x02, 0x00, 0x00, 0x00])
            }
            SavingCommand::On { threshold } => {
                let payload = [0x06, 0x40, 0x15, 0x02, 0x00, 0x01, threshold];
                device::write(&device, &payload)
            }
        },
    }
}
