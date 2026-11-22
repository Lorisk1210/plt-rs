# Contributing to pltrs

This document has the rules and commands for work on pltrs.
[`AGENTS.md`](AGENTS.md) explains what pltrs is and what the developer
wants. Read it first.

pltrs has no crates.io dependencies. Do not add a dependency or a
dev-dependency.

## Names

Use the name pltrs for everything that belongs to this project:

* the crate `pltrs` and the Python package `pltrs`
* the crates `pltrs-core` to `pltrs-ui`, `pltrs-python` and `pltrs-spec`
* the `PLTRS_*` environment variables, the `pltrsrc` file and the
  `pltrs.*` rcParams

The name `plt` is the matplotlib alias for pyplot. Do not use it for
pltrs. In prose, write pltrs. Write pltrs (Rust) or pltrs (Python) where
the language is important.

## Toolchain

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --workspace
cargo test -p <crate>
cargo clippy -p <crate> --all-targets -- -D warnings
cargo fmt --all
```

Builds use the gnu target of the host, and gcc links them. The file
`.cargo/config.toml` also configures `x86_64-unknown-linux-musl` with
the bundled `rust-lld`, for hosts that have no C toolchain. The crate
`pltrs-python` is a cdylib and needs the gnu target.

If more than one person or agent builds at the same time, use a private
target directory: `CARGO_TARGET_DIR=target/agents/<name>`. This prevents
lock contention.

pltrs supports Linux, macOS and Windows. Only Linux runs on this
machine. Before you hand over platform-specific code, compile it for
macOS and Windows. rustup has the targets installed.

```sh
cargo check --workspace --all-targets --target aarch64-apple-darwin
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --target <target> -- -D warnings
```

Put platform-specific code behind `cfg(target_os = ...)` in the crate
that needs it. Code outside `cfg(unix)` must not assume Unix.

## Crate map

A crate depends only on crates above it in this table.

| Crate            | Contents                                                             |
|------------------|----------------------------------------------------------------------|
| `pltrs-core`     | `Point`, `Affine2D`, `Bbox`, `Path` and path conditioning, `Rgba`, archives (pickling) |
| `pltrs-codec`    | CRC32/Adler32, SHA-1/SHA-256, DEFLATE/zlib, PNG/APNG, JPEG, TIFF, GIF, WebP, AV1/AVIF, base64 |
| `pltrs-font`     | TrueType/OpenType and Type 1 parsing, outlines, metrics, kerning, font manager |
| `pltrs-raster`   | Anti-aliased scanline rasterizer, stroker, dashes, clipping, compositing |
| `pltrs-text`     | Text layout into glyph runs, mathtext parser and typesetter, usetex  |
| `pltrs-render`   | `Renderer` trait and the raster (PNG), SVG, PDF, PS and PGF backends |
| `pltrs-platform` | Windows, input events and the event loop for X11, Wayland, macOS and Windows |
| `pltrs-ui`       | Retained-mode UI engine: widgets, theme, layout, painting            |
| `pltrs`          | The matplotlib model: Figure, Axes, artists, ticker, styles, pyplot  |
| `pltrs-spec`     | Conformance harness: runs spec cases and compares them to goldens    |
| `pltrs-python`   | The CPython extension of pltrs (Python): abi3, hand-written FFI      |
| `xtask`          | Developer automation (`cargo xtask ...`)                             |

## Rust style

These rules are mandatory.

* Write idiomatic, readable, documented Rust.
* Do not leave dead code, commented-out code, `TODO` comments or
  copied code. Do not write functions that do many jobs.
* Keep modules small, with one purpose. Aim for less than 600 lines.
* Put `#![forbid(unsafe_code)]` in each crate root. Only two crates are
  exceptions. In these crates, give each `unsafe` block a `// SAFETY:`
  comment and put it behind a safe wrapper.
  * `pltrs-platform` uses `#![deny(unsafe_code)]`. It permits unsafe
    code only in its `sys` modules, which are the FFI to the operating
    system. `wayland::sys` has `sendmsg` and `recvmsg`, which pass file
    descriptors over the compositor socket, and `memfd_create`.
    `macos::sys` has the Objective-C runtime and AppKit. `windows::sys`
    has Win32. The X11 code uses a plain socket and has no unsafe code.
  * `pltrs-python` keeps all unsafe code in its `ffi` module
    (declarations) and its `binding` module (safe wrappers).
