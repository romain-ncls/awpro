use std::fmt;

use crate::device::{PRODUCT_ID, VENDOR_ID};

#[derive(Debug)]
pub enum AppError {
    /// No device with our VID/PID is present.
    DeviceNotFound,
    /// The device is present but would not open — almost always a missing
    /// udev rule rather than a hardware fault.
    Open(String),
    /// hidapi itself failed to initialise.
    Init(String),
    /// Sending a feature report failed.
    HidWrite(String),
    /// Reading a feature report failed.
    HidRead(String),
    /// The device did not answer a query within the poll window.
    Timeout,
}

impl AppError {
    /// Stable machine-readable tag, emitted under `--json`. Unlike the display
    /// message, these strings are part of the output contract.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "device-not-found",
            Self::Open(_) => "open-failed",
            Self::Init(_) => "init-failed",
            Self::HidWrite(_) => "write-failed",
            Self::HidRead(_) => "read-failed",
            Self::Timeout => "timeout",
        }
    }

    /// Distinct exit codes so a script can tell "dongle unplugged" from
    /// "headset asleep" from "no permission" without parsing the message.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::DeviceNotFound => 2,
            Self::Open(_) => 3,
            Self::Timeout => 4,
            Self::Init(_) | Self::HidWrite(_) | Self::HidRead(_) => 1,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => {
                write!(
                    f,
                    "device not found (VID 0x{VENDOR_ID:04x}, PID 0x{PRODUCT_ID:04x})"
                )
            }
            Self::Open(msg) => write!(
                f,
                "device is present but could not be opened: {msg} \
                 (is the udev rule for {VENDOR_ID:04x}:{PRODUCT_ID:04x} installed?)"
            ),
            Self::Init(msg) => write!(f, "failed to initialise hidapi: {msg}"),
            Self::HidWrite(msg) => write!(f, "failed to send command: {msg}"),
            Self::HidRead(msg) => write!(f, "failed to read device response: {msg}"),
            Self::Timeout => write!(f, "device did not respond"),
        }
    }
}

impl std::error::Error for AppError {}
