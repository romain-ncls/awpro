//! Wire-frame construction, the opcode table, and reply decoding.
//!
//! Every exchange is a HID feature report. Only the leading bytes carry
//! meaning; the rest of the report is zero padding.
//!
//! ```text
//! SET    [0x06, 0x40, func, param_count, 0x00, params...]   62 bytes
//! GET    [0x06, 0xC0, func]                                 62 bytes
//! reply  [0x07, 0xC0, func, payload_len, 0x00, payload...]  63 bytes (index 0 = report ID)
//! ```
//!
//! The opcodes below were recovered by capturing USB traffic from the Windows
//! app. They are *not* the function IDs found in the decompiled Airoha SDK
//! under `context/`: 0x1b, 0x1c, 0x63 and 0x64 were taken from there, tried on
//! real hardware, and every one of them timed out. Treat any opcode that has
//! not been seen on the wire as unconfirmed.

/// Bytes in an outgoing feature report, including the report ID at index 0.
pub const REPORT_LEN: usize = 62;

/// Bytes in a reply buffer: the report payload plus the report ID that hidraw
/// writes at index 0.
pub const REPLY_LEN: usize = REPORT_LEN + 1;

/// `[report_id, direction, func, param_count, 0x00]` — payload starts at 5.
const HEADER_LEN: usize = 5;

const REPORT_ID_OUT: u8 = 0x06;
const DIR_SET: u8 = 0x40;
const DIR_GET: u8 = 0xC0;

/// Report ID the device answers on.
pub const REPORT_ID_IN: u8 = 0x07;

/// A reply as returned by [`crate::device::query`]: index 0 is the report ID.
pub type Reply = [u8; REPLY_LEN];

/// Opcodes. Where a setter and a getter use *different* IDs for the same
/// feature that is noted — it is not a typo, it is what the capture shows.
pub mod op {
    /// Battery level. Query only.
    pub const BATTERY: u8 = 0x0A;
    /// Power saving on/off plus battery threshold. Same ID both ways.
    pub const POWER_SAVING: u8 = 0x15;
    /// Auto power-off. Same ID both ways.
    pub const AUTO_OFF: u8 = 0x8D;
    /// Mic mute — setter. The getter uses a different ID.
    pub const MIC_MUTE_SET: u8 = 0x73;
    /// Mic mute — getter. The setter uses a different ID.
    pub const MIC_MUTE_GET: u8 = 0x76;
    /// ANC / transparency — setter. The getter uses a different ID.
    pub const ANC_SET: u8 = 0x75;
    /// ANC / transparency — getter. The setter uses a different ID.
    pub const ANC_GET: u8 = 0x77;
    /// Mic (uplink) noise cancellation. Same ID both ways.
    pub const MIC_NOISE_CANCEL: u8 = 0x80;
    /// Sidetone level. Setter only — querying 0x8A times out, so the level is
    /// read back out of the 0x80 reply instead, see [`super::parse_sidetone`].
    pub const SIDETONE_SET: u8 = 0x8A;
    /// State of the 2.4 GHz link to the headset. Query only.
    ///
    /// Not a setting: it reports whether the headset is on the dongle's radio
    /// link. A full sweep of all 256 opcodes found this one answering, and
    /// watching it across cable plug/unplug cycles showed it tracking the link
    /// exactly.
    pub const WIRELESS_LINK: u8 = 0x09;
}

/// Build a SET frame, zero-padded to a full report.
///
/// The parameter-count byte is derived from `params`, never hand-typed: a
/// frame whose count disagrees with the bytes that follow is accepted by the
/// USB stack and silently ignored by the headset, which looks exactly like a
/// wrong opcode.
pub fn set_frame(func: u8, params: &[u8]) -> [u8; REPORT_LEN] {
    assert!(
        params.len() <= REPORT_LEN - HEADER_LEN,
        "SET frame for 0x{func:02x} carries {} params, the report holds {}",
        params.len(),
        REPORT_LEN - HEADER_LEN
    );
    let mut buf = [0u8; REPORT_LEN];
    buf[0] = REPORT_ID_OUT;
    buf[1] = DIR_SET;
    buf[2] = func;
    buf[3] = params.len() as u8;
    buf[HEADER_LEN..HEADER_LEN + params.len()].copy_from_slice(params);
    buf
}

