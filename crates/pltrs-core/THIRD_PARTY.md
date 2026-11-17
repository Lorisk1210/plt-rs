# Third-party code and data in pltrs-core

<!-- Written by python/tools/crate_notices.py from the repository's THIRD_PARTY.md: edit that file, then run the tool. -->

pltrs-core is part of pltrs, licensed under the MIT licence or the
Apache License 2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`).
The parts of it listed below come from other projects and keep their own
licences, whose texts are in `LICENSES/` (or beside the files they
cover). These are the rows of pltrs' `THIRD_PARTY.md` that concern this
crate; paths are those of pltrs' repository.

## Ported code

| Project | Licence (text) | Ported into |
|---|---|---|
| NumPy 2.5 and Intel's x86-simd-sort | BSD-3-Clause (`numpy.txt`) | `argsort` of floats, down to the order of ties, as numpy sorts at each processor level: its own introsort, and x86-simd-sort's AVX2 and AVX-512 sorts of indices (`crates/pltrs/src/numeric/argsort`); numpy's rule for the level of a processor (`crates/pltrs-core/src/numeric/level.rs`); and the exact results of the numpy functions matplotlib calls (`interp`, `searchsorted`, `arange`, `linspace`, quantiles and others; `crates/pltrs/src/numeric` and throughout `pltrs`) |
| CPython 3.14 | PSF License Agreement (`cpython.txt`) | float formatting (`repr`, `%` and `str.format`, `crates/pltrs-core/src/pyfmt`), `divmod` and `round` (`crates/pltrs/src/ticker/pymath.rs`), the `datetime` types (`crates/pltrs/src/dates`), the `USTimeZone` example of the `datetime` documentation (the `tzinfo` classes of the `dates_tz` probes, `spec/reference/probes/_dates_tz.py` and `crates/pltrs-spec/src/probes/areas/dates_tz/program.rs`, and of `python/tests/test_program_zones.py`), and the program search of `shutil.which` (`crates/pltrs-core/src/os.rs`) |
| fdlibm (Sun Microsystems), as the C libraries carry it | Sun's fdlibm notice (`fdlibm.txt`) | the ranges and formulas of the inverse hyperbolic sine of `s_asinh.c`, which numpy's `arcsinh` computes by through the C library (`pltrs_core::numeric::np_arcsinh`) |
