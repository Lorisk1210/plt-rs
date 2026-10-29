//! Cold runs: whole programs in fresh processes, as a user waits for
//! them, timed from spawn to exit with their peak resident memory. The
//! harness's `cold` subcommand spawns them (Python's `os.wait4` reports
//! the memory, which Rust's standard library does not), the commands of a
//! workload taking turns so that a change in the machine's load weighs on
//! all of them alike.

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use super::python::{PYTHON_ENV, path_text};
use super::stats::{Samples, Sampling};

/// A program to time in fresh processes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ColdCommand {
    /// Its column, which names it in the harness's replies.
    pub(super) label: String,
    /// The program (an absolute path) and its arguments.
    pub(super) argv: Vec<OsString>,
    /// Environment on top of the benchmark's.
    pub(super) env: Vec<(String, String)>,
}

/// What the cold runs of one command measured.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ColdResult {
    /// The seconds of each run, from spawn to exit.
    pub(super) seconds: Samples,
    /// The median of the runs' peak resident memory, in KiB.
    pub(super) peak_kib: u64,
}

/// Times `commands` in fresh processes, taking turns, each run as often as
/// `sampling` asks, through the harness run by `python`. A command's
/// standard error goes to `<logs>/<label>.stderr`, where a failure can be
/// read.
pub(super) fn time_cold(
    python: &Path,
    commands: &[ColdCommand],
    sampling: Sampling,
    logs: &Path,
) -> Result<Vec<ColdResult>, Box<dyn Error>> {
    let mut labels: Vec<&str> = commands.iter().map(|c| c.label.as_str()).collect();
    labels.sort_unstable();
    if let Some([label, _]) = labels.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(format!("two cold commands are labelled {label}").into());
    }
    std::fs::create_dir_all(logs)?;
    let spec = spec_json(commands, sampling, logs)?;
    let output = Command::new(python)
        .arg(crate::python::root().join("spec/reference/bench/bench.py"))
        .arg("cold")
        .arg(spec)
        .envs(PYTHON_ENV)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "bench.py cold failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let mut runs: BTreeMap<&str, (Vec<f64>, Vec<u64>)> = BTreeMap::new();
    for line in std::str::from_utf8(&output.stdout)?.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        let [label, seconds, peak, code] = fields[..] else {
            return Err(format!("unexpected reply from bench.py cold: {line}").into());
        };
        if code != "0" {
            let log = logs.join(format!("{label}.stderr"));
            let stderr = std::fs::read_to_string(&log).unwrap_or_default();
            return Err(format!("{label} exited with code {code}: {}", stderr.trim()).into());
        }
        let entry = runs.entry(label).or_default();
        entry.0.push(seconds.parse()?);
        entry.1.push(peak.parse()?);
    }
    commands
        .iter()
        .map(|command| {
            let (seconds, mut peaks) = runs
                .remove(command.label.as_str())
                .ok_or_else(|| format!("bench.py cold did not run {}", command.label))?;
            peaks.sort_unstable();
            Ok(ColdResult {
                seconds: Samples::new(seconds).ok_or("no cold runs")?,
                peak_kib: peaks[peaks.len() / 2],
            })
        })
        .collect()
}

/// The harness's description of the cold runs, as JSON.
fn spec_json(
    commands: &[ColdCommand],
    sampling: Sampling,
    logs: &Path,
) -> Result<String, Box<dyn Error>> {
    let mut json = format!(
        "{{\"min_runs\": {}, \"min_seconds\": {}, \"max_runs\": {}, \"commands\": [",
        sampling.min_runs, sampling.min_seconds, sampling.max_runs
    );
    for (i, command) in commands.iter().enumerate() {
        if i > 0 {
            json.push_str(", ");
        }
        let argv = command
            .argv
            .iter()
            .map(|arg| {
                arg.to_str()
                    .map(json_string)
                    .ok_or_else(|| format!("{} is not UTF-8", arg.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let env: Vec<String> = command
            .env
            .iter()
            .map(|(key, value)| format!("{}: {}", json_string(key), json_string(value)))
            .collect();
        let stderr = logs.join(format!("{}.stderr", command.label));
        write!(
            json,
            "{{\"label\": {}, \"argv\": [{}], \"env\": {{{}}}, \"stderr\": {}}}",
            json_string(&command.label),
            argv.join(", "),
            env.join(", "),
            json_string(path_text(&stderr)?)
        )?;
    }
    json.push_str("]}");
    Ok(json)
}

/// `text` as a JSON string literal.
fn json_string(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for c in text.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(quoted, "\\u{:04x}", u32::from(c));
            }
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json_string("a\"b\\c\nd\u{1}"), r#""a\"b\\c\nd\u0001""#);
    }

    #[test]
    fn the_spec_lists_every_command() {
        let command = ColdCommand {
            label: "pltrs-rust".to_owned(),
            argv: vec!["/bin/true".into()],
            env: vec![("A".to_owned(), "1".to_owned())],
        };
        let sampling = Sampling {
            min_runs: 5,
            min_seconds: 2.0,
            max_runs: 20,
        };
        let spec = spec_json(&[command], sampling, Path::new("/logs")).expect("UTF-8 paths");
        assert_eq!(
            spec,
            "{\"min_runs\": 5, \"min_seconds\": 2, \"max_runs\": 20, \"commands\": [\
             {\"label\": \"pltrs-rust\", \"argv\": [\"/bin/true\"], \"env\": {\"A\": \"1\"}, \
             \"stderr\": \"/logs/pltrs-rust.stderr\"}]}"
        );
    }
}
