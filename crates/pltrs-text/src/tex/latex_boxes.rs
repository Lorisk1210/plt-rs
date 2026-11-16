//! Text measured by a running LaTeX (matplotlib's
//! `backend_pgf.LatexManager`): the PGF backend measures every text it
//! lays out by typesetting it in a box of the document its picture goes
//! into, and reading the box's width, height and depth back.
//!
//! The TeX system (`pgf.texsystem`) runs interactively in a temporary
//! directory. Given the document's header, it answers each box with a
//! `\typeout` line and prompts again; a text TeX cannot typeset stops it,
//! and the next text starts it again. Where matplotlib would wait for an
//! answer forever (a text reading from the terminal), pltrs stops TeX
//! after a while and reports what it printed.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Read as _, Write as _};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use super::error::TexError;
use super::tempdir::TemporaryDirectory;

/// What TeX prints once it has read the header (the header's last line
/// types it out).
const QUERY_START: &[u8] = b"*pgf_backend_query_start";

/// TeX's prompt for the next line.
const PROMPT: &[u8] = b"\n*";

/// How long TeX may take to read a header: loading its fonts, and for
/// LuaTeX building its font database the first time.
const HEADER_TIME: Duration = Duration::from_secs(600);

/// How long TeX may take to answer for one text.
const TEXT_TIME: Duration = Duration::from_secs(60);

/// The size of a box TeX typeset, in TeX points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxMetrics {
    /// The box's width.
    pub width: f64,
    /// Its height above the baseline.
    pub height: f64,
    /// Its depth below the baseline.
    pub depth: f64,
}

/// A running TeX system reading from a pipe, its output read as it comes
/// by a thread of its own.
#[derive(Debug)]
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    output: Receiver<Vec<u8>>,
    /// What TeX printed that no answer took yet.
    pending: Vec<u8>,
}

