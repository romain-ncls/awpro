use std::fmt;

#[derive(Debug)]
pub enum AppError {
    DeviceNotFound,
    HidWrite(String),
    Timeout,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => {
                write!(f, "device not found (VID 0x413c, PID 0xa529)")
            }
            Self::HidWrite(msg) => {
                write!(f, "failed to send command: {msg}")
            }
            Self::Timeout => {
                write!(f, "device did not respond")
            }
        }
    }
}

impl From<hidapi::HidError> for AppError {
    fn from(e: hidapi::HidError) -> Self {
        match &e {
            hidapi::HidError::HidApiError { message }
                if message.contains("not found")
                    || message.contains("No such device")
                    || message.contains("cannot open device") =>
            {
                Self::DeviceNotFound
            }
            _ => Self::HidWrite(e.to_string()),
        }
    }
}
