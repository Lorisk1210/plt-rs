//! What the benchmark ran on: the machine, its load, and the versions of
//! the tools, read from `/proc`, `/sys` and the tools themselves (on
//! Windows from its management interface, through PowerShell; on macOS
//! from `sysctl` and `sw_vers`), or, for a build packed elsewhere, from the
//! pack's manifest.

#[cfg(not(any(windows, target_os = "macos")))]
use std::collections::BTreeSet;
#[cfg(not(any(windows, target_os = "macos")))]
use std::fs;

use super::pack::{self, first_line, unknown};

/// The machine and the software around the timings.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Machine {
    /// The maker and model of the computer, where the firmware names them.
    pub(super) model: Option<String>,
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
    /// Why the benchmark runs on those CPUs only, when it is confined to
    /// the fastest class of CPUs of different speeds (`super::confine`).
    pub(super) confinement: Option<String>,
    /// The first CPU's frequency governor, where the kernel reports one.
    pub(super) governor: Option<String>,
    /// The power settings that steer the clock, where the kernel reports
    /// them: the platform's power profile, the first CPU's energy
    /// preference, and whether the machine runs on mains power.
    pub(super) power: Option<String>,
    /// The Rust compiler's version.
    pub(super) rustc: String,
    /// The commit benchmarked, `-dirty` when the tree has changes, or as
    /// the pack names it.
    pub(super) revision: String,
}

impl Machine {
    /// Describes this machine. What cannot be read is `unknown`.
    #[cfg(windows)]
    pub(super) fn describe() -> Self {
        windows::describe()
    }

    /// Describes this machine. What cannot be read is `unknown`.
    #[cfg(target_os = "macos")]
    pub(super) fn describe() -> Self {
        macos::describe()
    }

    /// Describes this machine. What cannot be read is `unknown`.
    #[cfg(not(any(windows, target_os = "macos")))]
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
        #[cfg(target_os = "linux")]
        let confinement = super::confine::wanted()
            .filter(|confinement| confinement.cpus == affinity)
            .map(|confinement| confinement.reason);
        #[cfg(not(target_os = "linux"))]
        let confinement = None;
        let governor = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
            .ok()
            .map(|g| g.trim().to_owned());
        let (rustc, revision) = build();
        Self {
            model: model(),
            cpu,
            cores: physical.len().max(1),
            threads,
            memory_gib,
            os,
            kernel: read("/proc/sys/kernel/osrelease").trim().to_owned(),
            affinity,
            confinement,
            governor,
            power: power(),
            rustc,
            revision,
        }
    }
}

/// The Rust compiler and the commit of the build benchmarked: the pack's,
/// else the workspace's.
fn build() -> (String, String) {
    let tree = pack::current();
    match &tree.build {
        Some(build) => (build.rustc.clone(), build.revision.clone()),
        None => (
            first_line(&tree.root, "rustc", &["--version"]),
            first_line(&tree.root, "git", &pack::describe()),
        ),
    }
}

