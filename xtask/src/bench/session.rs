//! Running implementations of a workload, ready for timed runs and saves:
//! pltrs (Rust) in this process, and the Python columns in processes of
//! the harness's `serve` mode, which keep the script and its data loaded
//! between requests (as `--child serve` does for pltrs (Rust) of another
//! build). The harness takes turns among them (`rounds`).

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use super::implementation::Implementation;
use super::python::PythonRunner;
use super::render::{Documents, render, timed_run};
use super::workloads::{Built, Output, Workload};

/// What an implementation made of a request.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Outcome<T> {
    /// It did it.
    Done(T),
    /// It raised this exception: pltrs (Python) lacks something the script
    /// uses.
    Unsupported(String),
}

/// A timed run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Run {
    /// The seconds the figure took to build and save.
    pub(super) seconds: f64,
    /// The size of the document saved, in bytes.
    pub(super) bytes: u64,
}

/// An implementation with a workload loaded.
pub(super) trait Session {
    /// Which implementation it is.
    fn implementation(&self) -> Implementation;

    /// Makes one timed run.
    fn run(&mut self) -> Result<Outcome<Run>, Box<dyn Error>>;

    /// Saves the figure to `path` as `output` asks.
    fn save(&mut self, output: Output, path: &Path) -> Result<Outcome<()>, Box<dyn Error>>;
}

/// pltrs (Rust), with the workload's data made.
pub(super) struct RustSession {
    /// The workload.
    workload: &'static Workload,
    /// What builds its figure from its data.
    built: Built,
}

impl RustSession {
    /// `workload` with its data made.
    pub(super) fn new(workload: &'static Workload) -> Self {
        Self {
            workload,
            built: workload.build(),
        }
    }
}

impl Session for RustSession {
    fn implementation(&self) -> Implementation {
        Implementation::Rust
    }

    fn run(&mut self) -> Result<Outcome<Run>, Box<dyn Error>> {
        let timed = timed_run(self.workload, &self.built)?;
        Ok(Outcome::Done(Run {
            seconds: timed.seconds,
            bytes: timed.document.len() as u64,
        }))
    }

    fn save(&mut self, output: Output, path: &Path) -> Result<Outcome<()>, Box<dyn Error>> {
        let timed = render(self.workload, &self.built, output, Documents::AsUsers)?;
        std::fs::write(path, timed.document)?;
        Ok(Outcome::Done(()))
    }
}

/// An implementation serving a workload from a process of its own, which
/// keeps the workload's data loaded between requests: a Python column in
/// the harness's `serve` mode, or pltrs (Rust) in an executable's
/// (`--child serve`).
pub(super) struct ServedSession {
    /// Which implementation it is.
    implementation: Implementation,
    /// What names it in errors.
    title: String,
    /// The process.
    child: Child,
    /// Where requests go.
    requests: ChildStdin,
    /// Where replies come from.
    replies: BufReader<ChildStdout>,
    /// Where the process writes its standard error.
    log: PathBuf,
}

impl ServedSession {
    /// Starts `runner` serving `script`, the script of `workload`, its
    /// standard error going to `log`; `Unsupported` when the script cannot
    /// be loaded.
    pub(super) fn start(
        runner: &PythonRunner,
        workload: &Workload,
        script: &str,
        log: &Path,
    ) -> Result<Outcome<Self>, Box<dyn Error>> {
        Self::spawn(
            runner.implementation,
            format!("bench.py ({})", runner.implementation.title()),
            runner.serve_command(workload, script),
            log,
        )
    }

    /// Starts `command`, a server of `implementation` named `title`, its
    /// standard error going to `log`, and waits until it is ready.
    pub(super) fn spawn(
        implementation: Implementation,
        title: String,
        mut command: Command,
        log: &Path,
    ) -> Result<Outcome<Self>, Box<dyn Error>> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(File::create(log)?)
            .spawn()?;
        let requests = child.stdin.take().ok_or("no pipe to the server")?;
        let replies = BufReader::new(child.stdout.take().ok_or("no pipe from the server")?);
        let mut session = Self {
            implementation,
            title,
            child,
            requests,
            replies,
            log: log.to_owned(),
        };
        Ok(match session.reply()? {
            Outcome::Done(line) if line == READY => Outcome::Done(session),
            Outcome::Done(line) => {
                return Err(format!("unexpected reply from {}: {line}", session.title).into());
            }
            Outcome::Unsupported(why) => Outcome::Unsupported(why),
        })
    }

    /// Sends `request` and reads the reply.
    fn request(&mut self, request: &str) -> Result<Outcome<String>, Box<dyn Error>> {
        writeln!(self.requests, "{request}")?;
        self.requests.flush()?;
        self.reply()
    }

    /// Reads one reply.
    fn reply(&mut self) -> Result<Outcome<String>, Box<dyn Error>> {
        let mut line = String::new();
        if self.replies.read_line(&mut line)? == 0 {
            let stderr = std::fs::read_to_string(&self.log).unwrap_or_default();
            return Err(format!("{} exited: {}", self.title, stderr.trim()).into());
        }
        let line = line.trim_end();
        Ok(match line.strip_prefix("unsupported\t") {
            Some(why) => Outcome::Unsupported(why.to_owned()),
            None => match line.strip_prefix("error\t") {
                Some(why) => return Err(format!("{}: {why}", self.title).into()),
                None => Outcome::Done(line.to_owned()),
            },
        })
    }
}

/// The reply of a server that has loaded its workload.
pub(super) const READY: &str = "ready";

/// The reply to `run`: `time`, the seconds and the document's size.
pub(super) fn time_reply(run: Run) -> String {
    format!("time\t{}\t{}", run.seconds, run.bytes)
}

impl Session for ServedSession {
    fn implementation(&self) -> Implementation {
        self.implementation
    }

    fn run(&mut self) -> Result<Outcome<Run>, Box<dyn Error>> {
        Ok(match self.request("run")? {
            Outcome::Done(line) => {
                let fields: Vec<&str> = line.split('\t').collect();
                let ["time", seconds, bytes] = fields[..] else {
                    return Err(format!("unexpected reply from {}: {line}", self.title).into());
                };
                Outcome::Done(Run {
                    seconds: seconds.parse()?,
                    bytes: bytes.parse()?,
                })
            }
            Outcome::Unsupported(why) => Outcome::Unsupported(why),
        })
    }

    fn save(&mut self, output: Output, path: &Path) -> Result<Outcome<()>, Box<dyn Error>> {
        let path = path.to_str().ok_or("artifact paths are UTF-8")?;
        Ok(
            match self.request(&format!("save\t{}\t{path}", output.name()))? {
                Outcome::Done(_) => Outcome::Done(()),
                Outcome::Unsupported(why) => Outcome::Unsupported(why),
            },
        )
    }
}

impl Drop for ServedSession {
    fn drop(&mut self) {
        // The process ends on `quit`, or on the end of its input should
        // the request fail.
        let _ = writeln!(self.requests, "quit");
        let _ = self.requests.flush();
        let _ = self.child.wait();
    }
}
