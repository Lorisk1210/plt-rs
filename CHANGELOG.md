# Changelog

## <!--fact:version-->0.1.1 (2026-11-22)

* The documentation is shorter and uses Simplified Technical English.
  This includes the README, the API documentation on docs.rs and the
  documents in the repository. The behavior of pltrs did not change.

## 0.1.0 (2026-11-20)

The first release.

### Packages

* pltrs (Python) is the package `pltrs` on PyPI. It has the matplotlib
  3.11 API. It needs Python 3.12 or later and numpy, cycler, Pillow,
  python-dateutil and packaging.
* The `matplotlib` alias package has the version `3.11.2+pltrs.0.1.0`.
  It lets libraries that import `matplotlib`, such as pandas and seaborn,
  run on pltrs. Its wheel is on the GitHub release and not on PyPI.
* pltrs (Rust) is the crate `pltrs` on crates.io. It uses eight more
  crates: `pltrs-core`, `pltrs-codec`, `pltrs-font`, `pltrs-raster`,
  `pltrs-platform`, `pltrs-text`, `pltrs-render` and `pltrs-ui`. All
  nine crates have the same version.

### Features

* Two looks. The modern look is the default for `import pltrs` and for
  Rust. The matplotlib look is the default for `import matplotlib`. The
  environment variable `PLTRS_LOOK` overrides the default.
* In the matplotlib look, PNG output has the same pixels as the
  matplotlib output. SVG, PDF, PostScript and PGF documents draw the same
  items.
* Figure windows on Wayland, X11, macOS and Windows, and figures in a
  web browser.
* pltrs (Python) runs in Jupyter with the inline, nbAgg and ipympl
  backends.
* pltrs (Python) runs the matplotlib backends for Qt, GTK 3, GTK 4, Tk,
  wx and Cairo.
* pltrs (Python) runs the matplotlib extensions for Sphinx.

### Supported systems

* CPython 3.12, 3.13 and 3.14. Each platform has one `cp312-abi3` wheel
  for all Python versions.
* Linux on x86_64 and aarch64, with glibc 2.17 or later or musl 1.2 or
  later.
* macOS 10.12 or later on Intel and macOS 11 or later on Apple silicon,
  in one `universal2` wheel.
* Windows 10 or later, 64-bit.
* On other systems, pip builds the source distribution. The build needs
  Rust 1.99 or later and no other tool. pltrs (Rust) needs the same Rust
  version.

### Limits

* The Rust API is not stable. A 0.x release can change types and
  functions.
* The output is not identical to the matplotlib output in every byte.
  [`DIFFERENCES.md`](DIFFERENCES.md) lists the known differences and the
  features that pltrs does not have.
* The `matplotlib` alias package does not replace the matplotlib wheels
  on PyPI. You must download it from the GitHub release.

### Test results

* pltrs passes all 10,205 tests of the matplotlib 3.11 test suite that
  pass with matplotlib on the same machine.
* pltrs passes 24,559 of the 24,559 tests of 75 libraries that pass with
  matplotlib.
* pltrs passes all 1,091 Jupyter comparisons and tests that pass with
  matplotlib.
* The differential tester runs the programs of 178 collections of user
  code with matplotlib and with pltrs. Of 4,102 programs, 3,970 give the
  same result.
* The conformance tests hold about 48,500 values and 3,400 figures that
  matplotlib produced. pltrs reproduces them.
* pltrs (Python) is 4.3× faster than matplotlib for a figure in a
  running program and 1.79× faster for a complete script. pltrs (Rust)
  is 5.3× and 16× faster. `benchmarks/README.md` has the measurements.
