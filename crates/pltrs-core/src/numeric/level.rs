//! Which of numpy's code paths a processor takes.
//!
//! numpy's wheels for x86-64 are built for one baseline (`X86_V2`: SSE4.2)
//! and hold a second and third copy of their hot loops, for `X86_V3` (AVX2
//! and FMA) and `X86_V4` (AVX-512), of which numpy picks the best the
//! processor runs when it is imported. The copies do not compute the same:
//! Intel's x86-simd-sort leaves equal values in another order with 4 lanes
//! than with 8, and only the AVX-512 copy for Linux computes `exp`, `log`,
//! `power` and their kin with Intel's SVML instead of the C library.
//! matplotlib's results follow, so pltrs replays the copy numpy runs on the
//! machine it runs on ([`CpuLevel::current`]).

use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

/// The variable naming the level to compute at, whatever the processor:
/// `v2`, `v3` or `v4`. It lets one machine check the results of another
/// (the conformance spec runs at a level on demand), as numpy's
/// `NPY_DISABLE_CPU_FEATURES` does for numpy.
pub const LEVEL_VARIABLE: &str = "PLTRS_CPU_LEVEL";

/// The level in force, once something asked for it.
static LEVEL: OnceLock<CpuLevel> = OnceLock::new();

/// The copy of numpy's dispatched loops a processor runs, in the terms of
/// numpy 2's x86-64 feature groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CpuLevel {
    /// No dispatched copy: an x86-64 processor without AVX2 (numpy's
    /// baseline, `X86_V2`), and every other architecture, where numpy's
    /// loops this crate follows are its plain ones.
    Baseline,
    /// `X86_V3`: AVX2 with FMA, BMI and the rest of that group, and no
    /// AVX-512 (or none the operating system saves).
    V3,
    /// `X86_V4`: AVX-512 F, CD, BW, DQ and VL on top of `X86_V3`.
    V4,
}

/// What numpy says of its dispatch (`__cpu_features__` of its
/// `_multiarray_umath`), as far as the level follows from it.
///
/// numpy 2.5 groups the features of x86-64 into `X86_V2`, `X86_V3` and
/// `X86_V4` and dispatches by group; a group it was told to leave out
/// (`NPY_DISABLE_CPU_FEATURES`) reads false while its features still read
/// true. Earlier releases (2.2 has no groups) dispatch the same copies by
/// the features `AVX2` and `AVX512_SKX`, and numpy 1 has x86-simd-sort's
/// AVX-512 sort of indices alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NumpyDispatch {
    /// numpy's major version.
    pub major: u32,
    /// Whether the group `X86_V3` is enabled; `None` where numpy has no
    /// such group.
    pub x86_v3: Option<bool>,
    /// Whether the group `X86_V4` is enabled; `None` where numpy has no
    /// such group.
    pub x86_v4: Option<bool>,
    /// Whether the feature `AVX2` is enabled.
    pub avx2: bool,
    /// Whether the feature `AVX512_SKX` is enabled.
    pub avx512_skx: bool,
}

/// A level name [`CpuLevel`] does not know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownLevel(String);

impl fmt::Display for UnknownLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown processor level {:?}: expected v2, v3 or v4",
            self.0
        )
    }
}

impl std::error::Error for UnknownLevel {}

impl CpuLevel {
    /// Every level, lowest first.
    pub const ALL: [Self; 3] = [Self::Baseline, Self::V3, Self::V4];

    /// The level pltrs computes at in this process, resolved on the first
    /// call and fixed from then on: the one [`CpuLevel::set`] named, else
    /// the one [`LEVEL_VARIABLE`] names, else the processor's
    /// ([`CpuLevel::detect`]). A value of the variable that names no level
    /// is ignored.
    pub fn current() -> Self {
        *LEVEL.get_or_init(|| {
            std::env::var(LEVEL_VARIABLE)
                .ok()
                .and_then(|name| name.parse().ok())
                .unwrap_or_else(Self::detect)
        })
    }

    /// Names the level for this process, and returns the one in force: the
    /// given one unless the level was resolved before. pltrs (Python)
    /// hands over numpy's own answer when it is imported, so that pltrs
    /// follows the numpy beside it whatever made numpy choose (the
    /// processor, `NPY_DISABLE_CPU_FEATURES`, another build).
    pub fn set(level: Self) -> Self {
        *LEVEL.get_or_init(|| level)
    }

