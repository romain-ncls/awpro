use std::thread;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::error::AppError;
use crate::protocol::{self, REPLY_LEN, REPORT_ID_IN, REPORT_LEN, Reply, WirelessLink, op};

pub const VENDOR_ID: u16 = 0x413c;

const POLL_RETRIES: usize = 20;
const POLL_DELAY_MS: u64 = 20;

/// How the headset is reached.
///
/// Both expose the same vendor interface: the report descriptors of the two
/// products are byte-identical (usage page 0xFF13, feature report 0x06, input
/// report 0x07) and every opcode in [`crate::protocol::op`] answers the same
/// way over either. Only the product ID differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// The headset itself, plugged in over USB-C.
    Wired,
    /// The 2.4 GHz dongle, with the headset on the other end of the link.
    Dongle,
}

impl Transport {
    pub const fn product_id(self) -> u16 {
        match self {
            Self::Wired => 0xa528,
            Self::Dongle => 0xa529,
        }
    }

    /// Lowercase name, spelled exactly like the CLI flag that forces it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Wired => "wired",
            Self::Dongle => "dongle",
        }
    }
}

/// The transports to try, in order.
///
/// The cable comes first because the two are mutually exclusive in practice:
/// plugging the headset into USB-C drops the 2.4 GHz link, and the dongle —
/// which goes on enumerating and goes on opening — then answers nothing.
/// Preferring the dongle whenever it is present would report "device did not
/// respond" with a working wired headset attached.
///
/// That exclusivity is also why there is no flag to force one: the cable being
/// present *is* the answer, and forcing the dongle could only ever select a
/// device that cannot reply.
pub const ORDER: [Transport; 2] = [Transport::Wired, Transport::Dongle];

/// Open the first candidate transport that is connected, distinguishing "not
/// plugged in" from "plugged in but we are not allowed to touch it".
///
/// Enumeration succeeds without read/write access to the hidraw node, so a
/// device that is listed and still refuses to open is a permissions problem —
/// reporting that as "device not found" sends people hunting a hardware fault
/// when they are missing a udev rule.
///
/// A candidate that is present but unopenable ends the search rather than
/// falling through to the next one. Falling through would trade an actionable
/// "install the udev rule" for a confusing timeout: the next candidate is the
/// dongle, and a dongle whose headset is on the cable opens happily and then
/// answers nothing.
pub fn open(api: &HidApi) -> Result<(HidDevice, Transport), AppError> {
    for transport in ORDER {
        match api.open(VENDOR_ID, transport.product_id()) {
            Ok(device) => return Ok((device, transport)),
            Err(e) => {
                let present = api.device_list().any(|d| {
                    d.vendor_id() == VENDOR_ID && d.product_id() == transport.product_id()
                });
                if present {
                    return Err(AppError::Open {
                        transport,
                        msg: e.to_string(),
                    });
                }
            }
        }
    }

    Err(AppError::DeviceNotFound)
}

/// Send a SET frame and wait briefly for the device's acknowledgement.
///
/// Returns whether an acknowledgement arrived. A missing ACK is deliberately
/// *not* an error: nothing in the captured traffic proves every command is
/// acknowledged, so callers surface it instead of failing on it. It matters
/// because `send_feature_report` succeeds as long as the *dongle* is there —
/// with the headset asleep or out of range the write returns Ok and the
/// setting never lands.
///
/// Draining the ACK also keeps it from contaminating the next query's poll.
pub fn write(device: &HidDevice, frame: &[u8; REPORT_LEN]) -> Result<bool, AppError> {
    device
        .send_feature_report(frame)
        .map_err(|e| AppError::HidWrite(e.to_string()))?;
    Ok(drain(device))
}

/// Read and discard any pending feature report the device queued as an
/// acknowledgement, reporting whether one turned up.
///
/// Polls up to POLL_RETRIES times so that a slow wireless ACK (which may
/// arrive after the initial sleep) is still consumed rather than left to
/// contaminate the next query's poll loop.
fn drain(device: &HidDevice) -> bool {
    for i in 0..POLL_RETRIES {
        if i > 0 {
            thread::sleep(Duration::from_millis(POLL_DELAY_MS));
        }
        let mut buf = [0u8; REPLY_LEN];
        buf[0] = REPORT_ID_IN;
        // A populated report (non-zero header byte) is the ACK. An all-zero
        // buffer means the device has not written anything yet.
        if device.get_feature_report(&mut buf).is_ok() && buf[1] != 0 {
            return true;
        }
    }
    false
}

/// Issue a GET query and poll for the matching response.
///
/// Returns the full reply buffer (index 0 = report ID 0x07) on success.
///
/// The declared payload length in byte 3 is deliberately not used to bound the
/// reply: no decoder has ever been validated against it. (An earlier comment
/// here justified that by saying `protocol::parse_sidetone` reads outside the
/// declared length — it does not; the 0x80 reply declares 4 and that decoder
/// reads byte 8, the last byte inside it.) Adding a bound would turn working
/// queries into timeouts with no way to retest them, for no benefit.
pub fn query(device: &HidDevice, func: u8) -> Result<Reply, AppError> {
    query_frame(device, protocol::get_frame(func), func)
}