* `cargo clippy --all-targets -- -D warnings` must report nothing. The
  workspace enables `clippy::pedantic` in `[workspace.lints]`. If you
  silence a lint locally, give the reason in one line.
* Give each public item a doc comment. Give each crate root a module
  overview.
* Library code must not panic on user input. Return a `Result` with a
  crate error enum that implements `std::error::Error` and `Display`.
* Use `expect("why")` only for internal invariants. Do not use
  `unwrap()` outside tests.
* Use enums, not strings. Use newtypes for units where they make the
  code clearer.
* Use iterators where an index loop is not necessary. Borrow where a
  clone is not necessary.
* Put unit tests next to the code in `#[cfg(test)] mod tests`. Put
  integration tests in `tests/`. Test behavior and edge cases: NaN,
  empty input and degenerate input.
* Keep the code `cargo fmt` clean. The edition is 2024.

## Documentation style

These rules apply to every Markdown file in the repository, and to prose
that a tool writes into one. They follow ASD-STE100 Simplified Technical
English. The reader knows programming and does not know this project.
English is possibly not the reader's first language.

Content:

* Write only what the reader needs to use, build or change pltrs.
* Remove history, task names, measurement dates and the story of how a
  result was found. Git keeps those.
* Give each document one purpose: instructions, reference or explanation.
  Link to another document. Do not repeat it.
* State a fact, a number or an instruction. Remove a sentence that fits
  the documentation of any other project.
* Say first what the thing is. Then say how to use it.

Sentences:

* Put one idea in one sentence. Use at most 20 words in an instruction
  and at most 25 words in a description.
* Put one topic in one paragraph. Use at most 6 sentences.
* Use the active voice and name the actor. Write "pltrs reads the file",
  not "the file is read".
* Use the simple present, the simple past, the simple future or the
  imperative. Do not write "has been", "would have" or "is being".
* Write an instruction as a command, for example "Run `cargo test`."
  Address the reader as "you".
* Put the condition first: "If TeX is not installed, the checks skip the
  `usetex` group."
* Keep the subject, the verb and the articles. Do not write fragments.
  Do not use arrows in place of words.
* Use normal word order: subject, verb, object. Do not write "what pltrs
  does otherwise than matplotlib" or "as matplotlib's does".

Words:

* Use one word for one thing and repeat it. Do not alternate synonyms.
* Use these names: pltrs, pltrs (Rust), pltrs (Python), the modern look,
  the matplotlib look, the `matplotlib` alias package.
* Use the plain word: "use", "help", "show", "start", "because", "make
  sure".
* Use at most three nouns in a row. Do not stack possessives.
* Use American spelling, as the matplotlib API does: "color", "license".
* Do not use marketing words or filler: "robust", "seamless", "powerful",
  "comprehensive", "leverage", "crucial", "simply", "easily", "very",
  "fully", "of its own", "serves as".
* Do not write "not only X but Y". State the point.
* Do not end a sentence with an "-ing" phrase such as ", ensuring that".
  Start a new sentence.

Punctuation and format:

* Do not use semicolons. Do not use dashes between clauses. End the
  sentence.
* Use a colon only before a list, an example or a code block.
* Use parentheses only for a short name or example. Do not put a second
  sentence in parentheses.
* Use a numbered list for steps. Use a bullet list for three or more
  parallel items.
* Use sentence case for headings. Do not bold terms in running text.
* Put commands, file names and identifiers in code font. Put a command
  that the reader runs in a code block.

## Conformance spec (`spec/`)

The spec runs against matplotlib 3.11 and records the results as
goldens. pltrs must reproduce the goldens.

### Goldens

* `spec/mpl-reference` runs the reference suite against matplotlib. Add
  `--update-goldens` to record the goldens again. Do not edit a golden
  by hand.
