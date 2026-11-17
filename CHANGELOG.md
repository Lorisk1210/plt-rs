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
  Rust: option structs, enums and `Result`s. The crates are not on
  crates.io yet.

Both draw in pltrs' own modern look by default. In matplotlib's look
(`PLTRS_LOOK=matplotlib`) the output is matplotlib's: PNG pixel for pixel,
and SVG, PDF, PostScript and PGF documents drawing the same items.
Interactive figures open in pltrs' own windows (Wayland, X11, macOS,
Windows) or in a browser (WebAgg). pltrs (Python) also runs in Jupyter
(inline, nbAgg, ipympl) and with matplotlib's Qt, GTK 3 and 4, Tk, wx and
Cairo backends.

pltrs is MIT OR Apache-2.0. The code it ports from matplotlib and the
libraries under it keeps their permissive licences (`THIRD_PARTY.md`,
`LICENSES/`).

### How it is measured

* matplotlib 3.11's own test suite, run unchanged against pltrs: all
  of the <!--fact:mpl-suite.all-passing-->10,191 tests that pass under matplotlib on the same
  machine pass.
* The test suites of <!--fact:ecosystem.libraries-->75 libraries built on matplotlib: <!--fact:ecosystem.pltrs-passing-->24,559
  of the <!--fact:ecosystem.reference-passing-->24,559 tests that pass under matplotlib pass under
  pltrs, every one in <!--fact:ecosystem.complete-libraries-->75 of the libraries.
* Jupyter: all <!--fact:notebooks.all-passing-->1,091 notebook comparisons and tests of IPython,
  matplotlib-inline and ipympl that pass under matplotlib.
* A differential tester running matplotlib's gallery, generated programs
  and the programs of <!--fact:corpora-->178 corpora (books, courses, cheat sheets,
  galleries) under both: of the corpora's programs compared whole,
  all but <!--fact:corpora.compared-different-->130 of <!--fact:corpora.compared-->3,756 behave as under matplotlib.
* A conformance spec recorded from matplotlib: about <!--fact:spec.probes-->48,500 probes
  and some <!--fact:spec.figure-cases-->3,400 figures, accounting for every one of matplotlib's
  own <!--fact:mpl-tests-->2,939 tests.
* Speed (`benchmarks/README.md`, on <!--fact:bench.date-->2026-11-14 at
  `<!--fact:bench.commit-->c06bfe9e+perf9`): pltrs (Python) typically <!--fact:bench.python.warm-->4.4× faster than
  matplotlib warm and <!--fact:bench.python.cold-->1.89× cold; pltrs (Rust) <!--fact:bench.rust.warm-->5.4× and <!--fact:bench.rust.cold-->16×.

### Known differences from matplotlib

What pltrs (Python) does otherwise than matplotlib 3.11.2, on purpose or
not yet. The differential tester's intended differences are in
`spec/diff/allowlist.txt`, `types-allowlist.txt` and
`private-allowlist.txt`, each with its proof.

**Defaults you will notice**

* Figures take pltrs' modern look (Inter, constrained layout, a palette
  checked for colour-vision deficiencies) unless `PLTRS_LOOK=matplotlib`
  is set. `rcdefaults()`, `rcParamsDefault` and `style.use("default")`
  are matplotlib's defaults in either look.
* The interactive backend chosen by default is pltrs' own windows
  (`pltrs`, or `macosx` on macOS), before Qt, GTK, Tk and wx. The backend
  registry lists `pltrs` among matplotlib's backends, and so do the
  errors naming the valid ones.
* `style.available` lists two more sheets, `pltrs-modern` and
  `pltrs-modern-dark`, and the font lists hold Inter.
* The configuration directory is `PLTRSCONFIGDIR` (else the XDG
  directories), and the rc file `pltrsrc`; `MPLCONFIGDIR` and
  `matplotlibrc` files are read too. `get_data_path()` is pltrs' data
  directory there, and `matplotlib_fname()` names its `pltrsrc`.

**Files written**

* PDF and PGF files name pltrs as the software that wrote them.
* An SVG's embedded images can have other ids than matplotlib's: the ids
  are hashed from the images' PNG bytes, which pltrs' encoder writes
  itself. AVIF is saved through Pillow, as matplotlib saves it.
