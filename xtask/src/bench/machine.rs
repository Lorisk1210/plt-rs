//! What the benchmark ran on: the machine, its load, and the versions of
//! the tools, read from `/proc`, `/sys` and the tools themselves.

use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

/// The machine and the software around the timings.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Machine {
    /// The CPU model.
    pub(super) cpu: String,
    /// Physical cores.
    pub(super) cores: usize,
    /// Hardware threads.
    pub(super) threads: usize,
    /// Memory, in GiB.
    pub(super) memory_gib: f64,
    /// The operating system's name.
    pub(super) os: String,
    /// The kernel's release.
    pub(super) kernel: String,
    /// The CPUs this process may run on.
    pub(super) affinity: String,
    /// The first CPU's frequency governor, where the kernel reports one.
    pub(super) governor: Option<String>,
    /// The Rust compiler's version.
    pub(super) rustc: String,
    /// The commit benchmarked, `-dirty` when the tree has changes.
    pub(super) revision: String,
}

impl Machine {
    /// Describes this machine. What cannot be read is `unknown`.
    pub(super) fn describe() -> Self {
        let cpuinfo = read("/proc/cpuinfo");
        let field = |line: &str, name: &str| {
            line.split_once(':')
                .filter(|(key, _)| key.trim() == name)
                .map(|(_, value)| value.trim().to_owned())
        };
        let cpu = cpuinfo
            .lines()
            .find_map(|line| field(line, "model name"))
            .unwrap_or_else(unknown);
        let threads = cpuinfo
            .lines()
            .filter(|line| field(line, "processor").is_some())
            .count();
        let mut physical = BTreeSet::new();
        let mut package = String::new();
        for line in cpuinfo.lines() {
            if let Some(id) = field(line, "physical id") {
                package = id;
            } else if let Some(core) = field(line, "core id") {
                physical.insert((package.clone(), core));
            }
        }
        let memory_gib = read("/proc/meminfo")
            .lines()
            .find_map(|line| field(line, "MemTotal"))
            .and_then(|value| value.trim_end_matches("kB").trim().parse::<f64>().ok())
            .map_or(0.0, |kib| kib / (1024.0 * 1024.0));
        let os = read("/etc/os-release")
            .lines()
            .find_map(|line| line.strip_prefix("PRETTY_NAME="))
            .map_or_else(unknown, |name| name.trim_matches('"').to_owned());
        let affinity = read("/proc/self/status")
            .lines()
            .find_map(|line| field(line, "Cpus_allowed_list"))
            .unwrap_or_else(unknown);
        let governor = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
            .ok()
            .map(|g| g.trim().to_owned());
        Self {
            cpu,
            cores: physical.len().max(1),
            threads,
            memory_gib,
            os,
            kernel: read("/proc/sys/kernel/osrelease").trim().to_owned(),
            affinity,
            governor,
            rustc: tool_output("rustc", &["--version"]),
            revision: tool_output("git", &["describe", "--always", "--dirty", "--abbrev=10"]),
        }
    }
}

/// The 1-, 5- and 15-minute load averages, as the kernel prints them.
pub(super) fn load_average() -> String {
    let loadavg = read("/proc/loadavg");
    let averages: Vec<&str> = loadavg.split_whitespace().take(3).collect();
    if averages.len() == 3 {
        averages.join(" ")
    } else {
        unknown()
    }
}

/// The text of `path`, empty when it cannot be read.
fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// What stands for a value that cannot be read.
fn unknown() -> String {
    "unknown".to_owned()
}

/// The first line `program args` prints, run in the workspace, or
/// `unknown`.
fn tool_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .current_dir(crate::python::root())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| text.lines().next().map(str::to_owned))
        .unwrap_or_else(unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_is_described() {
        if !std::path::Path::new("/proc/cpuinfo").exists() {
            return;
        }
        let machine = Machine::describe();
        assert!(machine.threads >= machine.cores);
        assert!(machine.memory_gib > 0.0);
        assert_eq!(load_average().split(' ').count(), 3);
    }
}
