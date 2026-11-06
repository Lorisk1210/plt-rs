//! What the benchmark ran on: the machine, its load, and the versions of
//! the tools, read from `/proc`, `/sys` and the tools themselves (on
//! Windows from its management interface, through PowerShell).

#[cfg(not(windows))]
use std::collections::BTreeSet;
#[cfg(not(windows))]
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
    #[cfg(windows)]
    pub(super) fn describe() -> Self {
        windows::describe()
    }

    /// Describes this machine. What cannot be read is `unknown`.
    #[cfg(not(windows))]
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
#[cfg(not(windows))]
pub(super) fn load_average() -> String {
    let loadavg = read("/proc/loadavg");
    let averages: Vec<&str> = loadavg.split_whitespace().take(3).collect();
    if averages.len() == 3 {
        averages.join(" ")
    } else {
        unknown()
    }
}

/// How busy the processors are, as Windows reports it in place of load
/// averages (`Win32_Processor.LoadPercentage`, the last second's).
#[cfg(windows)]
pub(super) fn load_average() -> String {
    windows::query(
        "(Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average",
    )
    .first()
    .map_or_else(unknown, |load| format!("{load}% busy"))
}

/// The machine as Windows describes it.
#[cfg(windows)]
mod windows {
    use super::{Machine, tool_output, unknown};

    /// What [`describe`] asks: the CPU, its cores and threads, the memory
    /// in bytes, the system's name and version, the active power plan and
    /// the processors this process may run on (a bit mask).
    const DESCRIPTION: &str = "$p = Get-CimInstance Win32_Processor | Select-Object -First 1; \
        $o = Get-CimInstance Win32_OperatingSystem; \
        $p.Name; $p.NumberOfCores; $p.NumberOfLogicalProcessors; \
        (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory; \
        $o.Caption; $o.Version; \
        (Get-CimInstance -Namespace root/cimv2/power Win32_PowerPlan -Filter 'IsActive=True').ElementName; \
        (Get-Process -Id $PID).ProcessorAffinity";

    /// The lines PowerShell prints running `script`, empty when it cannot
    /// run.
    pub(super) fn query(script: &str) -> Vec<String> {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|line| line.trim().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The processors a bit mask names, as ranges (`0-19`).
    fn processors(mask: u64) -> String {
        let mut ranges: Vec<String> = Vec::new();
        let mut cpu = 0;
        while cpu < 64 {
            if mask >> cpu & 1 == 0 {
                cpu += 1;
                continue;
            }
            let first = cpu;
            while cpu < 64 && mask >> cpu & 1 == 1 {
                cpu += 1;
            }
            ranges.push(if cpu - 1 == first {
                first.to_string()
            } else {
                format!("{first}-{}", cpu - 1)
            });
        }
        ranges.join(",")
    }

    /// Describes this machine (PowerShell's processes run where this one
    /// may: they inherit its processors).
    pub(super) fn describe() -> Machine {
        let lines = query(DESCRIPTION);
        let line = |i: usize| lines.get(i).filter(|line| !line.is_empty()).cloned();
        let number = |i: usize| line(i).and_then(|text| text.parse::<u64>().ok());
        let threads = number(2).and_then(|n| usize::try_from(n).ok());
        Machine {
            cpu: line(0).unwrap_or_else(unknown),
            cores: number(1).and_then(|n| usize::try_from(n).ok()).unwrap_or(1),
            threads: threads.unwrap_or(1),
            #[expect(clippy::cast_precision_loss, reason = "bytes of memory, as GiB")]
            memory_gib: number(3).map_or(0.0, |bytes| bytes as f64 / f64::from(1 << 30)),
            os: line(4).unwrap_or_else(unknown),
            kernel: line(5).map_or_else(unknown, |version| format!("Windows NT {version}")),
            affinity: number(7).map_or_else(unknown, processors),
            governor: line(6).map(|plan| format!("power plan {plan}")),
            rustc: tool_output("rustc", &["--version"]),
            revision: tool_output("git", &["describe", "--always", "--dirty", "--abbrev=10"]),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn masks_read_as_ranges() {
            assert_eq!(processors(0xF_FFFF), "0-19");
            assert_eq!(processors(0b1011), "0-1,3");
            assert_eq!(processors(1 << 63), "63");
        }
    }
}

/// The text of `path`, empty when it cannot be read.
#[cfg(not(windows))]
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
        if cfg!(not(windows)) && !std::path::Path::new("/proc/cpuinfo").exists() {
            return;
        }
        let machine = Machine::describe();
        assert!(machine.threads >= machine.cores);
        assert!(machine.memory_gib > 0.0);
        let load = load_average();
        if cfg!(windows) {
            assert!(load.ends_with("% busy"), "{load}");
        } else {
            assert_eq!(load.split(' ').count(), 3);
        }
    }
}