impl Session {
    /// Starts `program` interactively in `directory`, as matplotlib does:
    /// stopping at the first error, without shell escapes.
    fn start(program: &str, directory: &Path) -> Result<Self, TexError> {
        let unstartable = |err: &io::Error| TexError::Unstartable {
            program: program.to_owned(),
            missing: err.kind() == io::ErrorKind::NotFound,
        };
        let mut child = Command::new(program)
            .args(["-halt-on-error", "-no-shell-escape"])
            .current_dir(directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|err| unstartable(&err))?;
        let pipes = child.stdin.take().zip(child.stdout.take());
        let Some((stdin, mut stdout)) = pipes else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(unstartable(&io::Error::other("no pipes")));
        };
        let (sender, output) = channel();
        // Ends when TeX's output closes (TeX ended or was stopped).
        std::thread::spawn(move || {
            let mut chunk = [0; 4096];
            loop {
                match stdout.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(read) => {
                        if sender.send(chunk[..read].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            child,
            stdin: Some(stdin),
            output,
            pending: Vec::new(),
        })
    }

    /// Writes `line` and a line end.
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
    }

    /// What TeX prints up to and including `pattern`, within `time`; what
    /// it printed if it stops first or takes longer (then it is stopped).
    fn expect(&mut self, pattern: &[u8], time: Duration) -> Result<String, String> {
        let deadline = Instant::now() + time;
        loop {
            if let Some(at) = find(&self.pending, pattern) {
                let answer: Vec<u8> = self.pending.drain(..at + pattern.len()).collect();
                return Ok(text_of(&answer));
            }
            if let Err(stopped) = self.receive(deadline) {
                return Err(self.printed(stopped, time));
            }
        }
    }

    /// Waits until `deadline` for more of TeX's output.
    fn receive(&mut self, deadline: Instant) -> Result<(), RecvTimeoutError> {
        let left = deadline.saturating_duration_since(Instant::now());
        let chunk = self.output.recv_timeout(left)?;
        self.pending.extend(chunk);
        Ok(())
    }

    /// What TeX printed that no answer took, once its output `stopped`:
    /// it ended, or TeX took longer than `time` and is stopped.
    fn printed(&mut self, stopped: RecvTimeoutError, time: Duration) -> String {
        let mut printed = text_of(&self.pending);
        if stopped == RecvTimeoutError::Timeout {
            let _ = self.child.kill();
            let _ = write!(
                printed,
                "\n(no answer within {} s: stopped)",
                time.as_secs()
            );
        }
        printed
    }

    /// Ends the document right after its header (`\@@end`, then the end
    /// of its input): whether TeX read the header without an error, and
    /// what it printed.
    fn end(mut self) -> (bool, String) {
        let written = self.write_line("\n\\makeatletter\\@@end").is_ok();
        self.stdin = None;
        let deadline = Instant::now() + HEADER_TIME;
        let stopped = loop {
            if let Err(stopped) = self.receive(deadline) {
                break stopped;
            }
        };
        let printed = self.printed(stopped, HEADER_TIME);
        let success = written
            && stopped == RecvTimeoutError::Disconnected
            && self.child.wait().is_ok_and(|status| status.success());
        (success, printed)
    }
}

/// TeX's output as text, as matplotlib reads it from a pipe in Python's
/// text mode: its line ends, `\r\n` (TeX's on Windows) or `\r`, read as
/// `\n`.
fn text_of(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

/// Where `pattern` first occurs in `data`.
fn find(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len())
        .position(|window| window == pattern)
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A TeX system measuring texts in the document a header begins
/// (matplotlib's `LatexManager`), each text measured once.
#[derive(Debug)]
pub struct LatexBoxes {
    program: String,
    header: String,
    directory: TemporaryDirectory,
    session: Option<Session>,
    measured: HashMap<String, BoxMetrics>,
    /// How long TeX may take to answer for one text.
    text_time: Duration,
}

impl LatexBoxes {
    /// The TeX system `program` measuring texts after `header` (the
    /// document as far as its `\begin{document}`, then a line typing out
    /// `pgf_backend_query_start`), run in a fresh directory in `parent`.
    /// The header is tested first, as matplotlib tests it.
    ///
    /// # Errors
    ///
    /// When the directory cannot be made, the program cannot be started,
    /// or it stops on the header.
    pub fn new(program: &str, header: &str, parent: &Path) -> Result<Self, TexError> {
        let directory =
            TemporaryDirectory::new(parent).map_err(|err| TexError::io(parent, &err))?;
        let mut test = Session::start(program, directory.path())?;
        let written = test.write_line(header).is_ok();
        let (success, output) = test.end();
        if !(written && success) {
            return Err(TexError::Preamble {
                header: header.to_owned(),
                output,
            });
        }
        Ok(Self {
            program: program.to_owned(),
            header: header.to_owned(),
            directory,
            session: None,
            measured: HashMap::new(),
            text_time: TEXT_TIME,
        })
    }

    /// The header texts are measured after.
    pub fn header(&self) -> &str {
        &self.header
    }

    /// The width, height and depth of `tex` typeset in a box, `^` and `%`
    /// taken as active characters (matplotlib's `_get_box_metrics`).
    ///
    /// # Errors
    ///
    /// When TeX stops on the text ([`TexError::Measuring`], after which
    /// the next text starts it again), its answer cannot be read, or it
    /// cannot be started again.
    pub fn measure(&mut self, tex: &str) -> Result<BoxMetrics, TexError> {
        if let Some(&metrics) = self.measured.get(tex) {
            return Ok(metrics);
        }
        let session = match &mut self.session {
            Some(session) => session,
            None => self.session.insert(self.started()?),
        };
        // `\sbox` does not take catcode changes inside its argument, so
        // they are made outside it too.
        let query = format!(
            "{{\\catcode`\\^=\\active\\catcode`\\%=\\active\\sbox0{{{tex}}}\
             \\typeout{{\\the\\wd0,\\the\\ht0,\\the\\dp0}}}}"
        );
        let answer = session
            .write_line(&query)
            .map_err(|err| err.to_string())
            .and_then(|()| session.expect(PROMPT, self.text_time));
        let answer = match answer {
            Ok(answer) => answer,
            Err(output) => {
                self.session = None;
                return Err(TexError::Measuring {
                    tex: tex.to_owned(),
                    output,
                });
            }
        };
        let metrics = parse_answer(&answer).ok_or_else(|| TexError::Measuring {
            tex: tex.to_owned(),
            output: answer.clone(),
        })?;
        self.measured.insert(tex.to_owned(), metrics);
        Ok(metrics)
    }

    /// The TeX system, started and past the header.
    fn started(&self) -> Result<Session, TexError> {
        let mut session = Session::start(&self.program, self.directory.path())?;
        let halted = |output| TexError::Preamble {
            header: self.header.clone(),
            output,
        };
        session
            .write_line(&self.header)
            .map_err(|err| halted(err.to_string()))?;
        session.expect(QUERY_START, HEADER_TIME).map_err(halted)?;
        session.expect(PROMPT, HEADER_TIME).map_err(halted)?;
        Ok(session)
    }
}

/// The metrics of TeX's answer to a box: its third line from the end
/// (before the blank line `\typeout` ends with and the prompt) reads
/// `<width>pt,<height>pt,<depth>pt`.
fn parse_answer(answer: &str) -> Option<BoxMetrics> {
    let lines: Vec<&str> = answer.lines().collect();
    let line = lines.len().checked_sub(3).map(|at| lines[at])?;
    let mut values = line.split(',').map(|value| {
        value
            .trim_end()
            .strip_suffix("pt")
            .and_then(|number| number.parse::<f64>().ok())
    });
    let metrics = BoxMetrics {
        width: values.next()??,
        height: values.next()??,
        depth: values.next()??,
    };
    values.next().is_none().then_some(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_lines_end_as_python_reads_them() {
        assert_eq!(text_of(b"! Error.\r\nl.1\rx\n*"), "! Error.\nl.1\nx\n*");
    }

    #[test]
    fn answers_are_read_from_their_third_line_from_the_end() {
        let answer = "{\\catcode ...}\n12.5pt,6.83331pt,0.0pt\n\n*";
        assert_eq!(
            parse_answer(answer),
            Some(BoxMetrics {
                width: 12.5,
                height: 6.83331,
                depth: 0.0,
            })
        );
    }

    #[test]
    fn malformed_answers_are_refused() {
        for answer in [
            "",
            "*",
            "1pt,2pt\n\n*",
            "1pt,2pt,3pt,4pt\n\n*",
            "1pt,xpt,3pt\n\n*",
            "1,2,3\n\n*",
        ] {
            assert_eq!(parse_answer(answer), None, "{answer:?}");
        }
    }

    #[test]
    fn missing_programs_cannot_be_started() {
        let parent = std::env::temp_dir();
        let err = LatexBoxes::new("pltrs-no-such-tex", "", &parent).expect_err("missing");
        assert_eq!(
            err,
            TexError::Unstartable {
                program: "pltrs-no-such-tex".to_owned(),
                missing: true,
            }
        );
    }

    /// The header matplotlib's PGF backend measures texts after, for the
    /// default preamble of `program`.
    fn header(program: &str) -> String {
        format!(
            "\\documentclass{{article}}\n% !TeX program = {program}\n\\usepackage{{graphicx}}\n\
             \\begin{{document}}\n\\typeout{{pgf_backend_query_start}}"
        )
    }

    #[test]
    fn latex_measures_boxes_and_starts_again_after_an_error() {
        if pltrs_core::os::which("pdflatex").is_none() {
            return;
        }
        let parent = std::env::temp_dir();
        let mut boxes = LatexBoxes::new("pdflatex", &header("pdflatex"), &parent).expect("TeX");
        let rule = boxes.measure(r"\rule{10pt}{5pt}").expect("measured");
        assert_eq!(
            rule,
            BoxMetrics {
                width: 10.0,
                height: 5.0,
                depth: 0.0,
            }
        );
        let err = boxes.measure(r"\oops").expect_err("undefined");
        assert!(matches!(err, TexError::Measuring { .. }), "{err:?}");
        let lowered = boxes
            .measure(r"\raisebox{-2pt}{\rule{3pt}{4pt}}")
            .expect("again");
        assert_eq!((lowered.height, lowered.depth), (2.0, 2.0));
    }

    #[test]
    fn texts_waiting_for_the_terminal_are_stopped() {
        if pltrs_core::os::which("pdflatex").is_none() {
            return;
        }
        let parent = std::env::temp_dir();
        let mut boxes = LatexBoxes::new("pdflatex", &header("pdflatex"), &parent).expect("TeX");
        boxes.text_time = Duration::from_secs(1);
        let err = boxes.measure(r"\read16 to\x").expect_err("waits");
        let TexError::Measuring { output, .. } = err else {
            panic!("{err:?}");
        };
        assert!(
            output.ends_with("(no answer within 1 s: stopped)"),
            "{output}"
        );
        let rule = boxes.measure(r"\rule{1pt}{1pt}").expect("started again");
        assert_eq!(rule.width, 1.0);
    }

    #[test]
    fn broken_headers_are_refused() {
        if pltrs_core::os::which("pdflatex").is_none() {
            return;
        }
        let parent = std::env::temp_dir();
        let broken = header("pdflatex").replace("graphicx", "pltrs-no-such-package");
        let err = LatexBoxes::new("pdflatex", &broken, &parent).expect_err("no package");
        assert!(matches!(err, TexError::Preamble { .. }), "{err:?}");
    }
}
