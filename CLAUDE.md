# plt-rs — working conventions

plt-rs is a from-scratch reimplementation of matplotlib in Rust. It is fully
self-contained: **no crates.io dependencies, ever** (not even dev-dependencies).
Everything — DEFLATE/PNG, TrueType, rasterization, text layout, mathtext,
PDF/SVG/PS writers, the UI engine and the windowing layer — lives in this repo.

## Toolchain

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --workspace
cargo test -p <crate>
cargo clippy -p <crate> --all-targets -- -D warnings
cargo fmt --all
```

The host has no C toolchain. Builds target `x86_64-unknown-linux-musl` and link
with the bundled `rust-lld` (see `.cargo/config.toml` and `~/.cargo/config.toml`).
When several people/agents build at once, use a private target directory to
avoid lock contention: `CARGO_TARGET_DIR=target/agents/<name>`.

## Crate map (dependencies only point downwards)

| crate          | responsibility                                                       |
|----------------|----------------------------------------------------------------------|
| `plt-core`     | `Point`, `Affine2D`, `Bbox`, `Path` (+ path conditioning), `Rgba`    |
| `plt-codec`    | CRC32/Adler32, DEFLATE/zlib, PNG/APNG, JPEG, TIFF, GIF, base64       |
| `plt-font`     | TrueType/OpenType parsing, outlines, metrics, kerning, font manager  |
| `plt-raster`   | AA scanline rasterizer, stroker, dashes, clipping, compositing       |
| `plt-text`     | text layout into glyph runs, mathtext parser + typesetter            |
| `plt-render`   | `Renderer` trait; raster (PNG), SVG, PDF, PS backends                |
| `plt-platform` | windows, input events, event loop (X11 first; pluggable platforms)  |
| `plt-ui`       | retained-mode UI engine: widgets, theme, layout, painting           |
| `plt`          | the matplotlib model: Figure, Axes, artists, ticker, styles, pyplot |
| `plt-spec`     | conformance harness: runs spec cases, compares to goldens           |
| `xtask`        | developer automation (`cargo xtask ...`)                             |

## Rust style — non-negotiable

* Idiomatic, readable, documented Rust. No code smells: no dead code, no
  commented-out code, no `TODO` left behind, no copy-paste duplication, no
  god-functions. Prefer small modules (aim < ~600 lines) with one purpose.
* `#![forbid(unsafe_code)]` in every crate root except `plt-platform`, where
  each `unsafe` block carries a `// SAFETY:` justification.
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

The spec is executed against real matplotlib 3.11 to produce goldens; plt-rs
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
* plt-rs draws in its modern look by default (`plt::look`: the
  `plt-modern` style sheet, Inter, the modern chrome). Parity is with
  matplotlib's look: the spec harness and `plt`'s unit tests use it, and
  `PLT_LOOK=matplotlib` selects it anywhere else (doctests, examples).
* Checking plt-rs against the goldens: `cargo xtask spec [AREA] [--failures]`
  (probes), `cargo xtask figures [GROUP] [--failures]` (structural facts),
  `cargo test -p plt-spec --test figure_pixels` (pixels, RMS ≤ 1.0),
  `cargo test -p plt-spec --test figure_vectors` (SVG/PDF/PS structure),
  `cargo xtask vectors [PREFIX...]` (every matching case's SVG/PDF/PS/PGF
  against documents matplotlib writes for it then and there), and
  `cargo xtask render PREFIX...` to write a case's rendering plus a diff
  against its golden PNG to `spec/out/` for inspection.
* Coverage of matplotlib's own tests: `cargo xtask mpl-tests [--strict]`
  sorts every `def test_*` of `matplotlib/tests` into ported (named or
  cited, with or without `test_`, in `spec/reference` or
  `crates/plt-spec/src`), not applicable (listed in
  `spec/reference/not_applicable.txt` as `test_file.py::test_name:
  reason`) and unaccounted. `--strict` fails on unaccounted tests and on
  ledger lines naming a test that is gone or ported.
* `cargo xtask render --look modern|modern-dark PREFIX...` renders cases
  in plt-rs' own looks instead, to `spec/out/look-<look>/` (no goldens, so
  no diff): the way to review how the modern looks treat a feature.
  Modern-look settings matplotlib has no rcParam for are plt-only `plt.*`
  rcParams (`EXTENSIONS` in `crates/plt/tools/gen_rcparams.py`), whose
  defaults keep matplotlib's look unchanged.
* Performance: `cargo xtask bench [PREFIX...] --reference` times large
  workloads in plt-rs and matplotlib. An optimisation must leave every
  output byte unchanged: compare the PNG/SVG/PDF/PS of all spec cases
  before and after.
* Robustness: `cargo test -p plt --test fuzz` feeds seeded random input
  to the parsers, plotting, output and interaction;
  `cargo test -p plt -p plt-ui -p plt-platform --lib fuzz_tests` fuzzes
  crate-private code (the web server's HTTP/WebSocket/messages, X11
  replies and events, the GUI dialogs and widgets, the config lookup).
  `PLT_FUZZ_SCALE=50` runs longer, and `PLT_FUZZ_REPLAY=<seed>` reruns a
  failure.
  Absurd input must return an error, never panic, hang or exhaust
  memory.

## Git

The lead integrates and commits. Subagents do not commit, do not push, and
only touch the files/crates they were assigned.
