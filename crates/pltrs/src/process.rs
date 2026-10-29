//! Starting external programs (ffmpeg, Ghostscript, browsers) robustly.
//!
//! Executing a file another thread has just written can fail with
//! "text file busy" (`ETXTBSY`): a child forked elsewhere in the program
//! may still hold the file's write descriptor for the moment between
//! its fork and its exec. The failure is transient, so starting a program
//! retries it a few times before giving up.

use std::io;
use std::process::{Child, Command, Output};
use std::thread;
use std::time::Duration;

/// How often a busy executable is retried, and how long to wait between
/// attempts: a fork-to-exec window lasts microseconds, so this is ample.
const BUSY_RETRIES: u32 = 50;
const BUSY_WAIT: Duration = Duration::from_millis(10);

/// Starts `command` ([`Command::spawn`]), retrying while its executable is
/// busy.
pub(crate) fn spawn(command: &mut Command) -> io::Result<Child> {
    retrying(|| command.spawn())
}

/// Runs `command` to completion ([`Command::output`]), retrying while its
/// executable is busy.
pub(crate) fn output(command: &mut Command) -> io::Result<Output> {
    retrying(|| command.output())
}

/// Calls `start` until it succeeds, fails for another reason, or the
/// executable stayed busy through every retry.
fn retrying<T>(mut start: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempt = 0;
    loop {
        match start() {
            Err(err)
                if err.kind() == io::ErrorKind::ExecutableFileBusy && attempt < BUSY_RETRIES =>
            {
                attempt += 1;
                thread::sleep(BUSY_WAIT);
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_executables_are_retried_and_other_errors_are_not() {
        let mut calls = 0;
        let result = retrying(|| {
            calls += 1;
            if calls < 3 {
                Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
            } else {
                Ok(calls)
            }
        });
        assert_eq!(result.unwrap(), 3);

        let mut calls = 0;
        let result: io::Result<()> = retrying(|| {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::NotFound))
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
        assert_eq!(calls, 1, "only a busy executable is retried");
    }

    #[test]
    fn a_program_that_stays_busy_fails_eventually() {
        let mut calls = 0;
        let result: io::Result<()> = retrying(|| {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
        });
        assert_eq!(
            result.unwrap_err().kind(),
            io::ErrorKind::ExecutableFileBusy
        );
        assert_eq!(calls, BUSY_RETRIES + 1);
    }
}