* The common goldens are the matplotlib results on Linux x86-64.
* On some platforms, the matplotlib output fails the pltrs check against
  a common golden. A check fails if a value is outside the tolerance or
  an image has an RMS above 1. It also fails if a document has a
  different structure. For these cases,
  `spec/goldens/platforms/<target>/` has the platform output under the
  same path.
* The targets are `aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`
  and `x86_64-pc-windows-msvc`. gnu builds for Windows also read the
  `x86_64-pc-windows-msvc` goldens.
* `pltrs-spec` built for a target uses the platform golden first.
* Only `spec/mpl-reference --update-goldens` on that platform writes a
  platform golden. If the common goldens change, run it again there.

### Processor levels

numpy has different copies of its loops for different processors. The
common goldens come from a machine with AVX-512, where numpy runs its
`X86_V4` loops. With AVX2 only (`X86_V3`) or without AVX2 (`X86_V2`),
the numpy function `argsort` puts equal values in a different order.

* pltrs (Rust) follows the processor through `pltrs::numeric::CpuLevel`.
  pltrs (Python) uses the answer from numpy.
* `spec/goldens/platforms/x86_64-v3/` and `x86_64-v2/` have the results
  that differ at those levels. They apply to all x86-64 systems.
* All commands accept `--level v2|v3|v4`. For example, run
  `cargo xtask spec --level v3` on an AVX-512 machine to check the `v3`
  goldens. The option sets `PLTRS_CPU_LEVEL` for pltrs (Rust). For
  programs that run numpy, it sets `NPY_DISABLE_CPU_FEATURES` and
  `OPENBLAS_CORETYPE`.
* To record the goldens of a level, run
  `spec/mpl-reference --update-goldens` with the two numpy variables
  set. For `v3`, use
  `NPY_DISABLE_CPU_FEATURES="X86_V4 AVX512_ICL AVX512_SPR" OPENBLAS_CORETYPE=Haswell`.
  Run the probes with one module in each process.
* Some targets compute different results at a level than Linux does at
  that level. For example, the numpy wheel for Windows sorts arrays that
  contain NaN with `std::sort` from MSVC at `X86_V3`. For these cases,
  `spec/goldens/platforms/<target>+<level>/` has the results, for
  example `x86_64-pc-windows-msvc+x86_64-v3`.
* The lookup order is: target and level, target, level, common.
* `spec/mpl-reference --update-goldens` on such a machine writes the
  target and level goldens. Run it at each level that the machine
  supports, with the variables of that level.

### Probes and figure cases

* A probe records `op(args) -> value`. Probes are in
  `spec/reference/probes/test_<area>.py`. They use
  `ProbeSet(area).probe("op", fn, **args)`. The goldens are in
  `spec/goldens/probes/<area>.json`.
* One module owns each area. To update the goldens of an area, run the
  full module.
* Figure cases are in `spec/reference/figures/cases_<group>.py`. A case
  registers with `@case("group/name")` and draws into a new `Figure`.
  The goldens are a PNG and a JSON file of structural facts in
  `spec/goldens/figures/<group>/<name>.*`.
* Use only deterministic data: formulas, `np.linspace` or
  `spec_support.rng(seed)`. This generator is SplitMix64, and the Rust
  code has an identical one. Do not use `np.random`, wall-clock dates or
  sample data files.
* The reference configuration is `matplotlib.testing.setup()` plus
  `text.hinting = "none"`.

### Looks

pltrs draws in the modern look by default. The modern look is the
`pltrs-modern` style sheet, the Inter font and the modern window chrome.
The module `pltrs::look` selects the look.

Parity with matplotlib applies to the matplotlib look. The spec harness
and the unit tests of `pltrs` use the matplotlib look. For doctests and
examples, set `PLTRS_LOOK=matplotlib`.

Some settings of the modern look have no matplotlib rcParam. They are
pltrs-only `pltrs.*` rcParams, listed in `EXTENSIONS` in
`crates/pltrs/tools/gen_rcparams.py`. Their defaults do not change the
matplotlib look.

