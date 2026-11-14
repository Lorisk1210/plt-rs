//! Confining the benchmark to one class of CPUs.
//!
//! Some processors have cores of two speeds (the Ryzen AI 300 series: four
//! Zen 5 cores at 5.2 GHz beside eight Zen 5c cores at 3.3 GHz). Where the
//! operating system places a new process on either at random, as Linux does
//! without preferred-core scheduling, the median of a short cold run
//! depends on where most of the runs landed: a bare interpreter starts in
//! 8.0 to 8.4 ms on a fast core of that laptop and in 12.4 to 13.2 ms on a
//! slow one. The speed-ups stay fair, since every side lands at random
//! alike, but no two reports agree. So on Linux, when the CPUs' highest
//! frequencies differ, the harness runs again under `taskset`, confined to
//! the CPUs of the fastest class, and every process it starts (matplotlib,
//! pltrs (Python) and pltrs (Rust) alike) inherits the confinement. A
//! machine whose CPUs are all alike runs as before.

#[cfg(any(target_os = "linux", test))]
use std::fmt::Write as _;

/// How the benchmark is confined: the CPUs and why.
#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Confinement {
    /// The CPUs, as `taskset` and the kernel list them (`0-3,12-15`).
    pub(super) cpus: String,
    /// Why, in the report's words.
    pub(super) reason: String,
}

/// The CPUs of the fastest class, the class' highest frequency and the
/// others' (in kHz), when the CPUs `max_freqs` describes (each CPU's number
/// and its `cpuinfo_max_freq`, `None` where it cannot be read) are of
/// more than one class; `None` when they are all alike or a frequency is
/// unknown (nothing can be told apart then).
#[cfg(any(target_os = "linux", test))]
pub(super) fn fastest_class(
    max_freqs: &[(usize, Option<u64>)],
) -> Option<(Vec<usize>, u64, Vec<u64>)> {
    let known: Option<Vec<(usize, u64)>> = max_freqs
        .iter()
        .map(|&(cpu, freq)| freq.map(|freq| (cpu, freq)))
        .collect();
    let known = known?;
    let fastest = known.iter().map(|&(_, freq)| freq).max()?;
    let mut others: Vec<u64> = known
        .iter()
        .map(|&(_, freq)| freq)
        .filter(|&freq| freq != fastest)
        .collect();
    if others.is_empty() {
        return None;
    }
    others.sort_unstable();
    others.dedup();
    let mut cpus: Vec<usize> = known
        .iter()
        .filter(|&&(_, freq)| freq == fastest)
        .map(|&(cpu, _)| cpu)
        .collect();
    cpus.sort_unstable();
    Some((cpus, fastest, others))
}

/// `cpus` (sorted) as the kernel lists CPUs: runs as `a-b`, joined by
/// commas.
#[cfg(any(target_os = "linux", test))]
pub(super) fn cpu_list(cpus: &[usize]) -> String {
    let mut out = String::new();
    let mut index = 0;
    while index < cpus.len() {
        let start = cpus[index];
        let mut end = start;
        while index + 1 < cpus.len() && cpus[index + 1] == end + 1 {
            index += 1;
            end = cpus[index];
        }
        if !out.is_empty() {
            out.push(',');
        }
        if start == end {
            let _ = write!(out, "{start}");
        } else {
            let _ = write!(out, "{start}-{end}");
        }
        index += 1;
    }
    out
}

