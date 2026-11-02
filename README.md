# pltrs

matplotlib, rebuilt from scratch in Rust: the figure model, every plot
type, text and mathtext, the Agg-exact rasterizer, interactive windows
with their own UI engine, and a conformance suite that checks the output
against matplotlib 3.11. No crates.io dependencies: fonts, PNG/DEFLATE,
rasterization, layout, windowing (Wayland and X11) and the UI are all implemented
here.

The Rust crate (`use pltrs::...`) and the Python package
(`import pltrs.pyplot as plt`) share the name pltrs, as the crate and the
package of polars do. Where the language matters, this README says
pltrs (Rust) or pltrs (Python). The crate is built from crates of its
own, `pltrs-core` to `pltrs-ui` (see [Crates](#crates)). Bare `plt` stays
matplotlib's: the name matplotlib users give `pyplot`, which pltrs keeps
for its own `pyplot` in both languages.

```rust,no_run
use pltrs::figure::Figure;
use pltrs::numeric::linspace;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 200);
    ax.plot(&x, x.iter().map(|x| x.sin()))?;
    ax.set_title("A sine");
    ax.set_xlabel("x");
    fig.savefig("sine.png")?; // in pltrs' modern look
    pltrs::gui::show(&[fig])?; // or explore it in a window
    Ok(())
}
```

Figures and windows take pltrs' modern look unless told otherwise:
Inter type, hairline grids behind the data, no top or right spines,
left-aligned titles, constrained layout and a palette checked for
colour-vision deficiencies (the `pltrs-modern` style sheet), in modern
windows. matplotlib's look is one switch away, for the figures, the
windows or both, and is what makes PNG output pixel-identical to
matplotlib's Agg:

```sh
PLTRS_LOOK=matplotlib cargo run ...          # matplotlib's figures and windows
PLTRS_FIGURE_LOOK=matplotlib cargo run ...   # matplotlib's figures, modern windows
PLTRS_FIGURE_LOOK=modern-dark cargo run ...  # the modern look on charcoal
```

or `pltrs::look::set_appearance(...)` in code. The modern look is checked
against matplotlib too: matplotlib drawing the same style sheet with
Inter gives identical figures.

The `pyplot`-style interface works too, with the module named `plt` as in
matplotlib:

```rust,no_run
use pltrs::pyplot as plt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    plt::plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
    plt::title("Squares");
    plt::show()?;
    Ok(())
}
```

Windows open natively: under Wayland or X11 on Linux (`PLTRS_PLATFORM`
picks one), with AppKit on macOS and Win32 on Windows. Where they cannot
open, `show()` shows nothing on Linux (a machine without a display), as
matplotlib falls back to `agg`, and on macOS and Windows serves the
figures to a web browser instead, like matplotlib's WebAgg backend, and
prints their address. `MPLBACKEND=webagg` (or
`plt::switch_backend(Backend::WebAgg)`) picks the browser everywhere (a
remote machine, WSL), and `pltrs::web::serve(&[fig], WebOptions::from_rc()?)`
does it without pyplot.

## New to matplotlib

The `pltrs::tutorial` module of the API documentation (`cargo doc -p pltrs
--open`) teaches plotting with pltrs from the ground up, in Rust terms:
a first plot, the parts of a figure and the handles that reach them,
plot types, data and option structs, styling, layout, text, scales and
dates, output formats, interactive figures, extending pltrs and testing
your figures. Every example in it runs as a doctest.

## Coming from matplotlib

The `pltrs::porting` module of the API documentation (`cargo doc -p pltrs
--open`) is a porting guide: topic by topic, it maps matplotlib's calls
to pltrs (keyword arguments to option structs, strings to enums,
exceptions to `Result`s, `get_xlim()` to `xlim()`) with equivalence
tables and side-by-side examples whose Rust half runs as a doctest.

## What is there

* **Plots**: lines and markers, scatter, bars and histograms, error bars,
  fills, stems, event plots, stack plots, box and violin plots, pies,
  spectral plots, images (`imshow`, `pcolormesh`, `hexbin`, images on
  non-uniform grids, ...), contours and triangulations,
  quiver/barbs/streamplot, Sankey diagrams, polar and geographic axes
  (Aitoff, Hammer, Lambert, Mollweide), 3-D axes (surfaces, wireframes,
  3-D scatter and bars, voxels, data clipped to the view limits with
  `axlim_clip`), annotations, fancy boxes and arrows, path effects, image
  filters on artists (`set_agg_filter`), tables, legends, colorbars;
  matplotlib's colormaps, norms and colour sequences, including the
  multivariate and bivariate colormaps.
* **Text**: TrueType fonts with OpenType shaping, right-to-left and
  Arabic text, font features and BCP 47 languages per text
  (`set_fontfeatures`, `set_language`), every `text.hinting` mode with
  FreeType's own bytecode interpreter and autohinter, text without
  anti-aliasing through FreeType's monochrome rasterizer, and mathtext in
  every matplotlib fontset (DejaVu Sans and Serif, Computer Modern, STIX,
  STIX sans, custom), placing every glyph and rule where matplotlib does
  on its own mathtext test suite. With TeX installed, `usetex` text is
  typeset by LaTeX as in matplotlib (`latex`, or `latex` with `dvipng`):
  pltrs reads the DVI files, virtual fonts, TFM metrics, font maps and
  Type 1 fonts itself, and draws the glyphs in PNG, PDF and SVG; PGF
  output hands the text to LaTeX, and PostScript output refuses it, since
  matplotlib needs `psfrag` and Ghostscript there.
* **Axes and layout**: scales, tickers and formatters, dates and
  categories, shared and twin axes, secondary axes, aspect handling,
  subplot grids, mosaics, subfigures, constrained and tight layout,
  aligned labels and titles, insets, `axes_grid1` (dividers, image
  grids, inset locators and bbox connectors, host and parasite axes,
  RGB axes) and `axisartist` (axis artists styled part by part and
  turned by axis direction, curvilinear grids with angle locators and
  formatters, floating axes and auxiliary axes in curved coordinates).
* **Interactive**: figure windows with a toolbar (home/back/forward,
  pan, zoom-to-rectangle, wheel zoom, subplot configuration, figure
  options, save), a save dialog (PNG, JPEG, TIFF, GIF, WebP, SVG, PDF,
  PS, EPS, PGF), copying the figure to the clipboard, matplotlib's keyboard
  shortcuts with a help window, toolbar tools of your own
  (`Figure::add_tool`), a live coordinate readout with the image value
  under the pointer, 3-D axes that rotate (arcball, trackball, sphere or
  azimuth/elevation), pan and zoom with the mouse, figure events
  (`connect`, including draw events), picking artists with the mouse
  (`set_picker` and pick events), draggable legends, annotations and
  offset boxes, blitting (`FigureCanvas::copy_from_bbox`,
  `restore_region`, `draw_artist`, `blit`, and blitted animations and
  cursors), timers and `FuncAnimation`/`ArtistAnimation` (saved as
  animated PNG, GIF or WebP, as an HTML page with matplotlib's JavaScript
  player, `to_jshtml`, or as a video through an installed ffmpeg,
  `to_html5_video`); pyplot's interactive mode with `pause`, `ginput`
  and `waitforbuttonpress`; the same figures in a web browser
  (matplotlib's WebAgg protocol and client over pltrs's own HTTP and
  WebSocket server): pan, zoom, history, readout, mouse and key events,
  timers and downloads in every output format.
* **Output**: PNG pixel-identical to matplotlib's Agg; JPEG and TIFF
  byte-identical to Pillow's for the same pixels, lossless WebP, GIF and
  raw RGBA; SVG, PDF and PostScript/EPS documents structured like
  matplotlib's, including the 14 core fonts of `pdf.use14corefonts` and
  `ps.useafm`; and PGF pictures for LaTeX (text typeset by LaTeX, laid
  out with pltrs' metrics). `imread` reads PNG, JPEG, TIFF, GIF and WebP (lossy and
  lossless, still or animated) to the pixels matplotlib gets.
* **Widgets**: buttons, sliders and range sliders, check and radio
  buttons, text boxes, crosshair cursors, and span, rectangle, ellipse,
  lasso and polygon selectors.
* **Extending**: artists of your own (the `Artist` trait,
  `Axes::add_custom_artist`), projections (the `Projection` trait,
  registered by name like matplotlib's `register_projection`), legend
  handlers (`LegendHandler`), unit converters for values of your own
  types (`ConversionInterface`, linked to the values by `Convertible`
  like matplotlib's `units.registry`), scales, colormaps and colour
  sequences registered by name, and toolbar tools.
  Removed artists stay usable and can be attached again, as in
  matplotlib.
* **Configuration and testing**: a `pltrsrc` file in matplotlib's
  `matplotlibrc` syntax and user style sheets in the configuration
  directory (`pltrs::config`; `PLTRSCONFIGDIR`, `PLTRSRC`), and
  `pltrs::testing` for testing your own figures, as `matplotlib.testing`
  does: `setup`, `compare_images` with its failure diff, and
  `check_figures_equal`.

## Examples

[`examples/`](examples/README.md) holds twenty common graphs, each
written twice and shown with its image: in Rust with pltrs (Rust), and
in Python with pltrs (Python) in matplotlib's syntax. Among them are
line, scatter, bar, area, pie and polar plots, histograms, box and
violin plots, error bars, an annotated heatmap, contours, streamlines, a
3-D surface, dates, a mosaic dashboard, log and twin axes, annotations,
mathtext and an animation. `cargo xtask examples` renders them and
compares each Rust image with matplotlib drawing the Python script.

## Gallery

`crates/pltrs/examples/gallery/` holds over ninety short, commented
examples grouped like matplotlib's gallery: lines, bars and areas;
statistics; images and meshes; contours and vector fields; colour; text
and annotations; layout; dates, categories and units; 3-D; polar,
geographic and custom projections; the `axes_grid1` and `axisartist`
toolkits; diagrams, widgets and animation; reference sheets of markers,
line styles and hatches; and style sheets.

```sh
cargo run -p pltrs --release --example gallery -- target/gallery
```

writes every figure as a PNG (the animation as an animated PNG), an
`index.html` showing them section by section, and a contact sheet of
each section (`sheet-<section>.png`). The figures take the look in
force: `PLTRS_FIGURE_LOOK=modern-dark` draws the gallery on a dark
background, `PLTRS_LOOK=matplotlib` in matplotlib's look.

## Python

pltrs (Python) is pltrs with matplotlib's syntax: code written for
matplotlib runs on it with only the import changed.

```python
import pltrs.pyplot as plt

fig, ax = plt.subplots()
ax.plot([1, 2, 3], [4, 1, 3], "o--", label="data")
ax.set_title("pltrs")
ax.legend()
fig.savefig("figure.png")
```

It draws in pltrs' modern look by default, as the crate does;
`PLTRS_LOOK=matplotlib` gives matplotlib's look, and with it matplotlib's
output:

```sh
python your_script.py                       # pltrs' modern look
PLTRS_LOOK=matplotlib python your_script.py # matplotlib's look
```

It is a Python package (`python/pltrs`) whose classes keep matplotlib's
names and signatures, over a native extension written in Rust against
CPython's stable ABI, without PyO3 or any crate (`crates/pltrs-python`). It
needs Python 3.12 or later, numpy, cycler, Pillow and python-dateutil.
Build it, and run it from the source tree:

```sh
cargo xtask python setup        # the venvs: .venv-pltrs for pltrs, .venv for matplotlib
cargo xtask python --release    # builds python/pltrs/_native.abi3.so
PYTHONPATH=python .venv-pltrs/bin/python your_script.py
cargo xtask python test         # pltrs's smoke tests
cargo xtask mpl-suite           # matplotlib 3.11's own test suite, run against pltrs
```

### Installing

`pip install .` builds and installs pltrs (Python) from the source tree,
and `pip wheel .` makes a wheel of it. The build backend is the tree's
own (`python/backend`, Python's standard library only): it needs no
package, only Python 3.12+ and cargo (the toolchain
`rust-toolchain.toml` names). It runs `cargo build --release -p
pltrs-python` (as `cargo rustc`, leaving the debug information out) and
packs `python/pltrs` with the library as `pltrs/_native.abi3.so`
(`_native.pyd` on Windows) into a wheel tagged `cp312-abi3-<platform>`:
one wheel for CPython 3.12 and every later version. The platform comes
from what the library was built for, never from the Python running the
build:

| system  | platform tag |
|---------|--------------|
| Linux   | `manylinux_2_<N>_<arch>`, `N` the newest glibc the library's symbols need (build in an old distribution, or a manylinux container, for wheels that install widely); `linux_<arch>` if it needs a library manylinux does not allow |
| macOS   | `macosx_<target>_<arch>` (`arm64`, `x86_64`), the target `MACOSX_DEPLOYMENT_TARGET` or rustc's default (10.12 on x86_64, 11.0 on arm64); `-C target=universal2` builds both architectures and joins them with `lipo` into one `universal2` wheel |
| Windows | `win_amd64`, `win_arm64`, `win32`; the extension links the stable ABI's `python3.dll` without an import library (`raw-dylib`) |

`-C target=<Rust target>` (or `CARGO_BUILD_TARGET`) cross-builds, when
the Rust target and its linker are installed. `python -m build` and uv
work the same way.

### Running pandas, seaborn and other matplotlib code unchanged

Libraries such as pandas and seaborn import `matplotlib` by name. A
second distribution of the tree, named `matplotlib`, makes that name,
`mpl_toolkits` (its three toolkits) and `pylab` aliases of pltrs:
`import matplotlib.pyplot` then gives `pltrs.pyplot` itself.

```sh
pip install .                               # pltrs
pip install . -C distribution=matplotlib    # matplotlib, as pltrs
pip install pandas seaborn                  # they find matplotlib installed
```

Its version is the matplotlib version pltrs implements with pltrs' own
as a local label (`3.11.2+pltrs.0.1.0`), so what requires matplotlib
(`matplotlib>=3.4`, even `matplotlib==3.11.2`) is satisfied by it and
pip fetches no real matplotlib. Being named `matplotlib`, it never shares
`site-packages` with a real matplotlib silently: installing one
uninstalls the other, and pip says so. To go back to the real one,
`pip install --force-reinstall matplotlib` (or `pip uninstall
matplotlib` first). It requires the pltrs of the same version, which pip
finds installed, or among the wheels given with it (`pip install
dist/pltrs-*.whl dist/matplotlib-*.whl`).

`cargo xtask ecosystem` runs pandas' plotting (line, bar, hist, box,
kde, area, scatter, hexbin, pie, the `pandas.plotting` functions) and
seaborn (relplot, displot, catplot, heatmap, clustermap, pairplot,
lmplot, jointplot, the objects interface) on the source tree's pltrs as
matplotlib, and checks each figure against what real matplotlib 3.11.2
draws for the same code (`python/ecosystem`; `cargo xtask ecosystem
setup` makes their venv, and the tests skip where pandas or seaborn is
missing).

pltrs (Python) is young: it covers a first slice of pyplot (figures,
subplots on grids and mosaics, subfigures, layout engines, `plot`,
`scatter`, `bar`, titles and labels, legends, artists made before they are
added, saving as PNG, SVG and PDF), rcParams with rc files and style
sheets, and the image-comparison test tools. Its measure is
matplotlib's own test suite: the share of the tests passing under
matplotlib that pass under pltrs, tracked in `spec/mpl-suite/`. How it is
built, and how to extend it, is in [`python/DESIGN.md`](python/DESIGN.md).

## Crates

| crate            | responsibility                                                    |
|------------------|-------------------------------------------------------------------|
| `pltrs-core`     | points, affine transforms, bounding boxes, paths, colours         |
| `pltrs-codec`    | CRC32/Adler32, SHA-1, SHA-256, DEFLATE/zlib, base64, PNG/APNG; JPEG, TIFF, GIF, WebP read and write |
| `pltrs-font`     | TrueType/OpenType and Type 1 parsing, shaping (GSUB/GPOS), font manager |
| `pltrs-raster`   | anti-aliased scanline rasterizer, stroker, dashes, compositing    |
| `pltrs-text`     | text layout, the mathtext parser and typesetter, and usetex (DVI, VF, TFM, font maps) |
| `pltrs-render`   | the `Renderer` trait, the Agg-compatible raster backend, SVG/PDF/PS/PGF |
| `pltrs-platform` | windows, input and the event loop (Wayland, X11; headless for tests) |
| `pltrs-ui`       | the retained-mode UI engine used by figure windows                |
| `pltrs`          | the matplotlib model: figures, axes, artists, pyplot, GUI, web    |
| `pltrs-spec`     | the conformance harness                                           |
| `pltrs-python`   | pltrs (Python)'s CPython extension (stable ABI, hand-written FFI) |
| `xtask`          | developer automation                                              |

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
cargo run -p pltrs --example figure_window      # needs an X server
cargo run -p pltrs --example web_figure         # an animated figure in a browser
cargo run -p pltrs --example widgets            # sliders and a span selector
```

The conformance spec in `spec/` was run against matplotlib 3.11 to
produce golden values and images: about 41 500 probes of functions and
artist behaviour, and some 3 400 figures, among them ports of
matplotlib's own image tests, of every example in its gallery and of
the figures of its tutorials and user guides. Every one of matplotlib's
2 939 tests is accounted for: ported, or listed with the reason it
cannot apply (`cargo xtask mpl-tests --strict`).
Every figure matches matplotlib's in its structure and pixels; the
SVG, PDF, PostScript and PGF documents of all of them can be compared
with matplotlib's item by item:

```sh
cargo xtask spec                                # probes: op(args) -> value
cargo xtask figures                             # figure structure (limits, ticks, ...)
cargo test -p pltrs-spec --test figure_pixels   # figure pixels, RMS <= 1.0
cargo test -p pltrs-spec --test figure_vectors  # SVG/PDF/PS documents, item by item
cargo xtask render polar/                       # write renderings and diffs to spec/out/
cargo xtask render polar/rose --vectors         # ... with the case's SVG, PDF and PS
cargo xtask vectors [PREFIX...]                 # every case's SVG/PDF/PS/PGF against
                                                # documents matplotlib writes then and there
cargo xtask mpl-tests --strict                  # matplotlib's tests: ported or not applicable
```

The `usetex` cases and probes need TeX on `PATH`; without it they are
skipped, and each check says so.

[`benchmarks/README.md`](benchmarks/README.md) times 62 figures in
matplotlib 3.11, in pltrs (Python), running the same scripts with only the
import changed, and in pltrs (Rust). The figures range in size from a single
line plot to million-point lines, 4000² images and text-heavy layouts.
Each is timed warm (the figure alone, in a running program) and cold (a
fresh process, as a user waits for it), with its peak memory, after a
pixel comparison checks that the three draw the same figure. `cargo
xtask bench --report` regenerates it. At the last run, on a shared
16-core Ryzen 9 9955HX:

* pltrs (Rust) was faster than matplotlib on all 61 comparable
  workloads, warm and cold: 1.23× to 54× faster warm (typically 4.5×
  faster, the geometric mean) and 1.69× to 42× faster cold (typically
  8.6× faster). The thinnest warm margin is a PDF of 10⁵ unsimplified
  points, where compressing the content stream takes most of the time
  in both.
* pltrs (Python) ran 22 of the 61 workloads; the other 39 need parts of
  matplotlib's API it lacks so far. It was faster than matplotlib on all
  22, typically 3.8× faster warm and 3.0× faster cold, and at most 28%
  slower than pltrs (Rust) warm.
* A fresh interpreter importing `matplotlib.pyplot` took 208 ms, and one
  importing `pltrs.pyplot` 68 ms. A single line plot saved as PNG took
  246 ms end to end in matplotlib, 85 ms in pltrs (Python) and 18 ms in
  pltrs (Rust) (26 ms, 5.0 ms and 4.9 ms warm).

See `CLAUDE.md` for the coding conventions.

pltrs ports code from matplotlib, Anti-Grain Geometry, FreeType,
HarfBuzz, libraqm, Qhull, ContourPy, NumPy and x86-simd-sort, CPython,
dateutil, fontTools, Pillow, libjpeg-turbo, libtiff and libwebp (and one
file from libstdc++, under GPL-3.0-or-later), and bundles fonts (including
Inter), colour data and Unicode data; see `THIRD_PARTY.md` for their
licences.
