//! `--level v2|v3|v4`: a command run as on a processor of another level.
//!
//! numpy runs another copy of its loops with AVX-512 than with AVX2 alone
//! or without it, and pltrs follows it (`pltrs::numeric::CpuLevel`). A
//! machine with AVX-512 can be all three: numpy leaves out the copies
//! `NPY_DISABLE_CPU_FEATURES` names, its `OpenBLAS` runs the kernels of the
//! processor `OPENBLAS_CORETYPE` names, and pltrs (Rust) computes at the
//! level `PLTRS_CPU_LEVEL` names. The option starts the command again with
//! the three set, so that everything it runs (the spec in this process,
//! matplotlib and pltrs (Python) in others) computes at that level.

use std::process::{Command, ExitCode};

use pltrs::numeric::CpuLevel;
use pltrs_core::numeric::LEVEL_VARIABLE;

/// The option naming the level.
const OPTION: &str = "--level";

/// The groups of numpy's dispatched loops above `level`, as
/// `NPY_DISABLE_CPU_FEATURES` takes them.
fn disabled_groups(level: CpuLevel) -> &'static str {
    match level {
        CpuLevel::Baseline => "X86_V3 X86_V4 AVX512_ICL AVX512_SPR",
        CpuLevel::V3 => "X86_V4 AVX512_ICL AVX512_SPR",
        CpuLevel::V4 => "",
    }
}

/// The processor whose kernels numpy's `OpenBLAS` runs at `level`: its
/// AVX2 kernels are Haswell's whatever the make, and Nehalem's are those
/// of the baseline; `None` leaves the choice to `OpenBLAS`.
fn blas_core(level: CpuLevel) -> Option<&'static str> {
    match level {
        CpuLevel::Baseline => Some("Nehalem"),
        CpuLevel::V3 => Some("Haswell"),
        CpuLevel::V4 => None,
    }
}

/// The name of the level a command runs at when [`OPTION`] started it
/// (`v3`), for what it keeps apart by level; `None` at the processor's
/// own.
pub(crate) fn asked() -> Option<String> {
    std::env::var(LEVEL_VARIABLE).ok()
}

/// Runs the command `args` again at the level its `--level` names, and
/// returns how that ended; `None` if `args` has no such option.
pub(crate) fn rerun(args: &[String]) -> Option<ExitCode> {
    let at = args.iter().position(|arg| arg == OPTION)?;
    Some(match rerun_at(args, at) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{OPTION}: {err}");
            ExitCode::FAILURE
        }
    })
}

/// [`rerun`] for the option at position `at` of `args`.
fn rerun_at(args: &[String], at: usize) -> Result<ExitCode, String> {
    let level: CpuLevel = args
        .get(at + 1)
        .ok_or("needs a level: v2, v3 or v4")?
        .parse()
        .map_err(|err: pltrs_core::numeric::UnknownLevel| err.to_string())?;
    let processor = CpuLevel::detect();
    if level > processor {
        return Err(format!(
            "this processor runs at {processor}: numpy cannot be run at {level} here"
        ));
    }
    let program = std::env::current_exe().map_err(|err| err.to_string())?;
    let mut command = Command::new(program);
    command
        .args(&args[..at])
        .args(&args[at + 2..])
        .env(LEVEL_VARIABLE, level.name());
    if level < processor {
        command.env("NPY_DISABLE_CPU_FEATURES", disabled_groups(level));
        if let Some(core) = blas_core(level) {
            command.env("OPENBLAS_CORETYPE", core);
        }
    }
    let status = command.status().map_err(|err| err.to_string())?;
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_level_leaves_out_the_groups_above_it() {
        assert_eq!(disabled_groups(CpuLevel::V4), "");
        assert!(!disabled_groups(CpuLevel::V3).contains("X86_V3"));
        assert!(disabled_groups(CpuLevel::V3).contains("X86_V4"));
        assert!(disabled_groups(CpuLevel::Baseline).starts_with("X86_V3 X86_V4"));
        assert_eq!(blas_core(CpuLevel::V4), None);
    }

    #[test]
    fn commands_without_the_option_run_as_they_are() {
        assert!(rerun(&["spec".to_owned(), "argsort".to_owned()]).is_none());
    }

    #[test]
    fn a_missing_or_unknown_level_fails() {
        for args in [vec!["spec", OPTION], vec![OPTION, "avx2", "spec"]] {
            let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
            let at = args.iter().position(|arg| arg == OPTION).unwrap();
            assert!(rerun_at(&args, at).is_err());
        }
    }
}