/// The confinement of CPUs `cpus` of `total`, the fastest class at
/// `fastest` kHz beside classes at `others` kHz.
#[cfg(any(target_os = "linux", test))]
pub(super) fn describe(cpus: &[usize], total: usize, fastest: u64, others: &[u64]) -> Confinement {
    let mhz = |khz: u64| format!("{} MHz", khz / 1000);
    let others: Vec<String> = others.iter().map(|&khz| mhz(khz)).collect();
    Confinement {
        cpus: cpu_list(cpus),
        reason: format!(
            "{} of the {total} CPUs, those of the fastest class ({}; the others {})",
            cpus.len(),
            mhz(fastest),
            others.join(", ")
        ),
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;
    use std::path::Path;
    use std::process::{Command, ExitCode};

    use super::{Confinement, describe, fastest_class};

    /// Set in the harness `taskset` started, so that it does not start
    /// itself again whatever the kernel made of the mask.
    const CONFINED: &str = "PLTRS_BENCH_CONFINED";

    /// Each online CPU's number and its highest frequency, from `/sys`.
    fn max_freqs() -> Vec<(usize, Option<u64>)> {
        let root = Path::new("/sys/devices/system/cpu");
        let Ok(entries) = fs::read_dir(root) else {
            return Vec::new();
        };
        let mut cpus: Vec<(usize, Option<u64>)> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let cpu = name.strip_prefix("cpu")?.parse::<usize>().ok()?;
                let freq = fs::read_to_string(entry.path().join("cpufreq/cpuinfo_max_freq"))
                    .ok()
                    .and_then(|text| text.trim().parse().ok());
                Some((cpu, freq))
            })
            .collect();
        cpus.sort_unstable();
        cpus
    }

    /// The CPUs this process may run on, as the kernel lists them.
    fn allowed() -> Option<String> {
        fs::read_to_string("/proc/self/status")
            .ok()?
            .lines()
            .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
            .map(|list| list.trim().to_owned())
    }

    /// The confinement this machine calls for, if any.
    pub(in crate::bench) fn wanted() -> Option<Confinement> {
        let freqs = max_freqs();
        let (cpus, fastest, others) = fastest_class(&freqs)?;
        Some(describe(&cpus, freqs.len(), fastest, &others))
    }

    /// Runs the harness again confined to `confinement`'s CPUs, unless it
    /// already is; its exit code, or `None` to go on in this process.
    pub(in crate::bench) fn enter(confinement: &Confinement) -> Option<ExitCode> {
        if allowed().as_deref() == Some(confinement.cpus.as_str())
            || std::env::var_os(CONFINED).is_some()
        {
            return None;
        }
        let exe = std::env::current_exe().ok()?;
        let status = Command::new("taskset")
            .args(["-c", &confinement.cpus])
            .arg(exe)
            .args(std::env::args_os().skip(1))
            .env(CONFINED, "1")
            .status();
        match status {
            Ok(status) => Some(match status.code() {
                Some(0) => ExitCode::SUCCESS,
                Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
                None => ExitCode::FAILURE,
            }),
            Err(err) => {
                eprintln!(
                    "warning: the CPUs differ in speed but `taskset` cannot run ({err}): \
                     timing unconfined, where the medians of short cold runs depend on \
                     where each run lands"
                );
                None
            }
        }
    }
}

/// The confinement this machine calls for, if any.
#[cfg(target_os = "linux")]
pub(super) fn wanted() -> Option<Confinement> {
    linux::wanted()
}

/// Runs the harness again, confined as this machine calls for, unless it
/// is already; the confined run's exit code, or `None` to go on here.
pub(super) fn enter() -> Option<std::process::ExitCode> {
    #[cfg(target_os = "linux")]
    {
        wanted().and_then(|confinement| linux::enter(&confinement))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{cpu_list, describe, fastest_class};

    #[test]
    fn two_classes_give_the_fastest() {
        // The laptop: CPUs 0-3 and 12-15 at 5.16 GHz, the rest at 3.29.
        let freqs: Vec<(usize, Option<u64>)> = (0..24)
            .map(|cpu| {
                let fast = cpu < 4 || (12..16).contains(&cpu);
                (cpu, Some(if fast { 5_157_895 } else { 3_289_474 }))
            })
            .collect();
        let (cpus, fastest, others) = fastest_class(&freqs).expect("two classes");
        assert_eq!(cpu_list(&cpus), "0-3,12-15");
        assert_eq!((fastest, others), (5_157_895, vec![3_289_474]));
        let confinement = describe(&cpus, freqs.len(), fastest, &[3_289_474]);
        assert_eq!(confinement.cpus, "0-3,12-15");
        assert_eq!(
            confinement.reason,
            "8 of the 24 CPUs, those of the fastest class (5157 MHz; the others 3289 MHz)"
        );
    }

    #[test]
    fn one_class_or_unknown_frequencies_confine_nothing() {
        let alike: Vec<(usize, Option<u64>)> = (0..8).map(|cpu| (cpu, Some(4_000_000))).collect();
        assert_eq!(fastest_class(&alike), None);
        let unreadable = vec![(0, Some(5_000_000)), (1, None), (2, Some(3_000_000))];
        assert_eq!(fastest_class(&unreadable), None);
        assert_eq!(fastest_class(&[]), None);
    }

    #[test]
    fn cpu_lists_join_runs() {
        assert_eq!(cpu_list(&[0, 1, 2, 5, 7, 8]), "0-2,5,7-8");
        assert_eq!(cpu_list(&[3]), "3");
        assert_eq!(cpu_list(&[]), "");
    }
}
