use serde_json::{Value, json};

use crate::error::AppError;
use crate::protocol::{Anc, WirelessLink};

/// Whether the process on the other end of `fd` has gone away, asked without
/// writing anything.
///
/// `| head -1` leaves no failed write to learn from: the line it wanted fitted
/// in the pipe buffer and succeeded, and only the *next* write would fail. A
/// command that then blocks waiting for the next event would sit there holding
/// a pipe nobody is reading — which is what `awpro watch | head -1` used to do.
/// `poll` reports `POLLERR` on the write end once the read end closes, so the
/// question can be asked directly.
pub fn reader_gone_fd(fd: std::os::fd::RawFd) -> bool {
    let mut pfd = libc::pollfd {
        fd,
        events: 0,
        revents: 0,
    };
    // Timeout 0: ask and return, never wait.
    let ready = unsafe { libc::poll(&mut pfd, 1, 0) };
    ready > 0 && pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0
}

/// Whether whoever was reading our stdout has gone away.
pub fn reader_gone() -> bool {
    reader_gone_fd(libc::STDOUT_FILENO)
}

/// Whether a write failed because the reader closed the pipe.
///
/// `head`, `grep -q` and friends do this the moment they have what they came
/// for. It is the normal way to stop a streaming command, not a failure, and
/// treating it as one is why `awpro watch | head -1` used to end in a panic
/// instead of a line of output.
pub fn reader_went_away(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::BrokenPipe
}

/// Write one line to stdout, surfacing the error rather than panicking.
///
/// `println!` panics on a broken pipe; every caller here wants to decide for
/// itself instead.
pub fn write_line(line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "{line}")
}

/// Print either a plain string or a JSON object depending on the `json` flag.
///
/// A reader that has gone away is ignored: this is the last thing a one-shot
/// command does, so there is nothing left to stop.
pub fn print(plain: &str, json_value: Value, json: bool) {
    let line = if json {
        json_value.to_string()
    } else {
        plain.to_string()
    };
    print_line(&line);
}

/// Write a line for a command that has nothing left to do afterwards.
///
/// A reader that has gone away is ignored: there is nothing left to stop. Any
/// other failure is worth a word on stderr, but not worth changing the exit
/// code of a command whose real work already succeeded.
fn print_line(line: &str) {
    if let Err(e) = write_line(line)
        && !reader_went_away(&e)
    {
        eprintln!("awpro: failed to write output: {e}");
    }
}

/// Report the outcome of a command that sets something.
///
/// Plain mode stays silent on success — a command that worked printing nothing
/// is the useful shell behaviour, and it is what this tool has always done.
/// `--json` gets one object describing the new state, so the globally-accepted
/// flag is no longer inert on every setter.
///
/// `acknowledged` is false when the headset never answered the write. That is
/// reported, not treated as a failure: whether every command is acknowledged
/// has not been established on hardware, so failing here could break commands
/// that work today.
pub fn report_set(state: Value, acknowledged: bool, json: bool) {
    if json {
        let mut value = state;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("ok".into(), Value::Bool(true));
            obj.insert("acknowledged".into(), Value::Bool(acknowledged));
        }
        print_line(&value.to_string());
    } else if !acknowledged {
        eprintln!("warning: the headset did not acknowledge the command (is it on and in range?)");
    }
}

/// Report a failure: always a human line on stderr, plus a machine-readable
/// object on stdout under `--json` so a consumer parsing stdout sees the error
/// instead of empty output.
pub fn report_error(err: &AppError, json: bool) {
    eprintln!("error: {err}");
    if json {
        print_line(
            &json!({ "error": { "kind": err.kind(), "message": err.to_string() } }).to_string(),
        );
    }
}

// ── Field rendering ───────────────────────────────────────────────────────────
//
// ANC is the one field with enough shape to be worth rendering in one place:
// `get anc` and `status` both have to spell out four variants, and a fifth
// spelling of any of them would be a silent inconsistency.

/// ANC state as a human-readable phrase.
pub fn anc_plain(anc: &Anc) -> String {
    match anc {
        Anc::Off => "off".to_string(),
        Anc::On => "on".to_string(),
        Anc::Transparency(level) => format!("transparency (level {level})"),
        Anc::Unknown(code) => format!("unknown (0x{code:02x})"),
    }
}

/// ANC state as its `--json` object.
///
/// `mode` stays a fixed token so a consumer can match on it; an unrecognised
/// byte goes in its own `raw` field rather than inside the token.
pub fn anc_json(anc: &Anc) -> Value {
    match anc {
        Anc::Off => json!({ "mode": "off" }),
        Anc::On => json!({ "mode": "on" }),
        Anc::Transparency(level) => json!({ "mode": "transparency", "level": level }),
        Anc::Unknown(code) => json!({ "mode": "unknown", "raw": code }),
    }
}

/// `up` / `down`, spelled the same by `status` and `info`.
pub fn wireless_link_plain(link: &WirelessLink) -> String {
    match link {
        WirelessLink::Up => "up".to_string(),
        WirelessLink::Down => "down".to_string(),
        WirelessLink::Unknown(code) => format!("unknown (0x{code:02x})"),
    }
}

pub fn wireless_link_json(link: &WirelessLink) -> Value {
    match link {
        WirelessLink::Up => json!({ "up": true }),
        WirelessLink::Down => json!({ "up": false }),
        WirelessLink::Unknown(code) => json!({ "up": null, "code": code }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn a_closed_reader_is_not_a_failure() {
        // `awpro watch | head -1` closes the pipe as soon as it has its line.
        // That is the reader saying "enough", not an error, and it is how a
        // streaming command is supposed to be stopped.
        assert!(reader_went_away(&Error::new(ErrorKind::BrokenPipe, "x")));
    }

    #[test]
    fn a_dropped_reader_is_noticed_without_writing_anything() {
        // This is what `| head -1` leaves behind: the line already went into
        // the pipe buffer and succeeded, so there is no failed write to learn
        // from. A streaming command has to be able to ask.
        use std::os::fd::AsRawFd;
        let (reader, writer) = std::io::pipe().expect("pipe");
        assert!(
            !reader_gone_fd(writer.as_raw_fd()),
            "reader is still open, nothing should be reported"
        );
        drop(reader);
        assert!(
            reader_gone_fd(writer.as_raw_fd()),
            "reader is gone and no write happened, so polling must say so"
        );
    }

    #[test]
    fn a_real_write_failure_is_still_a_failure() {
        // A full disk or a revoked descriptor must not be mistaken for a
        // reader that simply finished.
        assert!(!reader_went_away(&Error::new(ErrorKind::StorageFull, "x")));
        assert!(!reader_went_away(&Error::new(
            ErrorKind::PermissionDenied,
            "x"
        )));
    }
}
