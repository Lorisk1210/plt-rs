//! `_native.numeric`: the processor level pltrs (Rust) computes at, which
//! pltrs (Python) sets to numpy's.
//!
//! numpy picks one of the copies of its loops when it is imported: by the
//! processor, by `NPY_DISABLE_CPU_FEATURES`, by how it was built. pltrs
//! (Rust) replays the copy numpy runs (the order numpy's `argsort` leaves
//! equal values in), so the package hands numpy's answer over when it is
//! imported, before anything is computed; pltrs (Rust) on its own asks
//! the processor.

//!
//! The module also puts numpy's own ufuncs behind the few computations of
//! pltrs (Rust) whose last bit decides something discrete
//! (`pltrs_core::numeric::Ufunc`: the powers an `AsinhLocator` rounds
//! ticks to, the boundaries a `LogNorm` inverts to for a colorbar):
//! numpy computes them with Intel's SVML on Linux with AVX-512 and with
//! the C library elsewhere, and matplotlib decides by numpy's values.

use std::sync::OnceLock;

use pltrs::numeric::{CpuLevel, argsort_at};
use pltrs_core::numeric::{NumpyDispatch, Ufunc, UnknownLevel, evaluate_with};

use crate::binding::prelude::*;

/// numpy's ufuncs, as the package hands them over (each wrapped to issue
/// no warning: pltrs (Rust) issues those of what it replays).
#[derive(Debug)]
struct Ufuncs {
    arcsinh: Owned,
    sinh: Owned,
    log: Owned,
    log2: Owned,
    log10: Owned,
    exp: Owned,
    exp2: Owned,
    power: Owned,
}

/// The ufuncs handed over.
static UFUNCS: OnceLock<Ufuncs> = OnceLock::new();

/// Evaluates `ufunc` on `values` with numpy's ufunc, in place; `false`,
/// leaving them, where numpy's call fails.
fn numpy_evaluates(ufunc: Ufunc, values: &mut [f64]) -> bool {
    let Some(ufuncs) = UFUNCS.get() else {
        return false;
    };
    Python::with_gil(|py| {
        let evaluated = (|| {
            let array = NdArray::from_vec(values.to_vec()).into_py(py)?;
            let results = match ufunc {
                Ufunc::Arcsinh => ufuncs.arcsinh.bind(py).call(&[array], &[]),
                Ufunc::Sinh => ufuncs.sinh.bind(py).call(&[array], &[]),
                Ufunc::Log => ufuncs.log.bind(py).call(&[array], &[]),
                Ufunc::Log2 => ufuncs.log2.bind(py).call(&[array], &[]),
                Ufunc::Log10 => ufuncs.log10.bind(py).call(&[array], &[]),
                Ufunc::Exp => ufuncs.exp.bind(py).call(&[array], &[]),
                Ufunc::Exp2 => ufuncs.exp2.bind(py).call(&[array], &[]),
                Ufunc::PowerOf(base) => {
                    ufuncs.power.bind(py).call(&[base.into_py(py)?, array], &[])
                }
            }?;
            results.extract::<NdArray>()?.into_vec()
        })();
        match evaluated {
            Ok(results) if results.len() == values.len() => {
                values.copy_from_slice(&results);
                true
            }
            _ => false,
        }
    })
}

/// Puts numpy's `arcsinh`, `sinh`, `log`, `log2`, `log10`, `exp`, `exp2`
/// and `power` behind the computations of pltrs (Rust) that ask for
/// numpy's own values; the first ones handed over stay.
#[expect(
    clippy::too_many_arguments,
    clippy::unnecessary_wraps,
    reason = "a native function, in the form `Module::add` registers, taking one ufunc each"
)]
fn use_ufuncs(
    arcsinh: Owned,
    sinh: Owned,
    log: Owned,
    log2: Owned,
    log10: Owned,
    exp: Owned,
    exp2: Owned,
    power: Owned,
) -> PyResult<()> {
    UFUNCS.get_or_init(|| Ufuncs {
        arcsinh,
        sinh,
        log,
        log2,
        log10,
        exp,
        exp2,
        power,
    });
    evaluate_with(numpy_evaluates);
    Ok(())
}

/// Names the level the numpy of major version `major` runs at, from what
/// its `__cpu_features__` hold for the groups `X86_V3` and `X86_V4`
/// (`None` in a numpy without them) and the features `AVX2` and
/// `AVX512_SKX` (`None` on another architecture), and returns the name
/// of the level in force (`v2`, `v3` or `v4`): that one, unless the
/// level was resolved before.
fn follow_numpy(
    major: u32,
    x86_v3: Option<bool>,
    x86_v4: Option<bool>,
    avx2: Option<bool>,
    avx512_skx: Option<bool>,
) -> PyResult<&'static str> {
    let numpy = NumpyDispatch {
        major,
        x86_v3,
        x86_v4,
        avx2: avx2.unwrap_or(false),
        avx512_skx: avx512_skx.unwrap_or(false),
    };
    Ok(CpuLevel::set(CpuLevel::from_numpy(numpy)).name())
}

/// The name of the level in force.
fn cpu_level() -> PyResult<&'static str> {
    Ok(CpuLevel::current().name())
}

/// numpy's `argsort` of `values` as pltrs (Rust) replays it: at the level
/// in force, or at the level named.
fn argsort(values: Vec<f64>, level: Option<String>) -> PyResult<Vec<usize>> {
    let level = match level {
        Some(name) => name
            .parse()
            .map_err(|err: UnknownLevel| PyErr::value_error(err.to_string()))?,
        None => CpuLevel::current(),
    };
    Ok(argsort_at(&values, level))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("follow_numpy", follow_numpy)?;
    module.add("use_ufuncs", use_ufuncs)?;
    module.add("cpu_level", cpu_level)?;
    module.add("argsort", argsort)
}
