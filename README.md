# plt-rs

matplotlib, rebuilt from scratch in Rust: the figure model, every plot
type, text and mathtext, the Agg-exact rasterizer, interactive windows
with their own UI engine, and a conformance suite that checks the output
against matplotlib 3.11. No crates.io dependencies: fonts, PNG/DEFLATE,
rasterization, layout, windowing (X11) and the UI are all implemented
here.

```rust
use plt::figure::Figure;
use plt::numeric::linspace;

let fig = Figure::new();
let ax = fig.subplot();
let x = linspace(0.0, 10.0, 200);
ax.plot(x.clone(), x.iter().map(|x| x.sin()).collect::<Vec<_>>())?;
ax.set_title("A sine");
ax.set_xlabel("x");
fig.savefig("sine.png")?;      // pixel-identical to matplotlib's Agg output
plt::gui::show(&[fig])?;       // or explore it in a window
```

The `pyplot`-style interface works too:

```rust
use plt::pyplot as plt;
plt::plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
plt::title("Squares");
plt::show()?;
```

## What is there

* **Plots**: lines and markers, scatter, bars and histograms, error bars,
  fills, stems, event plots, stack plots, box and violin plots, pies,
  spectral plots, images (`imshow`, `pcolormesh`, `hexbin`, ...), contours
  and triangulations, quiver/barbs/streamplot, polar axes, 3-D axes
  (surfaces, wireframes, 3-D scatter and bars, voxels), annotations, fancy
  boxes and arrows, path effects, tables, colorbars.
* **Axes and layout**: scales, tickers and formatters, dates and
  categories, shared and twin axes, secondary axes, aspect handling,
  subplot grids, mosaics, subfigures, constrained layout, insets.
* **Interactive**: figure windows with a toolbar (home/back/forward,
  pan, zoom-to-rectangle, wheel zoom, save), matplotlib's keyboard
  shortcuts, a live coordinate readout, figure events (`connect`),
  timers and `FuncAnimation` (saved as animated PNG).

## Crates

| crate          | responsibility                                                    |
|----------------|-------------------------------------------------------------------|
| `plt-core`     | points, affine transforms, bounding boxes, paths, colours         |
| `plt-codec`    | CRC32/Adler32, DEFLATE/zlib, PNG and animated PNG                 |
| `plt-font`     | TrueType/OpenType parsing, shaping (GSUB/GPOS), font manager      |
| `plt-raster`   | anti-aliased scanline rasterizer, stroker, dashes, compositing    |
| `plt-text`     | text layout and the mathtext parser and typesetter                |
| `plt-render`   | the `Renderer` trait and the Agg-compatible raster backend        |
| `plt-platform` | windows, input and the event loop (X11; headless for tests)       |
| `plt-ui`       | the retained-mode UI engine used by figure windows                |
| `plt`          | the matplotlib model: figures, axes, artists, pyplot, GUI         |
| `plt-spec`     | the conformance harness                                           |
| `xtask`        | developer automation                                              |

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
cargo run -p plt --example figure_window        # needs an X server
```

The conformance spec in `spec/` was run against matplotlib 3.11 to
produce golden values and images:

```sh
cargo xtask spec                                # probes: op(args) -> value
cargo xtask figures                             # figure structure (limits, ticks, ...)
cargo test -p plt-spec --test figure_pixels     # figure pixels, RMS <= 1.0
cargo xtask render polar/                       # write renderings and diffs to spec/out/
```

See `CLAUDE.md` for the coding conventions.
