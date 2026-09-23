use std::fmt;

use crate::device::{ORDER, Transport, VENDOR_ID};

#[derive(Debug)]
pub enum AppError {
    /// Neither transport is plugged in.
    DeviceNotFound,
    /// A transport is present but would not open — almost always a missing
    /// udev rule rather than a hardware fault.
    Open { transport: Transport, msg: String },
    /// hidapi itself failed to initialise.
    Init(String),
    /// Sending a feature report failed.
    HidWrite(String),
    /// Reading a feature report failed.
    HidRead(String),
    /// The device did not answer a query within the poll window.
    Timeout,
    /// The dongle answered, but the headset is not on its radio link — so
    /// there is nothing on the other end to answer a query.
    HeadsetNotConnected,
    /// Writing to stdout failed for a reason other than the reader leaving.
    Output(String),
}

impl AppError {
    /// Stable machine-readable tag, emitted under `--json`. Unlike the display
    /// message, these strings are part of the output contract.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "device-not-found",
            Self::Open { .. } => "open-failed",
            Self::Init(_) => "init-failed",
            Self::HidWrite(_) => "write-failed",
            Self::HidRead(_) => "read-failed",
            Self::Timeout => "timeout",
            Self::HeadsetNotConnected => "headset-not-connected",
            Self::Output(_) => "output-failed",
        }
    }

    /// Distinct exit codes so a script can tell "dongle unplugged" from
    /// "headset asleep" from "no permission" without parsing the message.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::DeviceNotFound => 2,
            Self::Open { .. } => 3,
            Self::Timeout => 4,
            Self::HeadsetNotConnected => 5,
            Self::Init(_) | Self::HidWrite(_) | Self::HidRead(_) | Self::Output(_) => 1,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound => {
                let looked_for = ORDER
                    .iter()
                    .map(|t| format!("{VENDOR_ID:04x}:{:04x} {}", t.product_id(), t.name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "device not found (looked for {looked_for})")
            }
            Self::Open { transport, msg } => write!(
                f,
                "the {} device is present but could not be opened: {msg} \
                 (is the udev rule for {VENDOR_ID:04x}:{:04x} installed?)",
                transport.name(),
                transport.product_id()
            ),
            Self::Init(msg) => write!(f, "failed to initialise hidapi: {msg}"),
            Self::HidWrite(msg) => write!(f, "failed to send command: {msg}"),
            Self::HidRead(msg) => write!(f, "failed to read device response: {msg}"),
            Self::Timeout => write!(f, "device did not respond"),
            Self::Output(msg) => write!(f, "failed to write output: {msg}"),
            Self::HeadsetNotConnected => write!(
                f,
                "the dongle is connected but the headset is not on its 2.4 GHz link \
                 (is the headset powered on and in range?)"
            ),
        }
    }
}

impl std::error::Error for AppError {}
