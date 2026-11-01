# pltrs — working conventions

pltrs is a from-scratch reimplementation of matplotlib in Rust. It is fully
self-contained: **no crates.io dependencies, ever** (not even dev-dependencies).
Everything — DEFLATE/PNG, TrueType and Type 1, rasterization, text layout,
mathtext, usetex's DVI reader, PDF/SVG/PS/PGF writers, the UI engine, the
windowing layer and the CPython extension of pltrs (Python) — lives in this
repo.

One name, pltrs, for everything that is ours: the crate `pltrs` and the
Python package `pltrs`, the crates `pltrs-core` ... `pltrs-ui`,
`pltrs-python` and `pltrs-spec`, the `PLTRS_*` environment variables, the
`pltrsrc` file and the `pltrs.*` rcParams. Bare `plt` is matplotlib's
pyplot alias, never ours. In prose write pltrs, or pltrs (Rust) and
pltrs (Python) where the language matters.

## Toolchain

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --workspace
cargo test -p <crate>
cargo clippy -p <crate> --all-targets -- -D warnings
cargo fmt --all
```

Builds use the host's gnu target (gcc links them); `.cargo/config.toml` also
configures `x86_64-unknown-linux-musl` with the bundled `rust-lld`, for hosts
without a C toolchain. The extension of pltrs (Python), `pltrs-python` (a
cdylib), needs the gnu target.
When several people/agents build at once, use a private target directory to
avoid lock contention: `CARGO_TARGET_DIR=target/agents/<name>`.

pltrs supports Linux, macOS and Windows. Only Linux can run here, so the
other two are checked by compiling for them (the targets are installed
with rustup) before anything platform-specific is handed over:

```sh
cargo check --workspace --all-targets --target aarch64-apple-darwin
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --target <target> -- -D warnings
```

Platform-specific code lives behind `cfg(target_os = ...)` in the crate
that needs it; nothing assumes Unix outside `cfg(unix)`.

## Crate map (dependencies only point downwards)

| crate            | responsibility                                                       |
|------------------|----------------------------------------------------------------------|
| `pltrs-core`     | `Point`, `Affine2D`, `Bbox`, `Path` (+ path conditioning), `Rgba`, archives (pickling) |
| `pltrs-codec`    | CRC32/Adler32, SHA-1/SHA-256, DEFLATE/zlib, PNG/APNG, JPEG, TIFF, GIF, WebP, base64 |
| `pltrs-font`     | TrueType/OpenType and Type 1 parsing, outlines, metrics, kerning, font manager |
| `pltrs-raster`   | AA scanline rasterizer, stroker, dashes, clipping, compositing       |
| `pltrs-text`     | text layout into glyph runs, mathtext parser + typesetter, usetex    |
| `pltrs-render`   | `Renderer` trait; raster (PNG), SVG, PDF, PS, PGF backends           |
| `pltrs-platform` | windows, input events, event loop (X11, Wayland, macOS, Windows; pluggable platforms) |
| `pltrs-ui`       | retained-mode UI engine: widgets, theme, layout, painting           |
| `pltrs`          | the matplotlib model: Figure, Axes, artists, ticker, styles, pyplot |
| `pltrs-spec`     | conformance harness: runs spec cases, compares to goldens           |
| `pltrs-python`   | pltrs (Python)'s CPython extension (abi3, hand-written FFI)         |
| `xtask`          | developer automation (`cargo xtask ...`)                             |

## Rust style — non-negotiable

* Idiomatic, readable, documented Rust. No code smells: no dead code, no
  commented-out code, no `TODO` left behind, no copy-paste duplication, no
  god-functions. Prefer small modules (aim < ~600 lines) with one purpose.
* `#![forbid(unsafe_code)]` in every crate root. Only two crates lift it,
  with a `// SAFETY:` justification on each `unsafe` block, behind safe
  wrappers:
  - `pltrs-platform` goes to `#![deny(unsafe_code)]` and allows unsafe code
    only in its `sys` modules, the FFI to the operating system:
    `wayland::sys` (`sendmsg`/`recvmsg` to pass file descriptors over the
    compositor's socket, and `memfd_create`), `macos::sys` (the
    Objective-C runtime and AppKit) and `windows::sys` (Win32). X11 is
    spoken over a plain socket.
  - `pltrs-python`, the CPython extension, keeps all of its unsafe code in
    its `ffi` (declarations) and `binding` (safe wrappers) modules.
