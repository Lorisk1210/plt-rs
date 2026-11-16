//! Takes the maths functions of pltrs from the Universal C Runtime (UCRT)
//! on Windows with a GNU toolchain.
//!
//! On Windows numpy and CPython compute `sin`, `exp`, `pow`, ... with the
//! UCRT, so matplotlib's results there carry its last bits, and pltrs
//! agrees with them by calling the same functions. Built with MSVC it
//! does. Rustup's GNU toolchain (`x86_64-pc-windows-gnu`) links
//! MinGW-w64's instead: x87 code built into its `libmsvcrt.a` and
//! `libmingwex.a`, and the old `msvcrt.dll`'s, whose last bits differ
//! from the UCRT's for a few results in a thousand (one moved a bin edge
//! of matplotlib's `hexbin` test with log bins).
//!
//! For that target this script writes an import library of the UCRT's
//! maths functions ([`import_library`], for the API set
//! `api-ms-win-crt-math-l1-1-0.dll` that MSVC's programs import them
//! from) and has every program that links pltrs-core link it. rustc puts
//! the libraries crates link before the C runtime's on the linker's
//! command line, so these functions are found first, by the code of every
//! crate. LLVM computes a `sin` and a `cos` of the same
//! value with one call of `sincos`, which the UCRT lacks; the library
//! defines it from the UCRT's `sin` and `cos` ([`sincos`]).
//!
//! Rustup ships no import library of the UCRT, and rustc's `raw-dylib`
//! makes one on GNU targets with binutils' `dlltool` and assembler, which
//! rustup lacks: so the script writes it, in the format of the MSVC
//! tools, which GNU ld reads.
#![expect(
    clippy::print_stdout,
    reason = "a build script talks to cargo on stdout"
)]

mod coff;
mod import_library;
mod sincos;

use std::path::PathBuf;

/// The UCRT's API set of maths functions.
const UCRT_MATH: &str = "api-ms-win-crt-math-l1-1-0.dll";

/// The functions taken from [`UCRT_MATH`]: those Rust's standard library
/// and LLVM's code call, for `f64` and for `f32`.
const FUNCTIONS: [&str; 2] = [
    "acos asin atan atan2 cbrt ceil cos cosh erf erfc exp exp2 expm1 floor fma fmax fmin fmod \
     frexp _hypot hypot ldexp lgamma log log10 log1p log2 nearbyint pow rint round sin sinh tan \
     tanh tgamma trunc",
    "acosf asinf atanf atan2f cbrtf ceilf cosf coshf erff erfcf expf exp2f expm1f floorf fmaf \
     fmaxf fminf fmodf _hypotf lgammaf logf log10f log1pf log2f nearbyintf powf rintf roundf \
     sinf sinhf tanf tanhf tgammaf truncf",
];

/// The name the library is linked by (`-l pltrs_ucrt_math`).
const LIBRARY: &str = "pltrs_ucrt_math";

fn main() {
    println!("cargo::rerun-if-changed=build");
    let target = |key: &str| std::env::var(format!("CARGO_CFG_TARGET_{key}")).unwrap_or_default();
    if target("OS") != "windows" || target("ENV") != "gnu" || target("ARCH") != "x86_64" {
        return;
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let functions: Vec<&str> = FUNCTIONS
        .iter()
        .flat_map(|list| list.split_whitespace())
        .collect();
    let library = import_library::write(UCRT_MATH, &functions, vec![sincos::member()]);
    std::fs::write(out.join(format!("lib{LIBRARY}.a")), library)
        .expect("OUT_DIR takes the import library");
    println!("cargo::rustc-link-search=native={}", out.display());
    println!("cargo::rustc-link-lib=dylib={LIBRARY}");
}