* A pickled figure holds pltrs' own archive of it. pltrs loads the
  pickles pltrs writes, but pickles do not cross between pltrs and
  matplotlib in either direction: loading one fails.

**Objects**

* A figure's `artists`, `lines`, `patches`, `texts` and `legends` are
  list subclasses (`isinstance(fig.lines, list)` holds).
* The classes of matplotlib's C++ modules (`ft2font.FT2Font`, the `_tri`
  classes, `BufferRegion`) are Python classes: `pybind11_object` is not
  among their bases.
* matplotlib's type stubs ship with pltrs. Where stubtest reads pltrs
  otherwise (instance attributes kept by pltrs (Rust) behind a property,
  trees built on first read), reading and assigning behave as matplotlib's.
* Private names libraries use (`ax._get_lines`, `text._color`, ...) work
  as matplotlib's, but for values matplotlib only computes when it draws
  (pltrs computes them at every layout, so they read differently before
  the first draw), integers pltrs (Rust) keeps as floats, and private
  helpers with no state in pltrs (a polar spine's circle, a 3-D
  collection's faces, an axis artist's tick update).
* A program's code is called as often as matplotlib calls it, with two
  exceptions: an offset box of the program's inside a box pltrs lays out
  is measured once per layout (matplotlib asks again in each parent's
  `get_bbox`, `get_offset` and `draw`), and a projection's axis transforms
  are asked once per side rather than once per tick.
* Where matplotlib 3.11.2 raises or crashes because of a bug of its own,
  pltrs draws or answers: `set_mouseover` on a figure,
  `draw_path_collection` with an identity transform and offsets,
  `BoxStyle.register`, an annotation's arrow from a text at a non-finite
  position, a negative `FT2Image` size (a crash in matplotlib), a polar
  line drawn after its radial limits change, a callback that clears its
  Axes while they are drawn.
* `backend_webagg` imports and serves without Tornado, where matplotlib
  raises `RuntimeError`. Without contourpy installed, a contour set's
  `_contour_generator` is pltrs' own object.
* pltrs' windows give a program matplotlib's QtAgg input events, but
  compose dead keys and pass Shift+Tab to the canvas, as matplotlib's GTK
  and Tk backends do.

**Numbers on other machines**

pltrs follows what numpy and matplotlib's wheels compute on the machine
it runs on: numpy's processor level (the order `argsort` leaves ties in),
fused multiply-adds on arm64, Apple's Accelerate. Not followed, and seen
in last bits or in ill-conditioned results only:

* numpy's OpenBLAS kernels, which differ by processor (long sums, LAPACK);
* numpy's SVML maths on Linux with AVX-512, in pltrs (Rust) alone (pltrs
  (Python) asks numpy where a last bit decides a tick or a pixel);
* on Apple silicon, Accelerate's products of shapes other than 3×3 and
  4×4 and its LAPACK, and the sign of rounding noise in numpy's FFT;
* on aarch64 Linux, Gouraud-shaded colours;
* with numpy older than 2.3 on an AVX-512 processor, `argsort` orders
  ties beyond 256 values as numpy 2.5 does.

**Not done yet**

* On musl (Alpine), where Python gives threads other than the main one
  musl's 128 KiB stack (the interpreters of the pypa images), such a
  thread draws, but a draw started inside a draw (a callback saving its
  figure) raises `RecursionError` there; the message names
  `threading.stack_size`.
* The corpora's programs that still differ from matplotlib somewhere are
  listed by `cargo xtask diff --corpus` (`spec/diff/corpus-outcomes.json`).
* Private state a program assigns and pltrs does not draw from, or
  pickle, as matplotlib does (a path assigned to a colorbar's outline
  spine, a legend's `_fontsize`, a formatter's `_format` across a pickle):
  each is listed with its reason in `spec/diff/private-allowlist.txt`.
* Overrides pltrs does not call yet: an unpickled Axes' class
  `_set_lim_and_transforms`, `update_datalim`, `use_sticky_edges`,
  `AxesImage.can_composite`, tick labels under an overridden axis
  transform.
* pltrs (Rust) has no counterpart for what only Python has: `data=`
  keywords, pandas objects, backends of other Python packages, the Sphinx
  plot directive (`spec/reference/not_applicable.txt`).
