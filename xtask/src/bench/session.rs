//! Running implementations of a workload, ready for timed runs and saves:
//! pltrs (Rust) in this process, and the Python columns in processes of
//! the harness's `serve` mode, which keep the script and its data loaded
//! between requests. The harness takes turns among them (`rounds`).

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};

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

/// A Python column serving a script: `bench.py serve`.
pub(super) struct PythonSession {
    /// Which column it is.
    implementation: Implementation,
    /// The process.
    child: Child,
    /// Where requests go.
    requests: ChildStdin,
    /// Where replies come from.
    replies: BufReader<ChildStdout>,
    /// Where the process writes its standard error.
    log: PathBuf,
}

impl PythonSession {
    /// Starts `runner` serving `script`, the script of `workload`, its
    /// standard error going to `log`; `Unsupported` when the script cannot
    /// be loaded.
    pub(super) fn start(
        runner: &PythonRunner,
        workload: &Workload,
        script: &str,
        log: &Path,
    ) -> Result<Outcome<Self>, Box<dyn Error>> {
        let mut command = runner.serve_command(workload, script);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(File::create(log)?)
            .spawn()?;
        let requests = child.stdin.take().ok_or("no pipe to bench.py")?;
        let replies = BufReader::new(child.stdout.take().ok_or("no pipe from bench.py")?);
        let mut session = Self {
            implementation: runner.implementation,
            child,
            requests,
            replies,
            log: log.to_owned(),
        };
        Ok(match session.reply()? {
            Outcome::Done(line) if line == "ready" => Outcome::Done(session),
            Outcome::Done(line) => {
                return Err(format!("unexpected reply from bench.py: {line}").into());
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
            return Err(format!(
                "bench.py ({}) exited: {}",
                self.implementation.title(),
                stderr.trim()
            )
            .into());
        }
        let line = line.trim_end();
        Ok(match line.strip_prefix("unsupported\t") {
            Some(why) => Outcome::Unsupported(why.to_owned()),
            None => match line.strip_prefix("error\t") {
                Some(why) => return Err(format!("bench.py: {why}").into()),
                None => Outcome::Done(line.to_owned()),
            },
        })
    }
}

impl Session for PythonSession {
    fn implementation(&self) -> Implementation {
        self.implementation
    }

    fn run(&mut self) -> Result<Outcome<Run>, Box<dyn Error>> {
        Ok(match self.request("run")? {
            Outcome::Done(line) => {
                let fields: Vec<&str> = line.split('\t').collect();
                let ["time", seconds, bytes] = fields[..] else {
                    return Err(format!("unexpected reply from bench.py: {line}").into());
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

impl Drop for PythonSession {
    fn drop(&mut self) {
        // The process ends on `quit`, or on the end of its input should
        // the request fail.
        let _ = writeln!(self.requests, "quit");
        let _ = self.requests.flush();
        let _ = self.child.wait();
    }
}
