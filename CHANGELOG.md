# Changelog

The numbers below are tied to files of the tree (the HTML comments before
them, which `python/tests/test_readme_facts.py` checks).

## <!--fact:version-->0.1.0 (unreleased)

The first release.

### What pltrs is

matplotlib 3.11, rebuilt from scratch in Rust, with no crates.io
dependencies, in two faces:

* **pltrs (Python)**, the package `pltrs` on PyPI: matplotlib's API
  (`import pltrs.pyplot as plt`) over a native extension. It needs
  Python <!--fact:python.minimum-->3.12 or later and <!--fact:python.dependencies-->numpy, cycler, Pillow, python-dateutil and
  packaging. Wheels: Linux with glibc 2.17 or musl 1.2 (x86_64 and
  aarch64), macOS 10.12 or later (one `universal2` wheel), 64-bit Windows;
  one `cp312-abi3` wheel per platform serves every CPython from 3.12 on.
  The sdist builds with cargo alone.
* **pltrs as matplotlib**, a second distribution named `matplotlib`
  (version <!--fact:alias.version-->`3.11.2+pltrs.0.1.0`), attached to the GitHub release and not on PyPI,
  where the name is matplotlib's: libraries that import `matplotlib`
  (pandas, seaborn, ...) run on pltrs unchanged.
* **pltrs (Rust)**, the crate `pltrs` with matplotlib's model in typed
  Rust: option structs, enums and `Result`s, on crates.io with the eight
  crates it is built from (`pltrs-core`, `pltrs-codec`, `pltrs-font`,
  `pltrs-raster`, `pltrs-platform`, `pltrs-text`, `pltrs-render`,
  `pltrs-ui`). They are released together, at one version, and each
  requires the others at exactly that version.

Imported as `pltrs`, and in Rust, figures and windows take pltrs' own
modern look by default; imported as `matplotlib`, matplotlib's look (the
first import of a process decides, and `PLTRS_LOOK=matplotlib` or
`PLTRS_LOOK=modern` chooses over it). In matplotlib's look the output is
matplotlib's: PNG pixel for pixel, and SVG, PDF, PostScript and PGF
documents drawing the same items.
Interactive figures open in pltrs' own windows (Wayland, X11, macOS,
Windows) or in a browser (WebAgg). pltrs (Python) also runs in Jupyter
(inline, nbAgg, ipympl), with matplotlib's Qt, GTK 3 and 4, Tk, wx and
Cairo backends, and in Sphinx documentation builds (matplotlib's plot
directive, `mathmpl`, `figure-mpl` and roles).

pltrs is MIT OR Apache-2.0. The code it ports from matplotlib and the
libraries under it keeps their permissive licences (`THIRD_PARTY.md`,
`LICENSES/`).

### Where it runs

* Python <!--fact:python.minimum-->3.12 or later: CPython 3.12, 3.13 and 3.14 are tested, through one
  stable-ABI wheel per platform, also with the lowest versions of the
  requirements (numpy 1.26, cycler 0.10, Pillow 10.1, python-dateutil
  2.7, packaging 20.0).
* Linux, x86_64 and aarch64, with glibc 2.17 or later (CentOS 7, Ubuntu
  14.04 and newer) or musl 1.2 or later (Alpine 3.13 and newer); figure
  windows on Wayland and X11.
* macOS 10.12 or later on Intel, 11 or later on Apple silicon, in one
  `universal2` wheel.
* Windows 10 or later, 64-bit (`win_amd64`).
* Other systems and CPUs build from the sdist with Rust 1.99 or later
  (the version `rust-toolchain.toml` names) and nothing else; pltrs
  (Rust) needs the same toolchain.

### What 0.1.0 is not

* Not a stable Rust API: pltrs (Rust) follows matplotlib's model, and
  any 0.x release may change its types and functions. pltrs (Python)'s
  API is matplotlib 3.11's and changes with it.
* Not matplotlib in every byte: in matplotlib's look the figures match
  matplotlib's, but what differs, on purpose or not yet, is listed in
  `DIFFERENCES.md` (below).
* Not a replacement for matplotlib's own wheels on PyPI: the
  distribution named `matplotlib` is attached to the GitHub release
  only.

### How it is measured

* matplotlib 3.11's own test suite, run unchanged against pltrs: all
  of the <!--fact:mpl-suite.all-passing-->10,205 tests that pass under matplotlib on the same
  machine pass.
* The test suites of <!--fact:ecosystem.libraries-->75 libraries built on matplotlib: <!--fact:ecosystem.pltrs-passing-->24,559
  of the <!--fact:ecosystem.reference-passing-->24,559 tests that pass under matplotlib pass under
  pltrs, every one in <!--fact:ecosystem.complete-libraries-->75 of the libraries.
* Jupyter: all <!--fact:notebooks.all-passing-->1,091 notebook comparisons and tests of IPython,
  matplotlib-inline and ipympl that pass under matplotlib.
* A differential tester running matplotlib's gallery, generated programs
  and the programs of <!--fact:corpora-->178 corpora (books, courses, cheat sheets,
  galleries) under both: of the corpora's programs compared whole,
  all but <!--fact:corpora.compared-different-->132 of <!--fact:corpora.compared-->4,102 behave as under matplotlib.
* A conformance spec recorded from matplotlib: about <!--fact:spec.probes-->48,500 probes
  and some <!--fact:spec.figure-cases-->3,400 figures, accounting for every one of matplotlib's
  own <!--fact:mpl-tests-->2,939 tests.
* Speed (`benchmarks/README.md`, on <!--fact:bench.date-->2026-11-18 at
  `<!--fact:bench.commit-->a9cf3e1a`): pltrs (Python) typically <!--fact:bench.python.warm-->4.3× faster than
  matplotlib warm and <!--fact:bench.python.cold-->1.79× cold; pltrs (Rust) <!--fact:bench.rust.warm-->5.3× and <!--fact:bench.rust.cold-->16×.

### Known differences from matplotlib

What pltrs (Python) does otherwise than matplotlib 3.11.2, on purpose or
not yet, is listed by topic in [`DIFFERENCES.md`](DIFFERENCES.md), which
the package carries too (`pltrs/DIFFERENCES.md`); what is not built yet
is under its "Not built". Imported as `matplotlib`, pltrs draws in
matplotlib's look; imported first as `pltrs`, in its modern look.