`cargo xtask render --look modern|modern-dark PREFIX...` renders cases
in the modern looks to `spec/out/look-<look>/`. These renderings have no
goldens and no diff. Use them to review how the modern looks draw a
feature.

### Checks against the goldens

| Command | Checks |
|---|---|
| `cargo xtask spec [AREA] [--failures]` | Probes |
| `cargo xtask figures [GROUP] [--failures]` | Structural facts of figures |
| `cargo test -p pltrs-spec --test figure_pixels` | Pixels, RMS at most 1.0 |
| `cargo test -p pltrs-spec --test figure_vectors` | Structure of SVG, PDF and PS documents |
| `cargo xtask vectors [PREFIX...]` | SVG, PDF, PS and PGF of each matching case against documents that matplotlib writes during the run |
| `cargo xtask render PREFIX...` | Nothing. It writes the rendering of a case and a diff against its golden PNG to `spec/out/`. |

### Differential tests

`cargo xtask diff` runs programs with matplotlib and with pltrs (Python)
and compares each value, type, exception, warning, printed line and
figure. Figures must have an RMS of at most 1.

```sh
cargo xtask diff [--gallery] [--generated] [PATTERN...] [--failures] [--reduce] [--baseline REPORT]
cargo xtask diff --corpus [NAME...]
```

* `--gallery` runs the matplotlib gallery. `--generated` runs programs
  that `spec/diff/generators` generates from the matplotlib API.
* The report `target/diff/report.md` groups the differences by cause.
* Put intended differences in `spec/diff/allowlist.txt`, with the proof.
* `--corpus` runs programs that users wrote. `spec/diff/corpora.toml`
  lists the corpora. Each corpus is the code of a book, a cheat sheet or
  a gallery at one commit. The list gives its license and SHA-256. git
  fetches the corpora into `target/diff/corpora`. The report counts the clean programs
  of each corpus.

### Signatures and docstrings

`cargo xtask diff --docs [PATTERN...] [--failures] [--strict]` compares
pltrs with matplotlib, module by module. It writes `target/diff/docs.md`.

* For each public callable, it compares `inspect.signature`, `__doc__`
  after the matplotlib interpolation, `__module__` and `__qualname__`.
* For each public module-level name, it compares the type and the value.
  This includes re-exports, constants and modules as attributes.

The docstrings are the matplotlib docstrings. They are in the source and
use the matplotlib `_docstring` machinery.

### Type information

pltrs ships the matplotlib `py.typed` file and type stubs:

* `python/pltrs/**/*.pyi` imports the pltrs modules.
* `python/alias/matplotlib/**/*.pyi` has the matplotlib stubs, plus
  stubs for the matplotlib modules that have none.
* `pyplot.pyi` has the inline types of the matplotlib pyplot.

`python/tools/matplotlib_stubs.py` writes the stubs. Do not write them
by hand. Run it with `--check` after a new matplotlib release.

`cargo xtask diff --types [PATTERN...] [--failures] [--strict]` runs
`mypy.stubtest` with the matplotlib allowlists, against matplotlib and
against pltrs. It uses separate venvs from
`python/requirements-types.txt`. An error that the matplotlib run does
not have is a difference from the public matplotlib API as the stubs
describe it. The report `target/diff/types.md` groups these errors by
kind and module. Put intended differences in
`spec/diff/types-allowlist.txt`, with the proof.

### Private names that libraries use

`spec/diff/private_census.json` is a census of the private matplotlib
attributes, methods and module names that other libraries read, call,
write or override. `spec/diff/private_census.py` finds them with `ast`
in the libraries and corpora installed on this machine.

`cargo xtask diff --private [PATTERN...] [--failures] [--strict]`
measures each row of the census. It uses the scenes of the generators
(built, drawn, unpickled and cleared) and objects made alone. It writes
`target/diff/private.md`.

* Each name must be present and have the matplotlib type and value.
* A write to a name must have the matplotlib effect on the getter, the
  pickle and the drawing.

