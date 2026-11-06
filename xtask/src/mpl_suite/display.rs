//! The X display a suite run starts for itself: an Xvfb on the first free
//! display number, stopped by its process id when the run ends.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// The virtual screen: as wide and deep as the toolkits' own instructions
/// make it (`python/tools/toolkits.sh`, `crates/pltrs-platform`).
const SCREEN: &str = "1280x800x24";

/// An Xvfb the runner started, stopped (SIGTERM to its process id, then
/// waited for) when dropped.
#[derive(Debug)]
pub(crate) struct VirtualDisplay {
    /// The server.
    server: Child,
    /// Its display name (`:N`).
    name: String,
}

impl VirtualDisplay {
    /// Starts `xvfb` on the first free display number, which the server
    /// picks and writes to its standard output (`-displayfd 1`) once it
    /// accepts connections; its messages go to `log`.
    ///
    /// The server does not reset when its last client leaves
    /// (`-noreset`): it refuses connections while it resets, which the
    /// pytest workers' display checks, opening and closing connections
    /// at once, would run into.
    pub(crate) fn start(xvfb: &Path, log: &Path) -> Result<Self, String> {
        let messages = File::create(log).map_err(|err| format!("{}: {err}", log.display()))?;
        let mut server = Command::new(xvfb)
            .args(["-displayfd", "1", "-noreset", "-nolisten", "tcp"])
            .args(["-screen", "0", SCREEN])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(messages)
            .spawn()
            .map_err(|err| format!("cannot run {}: {err}", xvfb.display()))?;
        let mut line = String::new();
        let read = server
            .stdout
            .take()
            .map(|out| BufReader::new(out).read_line(&mut line));
        if let (Some(Ok(_)), Some(name)) = (read, display_name(&line)) {
            return Ok(Self { server, name });
        }
        // The server exited (or wrote nonsense): reap it.
        let _ = server.kill();
        let status = server
            .wait()
            .map_or_else(|err| err.to_string(), |s| s.to_string());
        Err(format!(
            "{} did not start ({status}); see {}",
            xvfb.display(),
            log.display()
        ))
    }

    /// The display's name, `DISPLAY`'s value (`:N`).
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The server's process id.
    pub(crate) fn id(&self) -> u32 {
        self.server.id()
    }
}

impl Drop for VirtualDisplay {
    fn drop(&mut self) {
        // SIGTERM lets the server remove its lock file and socket; only
        // if `kill` cannot be run does it get std's SIGKILL.
        let terminated = Command::new("kill")
            .args(["-TERM", &self.server.id().to_string()])
            .status()
            .is_ok_and(|status| status.success());
        if !terminated {
            let _ = self.server.kill();
        }
        if let Err(err) = self.server.wait() {
            eprintln!("cannot stop Xvfb {}: {err}", self.name);
        }
    }
}

/// The display name of the number `-displayfd` wrote (`"3\n"` → `":3"`).
fn display_name(written: &str) -> Option<String> {
    let number: u32 = written.trim().parse().ok()?;
    Some(format!(":{number}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_numbers_become_display_names() {
        assert_eq!(display_name("0\n").as_deref(), Some(":0"));
        assert_eq!(display_name("117\n").as_deref(), Some(":117"));
        assert_eq!(display_name(""), None);
        assert_eq!(display_name("x\n"), None);
        assert_eq!(display_name("-1\n"), None);
    }

    /// With Xvfb on `PATH`, a display starts, takes connections, and is
    /// gone once dropped; without it the test passes trivially.
    #[test]
    fn virtual_displays_start_and_stop() {
        let Some(xvfb) = pltrs_core::os::which("Xvfb") else {
            return;
        };
        let dir = crate::python::root().join("target/xtask-tests");
        std::fs::create_dir_all(&dir).expect("scratch directory");
        let display = VirtualDisplay::start(&xvfb, &dir.join("xvfb.log")).expect("Xvfb starts");
        let number = display.name().trim_start_matches(':').to_owned();
        let socket = Path::new("/tmp/.X11-unix").join(format!("X{number}"));
        assert!(socket.exists(), "{} is missing", socket.display());
        let id = display.id();
        drop(display);
        assert!(!Path::new(&format!("/proc/{id}")).exists());
        assert!(!socket.exists(), "{} is left behind", socket.display());
    }
}
