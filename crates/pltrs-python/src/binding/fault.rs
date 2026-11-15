//! The address a fatal fault touched, for the reports of crashes that do
//! not keep a core file (pltrs (Python)'s fuzzer): Python's `faulthandler`
//! prints where Python and the native code were, not which address was
//! touched, and only that tells a stack overflow (an address just below
//! the thread's stack) from a stray pointer.
//!
//! When `PLTRS_REPORT_FAULTS` is set as the extension is imported (on
//! Linux), a handler of `SIGSEGV` and `SIGBUS` writes one line to stderr,
//! `pltrs: fatal signal 11 at address 0x7ffd5e3fefe8`, then puts back
//! the handler it replaced and returns: the fault happens again, and that
//! handler (`faulthandler`'s, when Python enabled it first) reports it as
//! before. On a thread without an alternate signal stack (`faulthandler`
//! gives one to the thread enabling it) a fault of a full stack leaves no
//! room for any handler, and the process ends as it would have.

use std::ffi::{c_int, c_void};
use std::fmt::Write as _;
use std::sync::OnceLock;

use crate::ffi;

/// The variable asking for the report.
const ASKED_BY: &str = "PLTRS_REPORT_FAULTS";

/// The signals reported, and the action each replaced.
static REPLACED: [(c_int, OnceLock<ffi::sigaction_t>); 2] = [
    (ffi::SIGSEGV, OnceLock::new()),
    (ffi::SIGBUS, OnceLock::new()),
];

/// Installs the report when [`ASKED_BY`] is set.
pub(super) fn install_if_asked() {
    if std::env::var_os(ASKED_BY).is_none() {
        return;
    }
    let handler: ffi::sa_sigaction_t = report;
    let action = ffi::sigaction_t {
        sa_sigaction: handler as usize,
        sa_mask: [0; 16],
        sa_flags: ffi::SA_SIGINFO | ffi::SA_ONSTACK,
        sa_restorer: 0,
    };
    for (signal, replaced) in &REPLACED {
        let mut previous = action;
        // SAFETY: both pointers are to live `sigaction_t`s, laid out as
        // the C library's; `report` is async-signal-safe.
        if unsafe { ffi::sigaction(*signal, &raw const action, &raw mut previous) } == 0 {
            replaced.get_or_init(|| previous);
        }
    }
}

/// The handler: writes the line, puts back the action it replaced.
unsafe extern "C" fn report(signal: c_int, info: *mut c_void, _context: *mut c_void) {
    // SAFETY: with `SA_SIGINFO`, the kernel passes the signal's
    // `siginfo_t` (aligned as an address), whose `si_addr` is there.
    let address = unsafe { info.cast::<usize>().add(ffi::SI_ADDR_INDEX).read() };
    let mut line = Line::default();
    // Never fails: `Line` keeps what fits.
    let _ = writeln!(line, "pltrs: fatal signal {signal} at address {address:#x}");
    // SAFETY: `write` is async-signal-safe; the bytes are `line`'s.
    unsafe { ffi::write(2, line.bytes.as_ptr().cast(), line.len) };
    if let Some((_, replaced)) = REPLACED.iter().find(|(number, _)| *number == signal)
        && let Some(previous) = replaced.get()
    {
        // SAFETY: `previous` is the action `sigaction` handed back.
        unsafe { ffi::sigaction(signal, previous, std::ptr::null_mut()) };
    }
}

/// A line written without allocating (a signal handler may not).
struct Line {
    bytes: [u8; 96],
    len: usize,
}

impl Default for Line {
    fn default() -> Self {
        Self {
            bytes: [0; 96],
            len: 0,
        }
    }
}

impl std::fmt::Write for Line {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let room = &mut self.bytes[self.len..];
        let taken = text.len().min(room.len());
        room[..taken].copy_from_slice(&text.as_bytes()[..taken]);
        self.len += taken;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_keeps_what_fits() {
        let mut line = Line::default();
        let _ = writeln!(
            line,
            "pltrs: fatal signal 11 at address {:#x}",
            0x7ffd_5e3f_efe8_usize
        );
        assert_eq!(
            &line.bytes[..line.len],
            b"pltrs: fatal signal 11 at address 0x7ffd5e3fefe8\n"
        );
        let _ = write!(line, "{}", "x".repeat(200));
        assert_eq!(line.len, 96);
    }
}