Put intended differences in `spec/diff/private-allowlist.txt`.
`spec/diff/private_fields.py` regenerates the table of plain matplotlib
fields in `python/pltrs/_artist_private.py`.

### Coverage of the matplotlib tests

`cargo xtask mpl-tests [--strict]` sorts each `def test_*` of
`matplotlib/tests` into three groups:

* Ported. `spec/reference` or `crates/pltrs-spec/src` names or cites the
  test, with or without the `test_` prefix.
* Not applicable. `spec/reference/not_applicable.txt` lists the test as
  `test_file.py::test_name: reason`.
* Unaccounted.

`--strict` fails if a test is unaccounted. It also fails if a line of
the ledger names a test that no longer exists or that is ported.

### usetex

usetex needs TeX on `PATH`. On this machine, TeX is TinyTeX:
`export PATH="$HOME/.TinyTeX/bin/x86_64-linux:$PATH"`.

If TeX is not on `PATH`, each check skips the `usetex` group and the TeX
probes and reports this. When you change text code or a backend, run the
checks with TeX and without TeX.

### Performance

`cargo xtask bench [PREFIX...] --reference` measures large workloads in
pltrs and in matplotlib.

An optimization must not change one byte of output. Compare the PNG,
SVG, PDF and PS output of all spec cases before and after the change.

### Robustness

Invalid input must return an error. It must not cause a panic, a hang
or memory exhaustion.

* `cargo test -p pltrs --test fuzz` gives seeded random input to the
  parsers, plotting, output and interaction.
* `cargo test -p pltrs -p pltrs-ui -p pltrs-platform -p pltrs-text --lib fuzz_tests`
  fuzzes crate-private code:
  * the HTTP, WebSocket and message code of the web server
  * X11 replies and events, Wayland messages and XKB keymaps
  * a hostile compositor
  * the GUI dialogs and widgets, and the config lookup
  * the DVI, VF, TFM and font-map readers of usetex
* `PLTRS_FUZZ_SCALE=50` makes a run longer. `PLTRS_FUZZ_REPLAY=<seed>`
  runs a failure again.
* pltrs (Python) has a separate fuzzer in `python/tests/fuzz`. It runs
  random programs of the public API and mutated programs of the
  differential tester. It is part of `cargo xtask python test`. Run it
  alone with
  `PYTHONPATH=python:python/tests .venv-pltrs/bin/python -m fuzz --scale 50`.
  It accepts the same variables, and `PLTRS_FUZZ_FIRST=<seed>` for new
  seeds. It reduces each bug to a program in
  `target/python-fuzz/failures/`.

### Window tests with real display servers

The live tests of `pltrs-platform` run when `$DISPLAY` or
`$WAYLAND_DISPLAY` is set. If neither is set, they pass without doing
anything.

X11:

```sh
Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &
DISPLAY=:117 cargo test -p pltrs-platform
```

Wayland uses a headless weston that needs no root. Install it one time
into `~/.local/weston`:

```sh
crates/pltrs-platform/tools/weston.sh install
crates/pltrs-platform/tools/weston.sh run &
WAYLAND_DISPLAY=wayland-pltrs cargo test -p pltrs-platform wayland
```

With `weston.sh run --debug`, the command `weston.sh screenshot`
captures the screen.

macOS: the AppKit tests open windows on the screen, set the Dock icon
and replace the clipboard. They run only with `PLTRS_MACOS_LIVE=1`
(`cargo test -p pltrs-platform --test macos`). Run them only when the
user is at the Mac, and ask the user first.

`python/tests/test_gui_events.py` compares the input that programs see
in pltrs windows with the input in matplotlib QtAgg windows. It uses a
private Xvfb and a weston nested in it. It is part of
`cargo xtask python test`. From `python/tests`,
`.venv/bin/python -m gui_events [SCENARIO...]` runs the two side by
side. `--record` records the QtAgg goldens.

## pltrs (Python): `python/`, `crates/pltrs-python`

pltrs (Python) is pltrs with the matplotlib syntax
(`import pltrs.pyplot as plt`). Its measure is the test suite of
matplotlib 3.11, which runs against it without changes.