/// The computer's maker and model, as the firmware's DMI tables name
/// them.
#[cfg(not(any(windows, target_os = "macos")))]
fn model() -> Option<String> {
    let field = |name: &str| {
        fs::read_to_string(format!("/sys/class/dmi/id/{name}"))
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    match (field("sys_vendor"), field("product_name")) {
        (Some(vendor), Some(product)) => Some(format!("{vendor} {product}")),
        (vendor, product) => vendor.or(product),
    }
}

/// The power settings the kernel reports, described in words.
#[cfg(not(any(windows, target_os = "macos")))]
fn power() -> Option<String> {
    let setting = |path: &str| {
        fs::read_to_string(path)
            .ok()
            .map(|value| value.trim().to_owned())
    };
    let mut parts = Vec::new();
    if let Some(profile) = setting("/sys/firmware/acpi/platform_profile") {
        parts.push(format!("power profile {profile}"));
    }
    if let Some(preference) =
        setting("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference")
    {
        parts.push(format!("energy preference {preference}"));
    }
    if let Some(mains) = on_mains() {
        parts.push(
            if mains {
                "on mains power"
            } else {
                "on battery"
            }
            .to_owned(),
        );
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Whether a mains supply powers the machine, where it reports a supply
/// of that type.
#[cfg(not(any(windows, target_os = "macos")))]
fn on_mains() -> Option<bool> {
    let supplies = fs::read_dir("/sys/class/power_supply").ok()?;
    let mut mains = None;
    for supply in supplies.filter_map(Result::ok).map(|entry| entry.path()) {
        if read(&supply.join("type").to_string_lossy()).trim() == "Mains" {
            let online = read(&supply.join("online").to_string_lossy()).trim() == "1";
            mains = Some(mains.unwrap_or(false) || online);
        }
    }
    mains
}

/// The 1-, 5- and 15-minute load averages, as the kernel prints them.
#[cfg(not(any(windows, target_os = "macos")))]
pub(super) fn load_average() -> String {
    let loadavg = read("/proc/loadavg");
    let averages: Vec<&str> = loadavg.split_whitespace().take(3).collect();
    if averages.len() == 3 {
        averages.join(" ")
    } else {
        unknown()
    }
}

/// The 1-, 5- and 15-minute load averages, as `vm.loadavg` gives them
/// (`{ 1.0 2.0 3.0 }`).
#[cfg(target_os = "macos")]
pub(super) fn load_average() -> String {
    let loadavg = macos::sysctl("vm.loadavg");
    let averages: Vec<&str> = loadavg
        .split_whitespace()
        .filter(|word| !matches!(*word, "{" | "}"))
        .take(3)
        .collect();
    if averages.len() == 3 {
        averages.join(" ")
    } else {
        unknown()
    }
}

/// The machine as macOS describes it.
#[cfg(target_os = "macos")]
mod macos {
    use super::{Machine, build, first_line, unknown};

    /// The value of the kernel variable `name`, or `unknown`.
    pub(super) fn sysctl(name: &str) -> String {
        tool("sysctl", &["-n", name])
    }

    /// The first line `program args` prints, or `unknown`.
    fn tool(program: &str, args: &[&str]) -> String {
        first_line(std::path::Path::new("/"), program, args)
    }

    /// Describes this machine. Apple silicon has performance and
    /// efficiency cores, which the CPU's description counts; macOS binds
    /// no process to CPUs, so every process may run on all of them.
    pub(super) fn describe() -> Machine {
        let count = |name: &str| sysctl(name).parse::<usize>().ok();
        let threads = count("hw.logicalcpu").unwrap_or(1);
        let mut cpu = sysctl("machdep.cpu.brand_string");
        if let (Some(performance), Some(efficiency)) = (
            count("hw.perflevel0.physicalcpu"),
            count("hw.perflevel1.physicalcpu"),
        ) {
            cpu = format!("{cpu} ({performance} performance and {efficiency} efficiency cores)");
        }
        let model = sysctl("hw.model");
        let (rustc, revision) = build();
        Machine {
            model: (model != unknown()).then_some(model),
            cpu,
            cores: count("hw.physicalcpu").unwrap_or(1),
            threads,
            #[expect(clippy::cast_precision_loss, reason = "bytes of memory, as GiB")]
            memory_gib: sysctl("hw.memsize")
                .parse::<u64>()
                .map_or(0.0, |bytes| bytes as f64 / f64::from(1 << 30)),
            os: format!(
                "{} {}",
                tool("sw_vers", &["-productName"]),
                tool("sw_vers", &["-productVersion"])
            ),
            kernel: format!("{} {}", sysctl("kern.ostype"), sysctl("kern.osrelease")),
            affinity: format!("all {threads}"),
            confinement: None,
            governor: None,
            power: None,
            rustc,
            revision,
        }
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
    use super::{Machine, build, unknown};

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
        let (rustc, revision) = build();
        Machine {
            model: None,
            cpu: line(0).unwrap_or_else(unknown),
            cores: number(1).and_then(|n| usize::try_from(n).ok()).unwrap_or(1),
            threads: threads.unwrap_or(1),
            #[expect(clippy::cast_precision_loss, reason = "bytes of memory, as GiB")]
            memory_gib: number(3).map_or(0.0, |bytes| bytes as f64 / f64::from(1 << 30)),
            os: line(4).unwrap_or_else(unknown),
            kernel: line(5).map_or_else(unknown, |version| format!("Windows NT {version}")),
            affinity: number(7).map_or_else(unknown, processors),
            confinement: None,
            governor: line(6).map(|plan| format!("power plan {plan}")),
            power: None,
            rustc,
            revision,
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
#[cfg(not(any(windows, target_os = "macos")))]
fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_is_described() {
        if cfg!(not(any(windows, target_os = "macos")))
            && !std::path::Path::new("/proc/cpuinfo").exists()
        {
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