* Zero warnings under `cargo clippy --all-targets -- -D warnings` (the
  workspace enables `clippy::pedantic`; see `[workspace.lints]`). Don't
  silence lints locally without a one-line reason.
* Every public item has a doc comment. Crate roots have a module overview.
* Library code never panics on user input: return `Result` with a crate error
  enum that implements `std::error::Error` + `Display`. `expect("why")` only
  for true internal invariants. No `unwrap()` outside tests.
* Strong types over primitive obsession (enums instead of strings, newtypes
  for units where it clarifies), iterators over index loops where natural,
  borrowing over cloning.
* Tests live next to the code (`#[cfg(test)] mod tests`) plus `tests/` for
  integration. Test behaviour, including edge cases (NaN, empty, degenerate).
* `cargo fmt` clean. Edition 2024.

## Conformance spec (`spec/`)

The spec is executed against real matplotlib 3.11 to produce goldens; pltrs
must reproduce them.

* `spec/mpl-reference` — run the reference suite against matplotlib
  (`--update-goldens` to regenerate; never hand-edit goldens).
* Probes (`spec/reference/probes/test_<area>.py`) record `op(args) -> value`
  via `ProbeSet(area).probe("op", fn, **args)`; goldens in
  `spec/goldens/probes/<area>.json`. Each area is owned by one module; run the
  whole module when updating its goldens.
* Figure cases (`spec/reference/figures/cases_<group>.py`) register with
  `@case("group/name")` and build into a fresh `Figure`. Goldens: PNG +
  structural facts JSON in `spec/goldens/figures/<group>/<name>.*`.
* Use only deterministic data: formulas, `np.linspace`, or
  `spec_support.rng(seed)` (SplitMix64, mirrored bit-exactly in Rust). Never
  `np.random`, never wall-clock dates, never sample data files.
* The reference configuration is `matplotlib.testing.setup()` plus
  `text.hinting = "none"`.
* pltrs draws in its modern look by default (`pltrs::look`: the
  `pltrs-modern` style sheet, Inter, the modern chrome). Parity is with
  matplotlib's look: the spec harness and `pltrs`'s unit tests use it, and
  `PLTRS_LOOK=matplotlib` selects it anywhere else (doctests, examples).
* Checking pltrs against the goldens: `cargo xtask spec [AREA] [--failures]`
  (probes), `cargo xtask figures [GROUP] [--failures]` (structural facts),
  `cargo test -p pltrs-spec --test figure_pixels` (pixels, RMS ≤ 1.0),
  `cargo test -p pltrs-spec --test figure_vectors` (SVG/PDF/PS structure),
  `cargo xtask vectors [PREFIX...]` (every matching case's SVG/PDF/PS/PGF
  against documents matplotlib writes for it then and there), and
  `cargo xtask render PREFIX...` to write a case's rendering plus a diff
  against its golden PNG to `spec/out/` for inspection.
* Coverage of matplotlib's own tests: `cargo xtask mpl-tests [--strict]`
  sorts every `def test_*` of `matplotlib/tests` into ported (named or
  cited, with or without `test_`, in `spec/reference` or
  `crates/pltrs-spec/src`), not applicable (listed in
  `spec/reference/not_applicable.txt` as `test_file.py::test_name:
  reason`) and unaccounted. `--strict` fails on unaccounted tests and on
  ledger lines naming a test that is gone or ported.
* usetex needs TeX on `PATH` (here TinyTeX:
  `export PATH="$HOME/.TinyTeX/bin/x86_64-linux:$PATH"`). Without it,
  every check skips the `usetex` group and the TeX probes and says so;
  run the checks both ways when changing text or the backends.
