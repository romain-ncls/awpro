//! `awpro info` — what the headset is, rather than how it is set.

use hidapi::HidDevice;
use serde_json::{Value, json};

use crate::device;
use crate::error::AppError;
use crate::output;
use crate::protocol::{self, Identity, WirelessLink, op};

/// Width of the label column, matching `status` so the two read alike.
const LABEL_WIDTH: usize = 18;

/// What `info` prints.
///
/// The identity query and the link query fail independently, so each is
/// optional: a headset that answers one and not the other still prints what it
/// did answer.
#[derive(Debug, Default)]
struct Info {
    identity: Option<Identity>,
    wireless_link: Option<WirelessLink>,
}

impl Info {
    /// One `(label, value)` pair per field, in display order.
    fn rows(&self) -> Vec<(&'static str, String)> {
        let unavailable = || "unavailable".to_string();
        vec![
            (
                "firmware",
                self.identity
                    .as_ref()
                    .map_or_else(unavailable, |i| i.firmware.to_string()),
            ),
            (
                "product id",
                self.identity
                    .as_ref()
                    .map_or_else(unavailable, |i| format!("0x{:04x}", i.product_id)),
            ),
            (
                "wireless link",
                self.wireless_link
                    .as_ref()
                    .map_or_else(unavailable, output::wireless_link_plain),
            ),
        ]
    }

    fn plain(&self) -> String {
        self.rows()
            .iter()
            .map(|(label, value)| format!("{label:<LABEL_WIDTH$}{value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> Value {
        json!({
            "firmware": self.identity.as_ref().map(|i| json!({
                "version": i.firmware.to_string(),
                "major": i.firmware.major,
                "minor": i.firmware.minor,
                "patch": i.firmware.patch,
            })),
            "product_id": self.identity.as_ref().map(|i| format!("0x{:04x}", i.product_id)),
            "wireless_link": self.wireless_link.as_ref().map(output::wireless_link_json),
        })
    }
}

pub fn run(device_handle: &HidDevice, json: bool) -> Result<(), AppError> {
    let mut info = Info::default();

    // The identity query is the only one in the protocol that carries
    // parameters, and it answers something else without them.
    let identity =
        device::query_with_params(device_handle, op::IDENTITY, &protocol::IDENTITY_PARAMS);
    let link = device::query(device_handle, op::WIRELESS_LINK);

    // Neither failing alone is fatal, but a device that answered nothing at
    // all is: reporting three `unavailable` rows as success would hide a
    // headset that is simply not there.
    match (identity, link) {
        (Err(e), Err(_)) => return Err(e),
        (identity, link) => {
            if let Ok(reply) = identity {
                info.identity = Some(protocol::parse_identity(&reply));
            }
            if let Ok(reply) = link {
                info.wireless_link = Some(protocol::parse_wireless_link(&reply));
            }
        }
    }

    output::print(&info.plain(), info.json(), json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::FirmwareVersion;

    fn full() -> Info {
        Info {
            identity: Some(Identity {
                firmware: FirmwareVersion {
                    major: 4,
                    minor: 1,
                    patch: 1,
                },
                product_id: 0xA528,
            }),
            wireless_link: Some(WirelessLink::Down),
        }
    }

    #[test]
    fn rows_name_the_firmware_and_the_headset() {
        assert_eq!(
            full().rows(),
            vec![
                ("firmware", "4.1.1".to_string()),
                ("product id", "0xa528".to_string()),
                ("wireless link", "down".to_string()),
            ]
        );
    }

    #[test]
    fn a_field_that_could_not_be_read_says_so_rather_than_guessing() {
        let nothing = Info::default();
        assert_eq!(
            nothing.rows(),
            vec![
                ("firmware", "unavailable".to_string()),
                ("product id", "unavailable".to_string()),
                ("wireless link", "unavailable".to_string()),
            ]
        );
    }

    #[test]
    fn json_carries_the_version_parts_as_well_as_the_string() {
        // The string is for people; the parts are so a script can compare
        // versions without parsing it.
        assert_eq!(
            full().json(),
            json!({
                "firmware": { "version": "4.1.1", "major": 4, "minor": 1, "patch": 1 },
                "product_id": "0xa528",
                "wireless_link": { "up": false }
            })
        );
    }
}
