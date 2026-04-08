use hidapi::HidApi;

use crate::cli::SidetoneArgs;
use crate::device;
use crate::error::AppError;

pub fn run(args: SidetoneArgs, _json: bool) -> Result<(), AppError> {
    let api = HidApi::new().map_err(|_| AppError::DeviceNotFound)?;
    let device = device::open(&api)?;

    let payload = [0x06, 0x40, 0x8A, 0x01, 0x00, args.level.as_byte()];
    device::write(&device, &payload)
}
