//! Cold runs: whole programs in fresh processes, as a user waits for
//! them, timed from spawn to exit with their peak resident memory. The
//! harness's `cold` subcommand spawns them (Python's `os.wait4` reports
//! the memory, which Rust's standard library does not), in rounds in
//! which every command runs once, in an order rotated from round to
//! round, so that a change in the machine's load weighs on all of them
//! alike and the runs of a round can be paired.

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

/// What the cold runs of one command measured, run by run in round order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ColdResult {
    /// The seconds of each run, from spawn to exit.
    pub(super) seconds: Samples,
    /// The peak resident memory of each run, in KiB.
    pub(super) peaks_kib: Vec<u64>,
}

impl ColdResult {
    /// The median of the runs' peak resident memory, in KiB.
    pub(super) fn peak_kib(&self) -> u64 {
        let mut peaks = self.peaks_kib.clone();
        peaks.sort_unstable();
        peaks[peaks.len() / 2]
    }
}

/// Times `commands` in fresh processes, in rounds, as often as `sampling`
/// asks, through the harness run by `python`. A command's standard error
/// goes to `<logs>/<label>.stderr`, where a failure can be read.
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
        let [_round, label, seconds, peak, code] = fields[..] else {
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
            let (seconds, peaks_kib) = runs
                .remove(command.label.as_str())
                .ok_or_else(|| format!("bench.py cold did not run {}", command.label))?;
            Ok(ColdResult {
                seconds: Samples::new(seconds).ok_or("no cold runs")?,
                peaks_kib,
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
                    .map(pltrs_core::json::string)
                    .ok_or_else(|| format!("{} is not UTF-8", arg.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let env: Vec<String> = command
            .env
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}: {}",
                    pltrs_core::json::string(key),
                    pltrs_core::json::string(value)
                )
            })
            .collect();
        let stderr = logs.join(format!("{}.stderr", command.label));
        write!(
            json,
            "{{\"label\": {}, \"argv\": [{}], \"env\": {{{}}}, \"stderr\": {}}}",
            pltrs_core::json::string(&command.label),
            argv.join(", "),
            env.join(", "),
            pltrs_core::json::string(path_text(&stderr)?)
        )?;
    }
    json.push_str("]}");
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spec_lists_every_command() {
        let command = ColdCommand {
            label: "pltrs-rust".to_owned(),
            argv: vec!["/bin/true".into()],
            env: vec![("A".to_owned(), "1\"".to_owned())],
        };
        let sampling = Sampling {
            min_runs: 5,
            min_seconds: 2.0,
            max_runs: 20,
        };
        let logs = Path::new("/logs");
        let spec = spec_json(&[command], sampling, logs).expect("UTF-8 paths");
        // The log's path as the system joins it (`\logs\x` on Windows).
        let stderr = pltrs_core::json::string(path_text(&logs.join("pltrs-rust.stderr")).unwrap());
        assert_eq!(
            spec,
            format!(
                "{{\"min_runs\": 5, \"min_seconds\": 2, \"max_runs\": 20, \"commands\": [\
                 {{\"label\": \"pltrs-rust\", \"argv\": [\"/bin/true\"], \"env\": {{\"A\": \"1\\\"\"}}, \
                 \"stderr\": {stderr}}}]}}"
            )
        );
    }

    #[test]
    fn the_peak_is_the_median_run() {
        let result = ColdResult {
            seconds: Samples::new(vec![1.0, 2.0, 3.0]).expect("times"),
            peaks_kib: vec![30, 10, 20],
        };
        assert_eq!(result.peak_kib(), 20);
    }
}