/// Build a GET frame, zero-padded to a full report.
pub fn get_frame(func: u8) -> [u8; REPORT_LEN] {
    let mut buf = [0u8; REPORT_LEN];
    buf[0] = REPORT_ID_OUT;
    buf[1] = DIR_GET;
    buf[2] = func;
    buf
}

/// Whether `reply` is the answer to a query for `func`.
pub fn reply_matches(reply: &Reply, func: u8) -> bool {
    reply[1] == DIR_GET && reply[2] == func
}

// ── Reply decoding ────────────────────────────────────────────────────────────
//
// These take the whole fixed-size buffer rather than a slice so that no index
// below can ever be out of bounds, whatever the device sent. None of them
// consults the declared payload length in byte 3: nothing needs it, and no
// decoder has been validated against it.

#[derive(Debug, PartialEq, Eq)]
pub enum Anc {
    Off,
    On,
    Transparency(u8),
    Unknown(u8),
}

/// `07 C0 77 02 00 <mode> <level>`
pub fn parse_anc(reply: &Reply) -> Anc {
    match reply[5] {
        0x00 => Anc::Off,
        0x01 => Anc::On,
        0x02 => Anc::Transparency(reply[6]),
        other => Anc::Unknown(other),
    }
}

/// `07 C0 0A 0A 00 00 <level>` — byte 5 is a constant 0x00, the level is at 6.
pub fn parse_battery(reply: &Reply) -> u8 {
    reply[6]
}

/// State of the 2.4 GHz link between dongle and headset.
#[derive(Debug, PartialEq, Eq)]
pub enum WirelessLink {
    /// The headset is on the dongle's radio link.
    Up,
    /// It is not — because it is on the cable, or off.
    Down,
    Unknown(u8),
}

/// `07 C0 09 01 00 <state>`
///
/// The two values are `0xCC` and `0xDD`, and nothing else was ever observed
/// across four cable plug/unplug transitions watched on both hidraw nodes.
/// This is the pollable form of notification field 0x01, which carries the
/// same byte.
pub fn parse_wireless_link(reply: &Reply) -> WirelessLink {
    match reply[5] {
        0xCC => WirelessLink::Up,
        0xDD => WirelessLink::Down,
        other => WirelessLink::Unknown(other),
    }
}

/// Whether the headset is charging, out of the same 0x0A reply as the level.
///
/// Byte 7 is 1 while the cable is supplying power and 0 on battery. Confirmed
/// both ways on this hardware: every 0x0A reply captured from AWCC over the
/// dongle has 0 there, and reading the same opcode over USB-C gives 1.
pub fn parse_charging(reply: &Reply) -> bool {
    reply[7] != 0
}

/// `07 C0 76 01 00 <unmuted>` — 0 means muted, which matches the setter in
/// [`crate::commands::mic`] where Mute sends 0x00 and Unmute sends 0x01.
pub fn parse_mic_muted(reply: &Reply) -> bool {
    reply[5] == 0
}

/// `07 C0 80 04 00 <enabled> 00 00 <sidetone>` — the reply declares four
/// payload bytes and carries the sidetone level in the last of them, which
/// [`parse_sidetone`] reads.
pub fn parse_mic_noise_cancel(reply: &Reply) -> bool {
    reply[5] != 0
}

/// Sidetone level, read out of the *mic noise-cancel* reply (0x80).
///
/// There is still no sidetone getter — querying 0x8A times out — but byte 8 of
/// the 0x80 reply is confirmed to hold the level: over the cable, `sidetone 4`
/// reads back as 4 and `sidetone off` as 0, and the reply declares a payload
/// length of 4 in byte 3 (`07 C0 80 04 00 01 00 00 00`), which puts byte 8 at
/// the end of its own declared payload rather than outside it.
///
/// Earlier comments here called this unverified and claimed a declared length
/// of 1; both were wrong. Do not swap in 0x1b from the decompiled SDK, which
/// does still time out on this hardware.
pub fn parse_sidetone(reply: &Reply) -> u8 {
    reply[8]
}