    /// The level of this processor by numpy's rule (its
    /// `npy__cpu_init_features`): each group needs every feature of the
    /// one below. Other architectures have none of the groups.
    ///
    /// On an Intel Mac with AVX-512 numpy reads the kernel's commpage,
    /// because macOS enables the wide registers per thread on first use;
    /// the standard library asks the processor alone and reports
    /// `X86_V3` there.
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::is_x86_feature_detected as has;
            let v3 = has!("avx")
                && has!("avx2")
                && has!("f16c")
                && has!("fma")
                && has!("bmi1")
                && has!("bmi2")
                && has!("lzcnt")
                && has!("movbe");
            let v4 = v3
                && has!("avx512f")
                && has!("avx512cd")
                && has!("avx512bw")
                && has!("avx512dq")
                && has!("avx512vl");
            if v4 {
                Self::V4
            } else if v3 {
                Self::V3
            } else {
                Self::Baseline
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Self::Baseline
        }
    }

    /// The level numpy runs at, from what it says of its dispatch.
    ///
    /// ```
    /// use pltrs_core::numeric::{CpuLevel, NumpyDispatch};
    /// // numpy 2.5 on a processor with AVX2 and no AVX-512.
    /// let numpy = NumpyDispatch {
    ///     major: 2,
    ///     x86_v3: Some(true),
    ///     x86_v4: Some(false),
    ///     avx2: true,
    ///     avx512_skx: false,
    /// };
    /// assert_eq!(CpuLevel::from_numpy(numpy), CpuLevel::V3);
    /// ```
    #[must_use]
    pub fn from_numpy(numpy: NumpyDispatch) -> Self {
        let (v3, v4) = match (numpy.x86_v3, numpy.x86_v4) {
            (Some(v3), Some(v4)) => (v3, v3 && v4),
            // Before the groups, numpy dispatched by single features, and
            // its AVX2 copy of the sort came with numpy 2.
            _ => (
                numpy.avx512_skx || (numpy.avx2 && numpy.major >= 2),
                numpy.avx512_skx,
            ),
        };
        if v4 {
            Self::V4
        } else if v3 {
            Self::V3
        } else {
            Self::Baseline
        }
    }

    /// The level's short name: `v2`, `v3` or `v4`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Baseline => "v2",
            Self::V3 => "v3",
            Self::V4 => "v4",
        }
    }
}

impl fmt::Display for CpuLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for CpuLevel {
    type Err = UnknownLevel;

    /// Reads a level's name, in any case, with or without numpy's
    /// `X86_` prefix; `baseline` is `v2`.
    fn from_str(name: &str) -> Result<Self, UnknownLevel> {
        let lower = name.trim().to_ascii_lowercase();
        match lower.strip_prefix("x86_").unwrap_or(&lower) {
            "v2" | "baseline" => Ok(Self::Baseline),
            "v3" => Ok(Self::V3),
            "v4" => Ok(Self::V4),
            _ => Err(UnknownLevel(name.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for level in CpuLevel::ALL {
            assert_eq!(level.name().parse(), Ok(level));
            assert_eq!(level.to_string(), level.name());
        }
        assert_eq!("X86_V3".parse(), Ok(CpuLevel::V3));
        assert_eq!(" Baseline\n".parse(), Ok(CpuLevel::Baseline));
    }

    #[test]
    fn unknown_names_are_refused() {
        for name in ["", "v5", "avx2", "x86_", "3"] {
            let error = name.parse::<CpuLevel>().unwrap_err();
            assert!(error.to_string().contains("expected v2, v3 or v4"));
        }
    }

    #[test]
    fn numpys_groups_decide_where_it_has_them() {
        let groups = |x86_v3, x86_v4| NumpyDispatch {
            major: 2,
            x86_v3: Some(x86_v3),
            x86_v4: Some(x86_v4),
            // A group left out by NPY_DISABLE_CPU_FEATURES leaves the
            // features it is made of enabled.
            avx2: true,
            avx512_skx: true,
        };
        assert_eq!(CpuLevel::from_numpy(groups(true, true)), CpuLevel::V4);
        assert_eq!(CpuLevel::from_numpy(groups(true, false)), CpuLevel::V3);
        assert_eq!(
            CpuLevel::from_numpy(groups(false, false)),
            CpuLevel::Baseline
        );
        // numpy never enables a group without the one below.
        assert_eq!(
            CpuLevel::from_numpy(groups(false, true)),
            CpuLevel::Baseline
        );
    }

    #[test]
    fn numpys_features_decide_before_the_groups() {
        let features = |major, avx2, avx512_skx| NumpyDispatch {
            major,
            avx2,
            avx512_skx,
            ..NumpyDispatch::default()
        };
        assert_eq!(CpuLevel::from_numpy(features(2, true, true)), CpuLevel::V4);
        assert_eq!(CpuLevel::from_numpy(features(2, true, false)), CpuLevel::V3);
        assert_eq!(
            CpuLevel::from_numpy(features(2, false, false)),
            CpuLevel::Baseline
        );
        // numpy 1 sorts indices with AVX-512 or with its own code.
        assert_eq!(CpuLevel::from_numpy(features(1, true, true)), CpuLevel::V4);
        assert_eq!(
            CpuLevel::from_numpy(features(1, true, false)),
            CpuLevel::Baseline
        );
        // Another architecture: none of it.
        assert_eq!(
            CpuLevel::from_numpy(NumpyDispatch::default()),
            CpuLevel::Baseline
        );
    }

    #[test]
    fn levels_are_ordered() {
        assert!(CpuLevel::Baseline < CpuLevel::V3 && CpuLevel::V3 < CpuLevel::V4);
        assert_eq!(CpuLevel::ALL.iter().max(), Some(&CpuLevel::V4));
    }

    #[test]
    fn the_level_in_force_never_changes() {
        let first = CpuLevel::current();
        for level in CpuLevel::ALL {
            assert_eq!(CpuLevel::set(level), first);
        }
        assert_eq!(CpuLevel::current(), first);
    }

    #[test]
    fn detection_agrees_with_the_architecture() {
        let level = CpuLevel::detect();
        if cfg!(target_arch = "x86_64") {
            // Whatever the processor has, a level never lacks AVX2 while
            // claiming AVX-512.
            assert!(CpuLevel::ALL.contains(&level));
        } else {
            assert_eq!(level, CpuLevel::Baseline);
        }
    }
}