Read `python/DESIGN.md` before you work on pltrs (Python). It has the
architecture, the decisions that contributors must follow, and where to
add code.

```sh
cargo xtask python setup      # once: .venv-pltrs (no matplotlib), pytest-xdist in .venv
cargo xtask python [--release]  # build the extension into python/pltrs/_native.abi3.so
cargo xtask python test       # the extension plus the smoke tests of pltrs (python/tests)
cargo xtask mpl-suite [PATTERN...] [--failures] [--imports]   # the matplotlib suite on pltrs
cargo xtask mpl-suite --reference                           # the matplotlib suite on matplotlib
cargo xtask ecosystem [setup]   # pandas and seaborn on pltrs as matplotlib (setup: venvs, sources, baselines)
cargo xtask ecosystem suite [--reference] [PATTERN...] [--failures]  # the test suites and image tests of the libraries
cargo xtask notebooks [setup] [PATTERN...] [--failures]   # spec/notebooks in Jupyter, with both, compared
cargo xtask notebooks suite [--reference] [PATTERN...] [--failures]  # the tests of IPython, matplotlib-inline and ipympl
pip install . [-C distribution=matplotlib]   # pltrs, or matplotlib as pltrs
```

### Environments

* After you build the extension, `python/` on `PYTHONPATH` is a working
  install: `PYTHONPATH=python .venv-pltrs/bin/python`.
* `.venv` has matplotlib. The spec runner and the reference run of the
  suite use it.
* `.venv-pltrs` has numpy, pytest, pytest-xdist, pytest-timeout, Pillow
  and the packages that the matplotlib test files import. It must not
  have matplotlib, so that no program imports matplotlib by accident.
* Both venvs get `python/requirements-suite.txt` (pandas, tornado, the
  Qt bindings), so that both runs reach the same tests.
* The pinned lists are `python/requirements-*.txt`. Run the setup
  command again to bring the venvs up to them.

### The matplotlib suite

* The suite runs in `target/mpl-suite/<mode>/site/`. This directory has
  the package under test, with the test packages of the sdist inside.
  `MPL_SDIST` selects the sdist. Its default is
  `target/refs/matplotlib-dl/matplotlib-3.11.2`.
* In pltrs mode, `matplotlib`, `mpl_toolkits` and `pylab` load from
  `python/pltrs`.
* The outcomes of each test id are committed:
  `spec/mpl-suite/reference.json` and `spec/mpl-suite/pltrs.json`.
* Run both modes with the same tools on `PATH` as the recorded runs:
  `export PATH="$HOME/.local/mpl-suite/inkscape/bin:$HOME/.local/gs/bin:$HOME/.TinyTeX/bin/x86_64-linux:$PATH"`.
  Inkscape rasterizes the SVG comparisons. Ghostscript rasterizes the
  PDF and EPS comparisons.
* `python/tools/suite-tools.sh install` installs Inkscape and the fonts
  and TeX packages that the tests need into `~/.local/mpl-suite`. No
  other program looks there. The runner gives both modes the fonts and
  the TeX tree (`~/.local/mpl-suite/environment`). It reports what a run
  does not have.
* After a merge, run the suite again. Do not resolve conflicts in
  `pltrs.json` by hand.

### GUI backend tests

The tests of the GUI backends need the toolkits and a display.

* Run `python/tools/toolkits.sh install` one time. It unpacks the Debian
  packages of GTK 3 and 4, PyGObject, pycairo, cairocffi, wxPython and
  the X libraries of Qt into `~/.local/gtk`. It needs no root.
* `cargo xtask mpl-suite` then uses the toolkits. If `$DISPLAY` is not
  set, it starts a private Xvfb and stops it by its PID after the run.
* The command reports what it does not have. The tests that need a
  missing part skip.

### Imports

Inside `python/pltrs`, modules import each other with relative imports.
The toolkits and `pylab.py` import `pltrs` by name, because the suite
loads them as top-level packages.

## Git

The lead integrates and commits. Subagents do not commit and do not
push. They change only the files and crates that the lead assigned to
them.