#[derive(Debug, PartialEq, Eq)]
pub struct PowerSaving {
    pub enabled: bool,
    pub threshold: u8,
}

/// `07 C0 15 02 00 <enabled> <threshold>`
pub fn parse_power_saving(reply: &Reply) -> PowerSaving {
    PowerSaving {
        enabled: reply[5] != 0,
        threshold: reply[6],
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AutoOff {
    pub enabled: bool,
    pub minutes: u32,
}

/// `07 C0 8D 02 00 <enabled> <interval_code>`
///
/// Confirmed on the dongle: with the headset on a 30-minute timer the query
/// answers `07 C0 8D 02 00 01 02`, so the interval code matches the one the
/// setter sends. (Earlier comments here called the query unconfirmed.)
pub fn parse_auto_off(reply: &Reply) -> AutoOff {
    AutoOff {
        enabled: reply[5] != 0,
        minutes: auto_off_minutes(reply[6]),
    }
}

/// Interval code to minutes, mirroring the codes the setter sends.
pub fn auto_off_minutes(code: u8) -> u32 {
    match code {
        0x01 => 15,
        0x02 => 30,
        0x03 => 45,
        0x04 => 60,
        other => u32::from(other) * 15, // best-effort fallback
    }
}

/// Assert that a frame starts with `head` and is zero-padded after it.
#[cfg(test)]
pub(crate) fn assert_frame(frame: &[u8; REPORT_LEN], head: &[u8]) {
    assert_eq!(&frame[..head.len()], head, "frame head mismatch");
    assert!(
        frame[head.len()..].iter().all(|&b| b == 0),
        "frame is not zero-padded after byte {}",
        head.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(head: &[u8]) -> Reply {
        let mut buf = [0u8; REPLY_LEN];
        buf[..head.len()].copy_from_slice(head);
        buf
    }

    #[test]
    fn set_frame_derives_the_param_count() {
        let frame = set_frame(0x42, &[0xAA, 0xBB, 0xCC]);
        assert_frame(&frame, &[0x06, 0x40, 0x42, 0x03, 0x00, 0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn set_frame_with_no_params_declares_zero() {
        assert_frame(&set_frame(0x42, &[]), &[0x06, 0x40, 0x42, 0x00, 0x00]);
    }

    #[test]
    #[should_panic(expected = "the report holds")]
    fn set_frame_rejects_an_oversized_payload() {
        set_frame(0x42, &[0u8; REPORT_LEN - HEADER_LEN + 1]);
    }

    #[test]
    fn get_frame_uses_the_query_direction() {
        assert_frame(&get_frame(op::BATTERY), &[0x06, 0xC0, 0x0A]);
    }

    #[test]
    fn reply_matches_only_its_own_opcode() {
        let r = reply(&[0x07, 0xC0, 0x77, 0x02, 0x00, 0x01]);
        assert!(reply_matches(&r, 0x77));
        assert!(!reply_matches(&r, 0x76));
    }

    #[test]
    fn parses_anc_modes() {
        assert_eq!(
            parse_anc(&reply(&[0x07, 0xC0, 0x77, 0x02, 0x00, 0x00, 0x03])),
            Anc::Off
        );
        assert_eq!(
            parse_anc(&reply(&[0x07, 0xC0, 0x77, 0x02, 0x00, 0x01, 0x03])),
            Anc::On
        );
        assert_eq!(
            parse_anc(&reply(&[0x07, 0xC0, 0x77, 0x02, 0x00, 0x02, 0x04])),
            Anc::Transparency(4)
        );
        assert_eq!(
            parse_anc(&reply(&[0x07, 0xC0, 0x77, 0x02, 0x00, 0x09])),
            Anc::Unknown(9)
        );
    }

    #[test]
    fn parses_battery_level() {
        assert_eq!(
            parse_battery(&reply(&[0x07, 0xC0, 0x0A, 0x0A, 0x00, 0x00, 77])),
            77
        );
    }

    #[test]
    fn wireless_link_decodes_the_two_values_the_headset_actually_sends() {
        // 0xCC and 0xDD are the only values seen across four cable
        // plug/unplug transitions, watching both hidraw nodes at once.
        assert_eq!(
            parse_wireless_link(&reply(&[0x07, 0xC0, 0x09, 0x01, 0x00, 0xCC])),
            WirelessLink::Up
        );
        assert_eq!(
            parse_wireless_link(&reply(&[0x07, 0xC0, 0x09, 0x01, 0x00, 0xDD])),
            WirelessLink::Down
        );
        assert_eq!(
            parse_wireless_link(&reply(&[0x07, 0xC0, 0x09, 0x01, 0x00, 0x42])),
            WirelessLink::Unknown(0x42)
        );
    }

    #[test]
    fn charging_is_read_from_the_battery_reply() {
        // Both byte strings are whole 0x0A payloads seen on this hardware: the
        // first captured from AWCC with the headset on battery, the second read
        // over the cable while charging. They differ in byte 7 and nowhere that
        // matters.
        let on_battery = reply(&[
            0x07, 0xC0, 0x0A, 0x0A, 0x00, 0x00, 0x40, 0x00, 0x00, 0x04, 0x0F, 0x40, 0x00, 0x02,
            0x03,
        ]);
        let charging = reply(&[
            0x07, 0xC0, 0x0A, 0x0A, 0x00, 0x00, 0x64, 0x01, 0x00, 0x36, 0x10, 0x64, 0x00, 0x02,
            0x2F,
        ]);
        assert!(!parse_charging(&on_battery));
        assert!(parse_charging(&charging));
        // The level must keep reading the same way out of both.
        assert_eq!(parse_battery(&on_battery), 64);
        assert_eq!(parse_battery(&charging), 100);
    }

    #[test]
    fn mic_mute_decoding_agrees_with_the_setter() {
        // commands::mic sends 0x00 to mute and 0x01 to unmute.
        assert!(parse_mic_muted(&reply(&[
            0x07, 0xC0, 0x76, 0x01, 0x00, 0x00
        ])));
        assert!(!parse_mic_muted(&reply(&[
            0x07, 0xC0, 0x76, 0x01, 0x00, 0x01
        ])));
    }

    #[test]
    fn parses_mic_noise_cancel() {
        assert!(parse_mic_noise_cancel(&reply(&[
            0x07, 0xC0, 0x80, 0x01, 0x00, 0x01
        ])));
        assert!(!parse_mic_noise_cancel(&reply(&[
            0x07, 0xC0, 0x80, 0x01, 0x00, 0x00
        ])));
    }

    #[test]
    fn sidetone_reads_the_last_byte_of_the_noise_cancel_payload() {
        // Bytes as captured over the cable with noise-cancel on: the reply
        // declares four payload bytes and the sidetone level is the fourth.
        assert_eq!(
            parse_sidetone(&reply(&[
                0x07, 0xC0, 0x80, 0x04, 0x00, 0x01, 0x00, 0x00, 0x04
            ])),
            4
        );
        assert_eq!(
            parse_sidetone(&reply(&[
                0x07, 0xC0, 0x80, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00
            ])),
            0
        );
    }

    #[test]
    fn parses_power_saving() {
        assert_eq!(
            parse_power_saving(&reply(&[0x07, 0xC0, 0x15, 0x02, 0x00, 0x01, 20])),
            PowerSaving {
                enabled: true,
                threshold: 20
            }
        );
    }

    #[test]
    fn auto_off_codes_cover_the_intervals_the_setter_sends() {
        for (code, minutes) in [(0x01, 15), (0x02, 30), (0x03, 45), (0x04, 60)] {
            assert_eq!(auto_off_minutes(code), minutes);
        }
    }
}