/// Query `func` with parameters. Only `op::IDENTITY` needs them.
pub fn query_with_params(device: &HidDevice, func: u8, params: &[u8]) -> Result<Reply, AppError> {
    query_frame(device, protocol::get_frame_with_params(func, params), func)
}

fn query_frame(device: &HidDevice, req: [u8; REPORT_LEN], func: u8) -> Result<Reply, AppError> {
    device
        .send_feature_report(&req)
        .map_err(|e| AppError::HidWrite(e.to_string()))?;

    let mut consecutive_errors: usize = 0;
    for i in 0..POLL_RETRIES {
        // Sleep after the first attempt so a fast device response is caught
        // immediately rather than always waiting at least POLL_DELAY_MS.
        if i > 0 {
            thread::sleep(Duration::from_millis(POLL_DELAY_MS));
        }
        // Start each poll from a clean buffer so a short read cannot leave
        // bytes from the previous iteration behind, then set byte 0: the
        // hidraw backend reads the report number out of it to pick which
        // report to fetch, so it must be re-set every time.
        let mut buf = [0u8; REPLY_LEN];
        buf[0] = REPORT_ID_IN;
        match device.get_feature_report(&mut buf) {
            Ok(_) => {
                consecutive_errors = 0;
                if protocol::reply_matches(&buf, func) {
                    return Ok(buf);
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors >= 3 {
                    return Err(AppError::HidRead(e.to_string()));
                }
            }
        }
    }
    Err(AppError::Timeout)
}

/// Turn a bare timeout into the specific diagnosis, when there is one.
///
/// Only the dongle can be explained this way. Over the cable the link reads
/// `Down` as a matter of course — the headset is on USB-C, not the radio — so
/// blaming a wired timeout on the link would be wrong every time. A link that
/// is up, or a link query that itself failed, leaves the timeout as it was:
/// nothing more is known, so nothing more is claimed.
fn explain_timeout(transport: Transport, link: Option<WirelessLink>) -> AppError {
    match (transport, link) {
        (Transport::Dongle, Some(WirelessLink::Down)) => AppError::HeadsetNotConnected,
        _ => AppError::Timeout,
    }
}

/// Re-describe `error` if it is a timeout the link state accounts for.
///
/// Runs only on the failure path, and asks the one opcode the dongle answers
/// on its own: with the headset off, 0x09 still replies while every other
/// query, identity included, returns nothing.
pub fn diagnose(device: &HidDevice, transport: Transport, error: AppError) -> AppError {
    if !matches!(error, AppError::Timeout) {
        return error;
    }
    let link = query(device, op::WIRELESS_LINK)
        .ok()
        .map(|reply| protocol::parse_wireless_link(&reply));
    explain_timeout(transport, link)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cable_is_tried_before_the_dongle() {
        // With the headset plugged in over USB-C the dongle still enumerates
        // and still opens, but the 2.4 GHz link is down and every query times
        // out — so presence alone cannot pick it. Wired has to come first.
        assert_eq!(ORDER, [Transport::Wired, Transport::Dongle]);
    }

    #[test]
    fn transports_carry_the_product_ids_seen_on_the_bus() {
        assert_eq!(Transport::Dongle.product_id(), 0xa529);
        assert_eq!(Transport::Wired.product_id(), 0xa528);
    }

    #[test]
    fn a_dongle_timeout_with_the_link_down_names_the_real_problem() {
        // The common case: dongle in the port, headset off or out of range.
        // "device did not respond" sends people hunting a driver fault.
        assert!(matches!(
            explain_timeout(Transport::Dongle, Some(WirelessLink::Down)),
            AppError::HeadsetNotConnected
        ));
    }

    #[test]
    fn a_wired_timeout_is_never_blamed_on_the_link() {
        // Over the cable the link reads Down as a matter of course — the
        // headset is on USB-C, not the radio. Reporting that as "headset not
        // connected" would be wrong every single time.
        assert!(matches!(
            explain_timeout(Transport::Wired, Some(WirelessLink::Down)),
            AppError::Timeout
        ));
    }

    #[test]
    fn a_timeout_stays_a_timeout_when_the_link_does_not_explain_it() {
        // Link up, or the link query itself failed: nothing more is known
        // than before, so nothing more is claimed.
        assert!(matches!(
            explain_timeout(Transport::Dongle, Some(WirelessLink::Up)),
            AppError::Timeout
        ));
        assert!(matches!(
            explain_timeout(Transport::Dongle, None),
            AppError::Timeout
        ));
    }

    #[test]
    fn transport_names_are_the_flag_spellings() {
        // These strings appear in error messages next to `--wired` / `--dongle`
        // advice, so they must match the flags the CLI accepts.
        assert_eq!(Transport::Wired.name(), "wired");
        assert_eq!(Transport::Dongle.name(), "dongle");
    }
}