* `cargo xtask render --look modern|modern-dark PREFIX...` renders cases
  in pltrs' own looks instead, to `spec/out/look-<look>/` (no goldens, so
  no diff): the way to review how the modern looks treat a feature.
  Modern-look settings matplotlib has no rcParam for are pltrs-only `pltrs.*`
  rcParams (`EXTENSIONS` in `crates/pltrs/tools/gen_rcparams.py`), whose
  defaults keep matplotlib's look unchanged.
* Performance: `cargo xtask bench [PREFIX...] --reference` times large
  workloads in pltrs and matplotlib. An optimisation must leave every
  output byte unchanged: compare the PNG/SVG/PDF/PS of all spec cases
  before and after.
* Robustness: `cargo test -p pltrs --test fuzz` feeds seeded random input
  to the parsers, plotting, output and interaction;
  `cargo test -p pltrs -p pltrs-ui -p pltrs-platform -p pltrs-text --lib fuzz_tests`
  fuzzes crate-private code (the web server's HTTP/WebSocket/messages,
  X11 replies and events, Wayland messages, XKB keymaps and a hostile
  compositor, the GUI dialogs and widgets, the config lookup, the
  DVI/VF/TFM/font-map readers of usetex).
  `PLTRS_FUZZ_SCALE=50` runs longer, and `PLTRS_FUZZ_REPLAY=<seed>` reruns a
  failure.
  Absurd input must return an error, never panic, hang or exhaust
  memory.
* Windowing against real display servers: `pltrs-platform`'s live tests run
  when `$DISPLAY` / `$WAYLAND_DISPLAY` is set and pass trivially
  otherwise. X11: `Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &` then
  `DISPLAY=:117 cargo test -p pltrs-platform`. Wayland: a headless weston
  installed without root, `crates/pltrs-platform/tools/weston.sh install`
  (into `~/.local/weston`) and `weston.sh run &`, then
  `WAYLAND_DISPLAY=wayland-pltrs cargo test -p pltrs-platform wayland`;
  `weston.sh run --debug` lets `weston.sh screenshot` capture the screen.

## pltrs (Python): `python/`, `crates/pltrs-python`

pltrs (Python) is pltrs with matplotlib's syntax (`import pltrs.pyplot as
plt`); its measure is matplotlib 3.11's own test suite run unchanged against
it. The architecture, the decisions that bind contributors and where to add
code are in `python/DESIGN.md`: read it before working on pltrs.

```sh
cargo xtask python setup      # once: .venv-pltrs (no matplotlib), pytest-xdist in .venv
cargo xtask python [--release]  # build the extension into python/pltrs/_native.abi3.so
cargo xtask python test       # the extension plus pltrs's smoke tests (python/tests)
cargo xtask mpl-suite [PATTERN...] [--failures] [--imports]   # matplotlib's suite on pltrs
cargo xtask mpl-suite --reference                           # ... on real matplotlib
```

* `python/` on `PYTHONPATH` is a working install once the extension is
  built (`PYTHONPATH=python .venv-pltrs/bin/python`).
* Two venvs: `.venv` holds real matplotlib (the spec runner and the suite's
  reference run) and `.venv-pltrs` holds numpy, pytest, pytest-xdist,
  pytest-timeout, Pillow and the packages matplotlib's test files import,
  but never matplotlib, so nothing imports the real one by accident.
  Pinned lists: `python/requirements-*.txt`.
* The suite runs in `target/mpl-suite/<mode>/site/`, which lays out the
  package under test with the sdist's test packages inside (`MPL_SDIST`,
  default `target/refs/matplotlib-dl/matplotlib-3.11.2`); in pltrs mode
  `matplotlib`, `mpl_toolkits` and `pylab` load from `python/pltrs`.
* Outcomes are committed: `spec/mpl-suite/reference.json` and
  `spec/mpl-suite/pltrs.json` (per test id). Run both modes with TinyTeX on
  `PATH`, as they were recorded. After merging, rerun the suite rather
  than resolving conflicts in `pltrs.json` by hand.
* Inside `python/pltrs`, modules import each other relatively; the
  toolkits and `pylab.py` import `pltrs` by name (the suite loads them as
  top-level packages).

## Git

The lead integrates and commits. Subagents do not commit, do not push, and
only touch the files/crates they were assigned.
