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
fig.savefig("sine.png")?;      // in plt-rs' modern look
plt::gui::show(&[fig])?;       // or explore it in a window
```

Figures and windows take plt-rs' modern look unless told otherwise:
Inter type, hairline grids behind the data, no top or right spines,
left-aligned titles, constrained layout and a palette checked for
colour-vision deficiencies (the `plt-modern` style sheet), in modern
windows. matplotlib's look is one switch away, for the figures, the
windows or both, and is what makes PNG output pixel-identical to
matplotlib's Agg:

```sh
PLT_LOOK=matplotlib cargo run ...          # matplotlib's figures and windows
PLT_FIGURE_LOOK=matplotlib cargo run ...   # matplotlib's figures, modern windows
PLT_FIGURE_LOOK=modern-dark cargo run ...  # the modern look on charcoal
```

or `plt::look::set_appearance(...)` in code. The modern look is checked
against matplotlib too: matplotlib drawing the same style sheet with
Inter gives identical figures.

The `pyplot`-style interface works too:

```rust
use plt::pyplot as plt;
plt::plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
plt::title("Squares");
plt::show()?;
```

On systems without X11 (macOS, Windows), `show()` serves the figures to a
web browser instead, like matplotlib's WebAgg backend, and prints their
address; on an X11 system without a display it shows nothing, as
matplotlib falls back to `agg`. `MPLBACKEND=webagg` (or
`plt::switch_backend(Backend::WebAgg)`) picks the browser everywhere (a
remote machine, WSL), and `plt::web::serve(&[fig], &WebOptions::from_rc()?)`
does it without pyplot.

## Coming from matplotlib

The `plt::porting` module of the API documentation (`cargo doc -p plt
--open`) is a porting guide: topic by topic, it maps matplotlib's calls
to plt-rs (keyword arguments to option structs, strings to enums,
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
  on its own mathtext test suite.
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
  (matplotlib's WebAgg protocol and client over plt-rs's own HTTP and
  WebSocket server): pan, zoom, history, readout, mouse and key events,
  timers and downloads in every output format.
* **Output**: PNG pixel-identical to matplotlib's Agg; JPEG and TIFF
  byte-identical to Pillow's for the same pixels, lossless WebP, GIF and
  raw RGBA; SVG, PDF and PostScript/EPS documents structured like
  matplotlib's, including the 14 core fonts of `pdf.use14corefonts` and
  `ps.useafm`; and PGF pictures for LaTeX (text typeset by LaTeX, laid
  out with plt-rs' metrics). `imread` reads PNG, JPEG, TIFF, GIF and WebP (lossy and
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

## Gallery

`crates/plt/examples/gallery/` holds short, commented examples grouped
like matplotlib's gallery: lines and areas, bars, statistics, images and
fields, text and annotations, layout, projections (polar, geographic,
3-D) and style sheets.

```sh
cargo run -p plt --example gallery -- target/gallery
```

writes every figure as a PNG with an `index.html` showing them all.

## Crates

| crate          | responsibility                                                    |
|----------------|-------------------------------------------------------------------|
| `plt-core`     | points, affine transforms, bounding boxes, paths, colours         |
| `plt-codec`    | CRC32/Adler32, SHA-1, DEFLATE/zlib, base64, PNG/APNG; JPEG, TIFF, GIF, WebP read and write |
| `plt-font`     | TrueType/OpenType parsing, shaping (GSUB/GPOS), font manager      |
| `plt-raster`   | anti-aliased scanline rasterizer, stroker, dashes, compositing    |
| `plt-text`     | text layout and the mathtext parser and typesetter                |
| `plt-render`   | the `Renderer` trait, the Agg-compatible raster backend, SVG/PDF/PS/PGF |
| `plt-platform` | windows, input and the event loop (X11; headless for tests)       |
| `plt-ui`       | the retained-mode UI engine used by figure windows                |
| `plt`          | the matplotlib model: figures, axes, artists, pyplot, GUI, web    |
| `plt-spec`     | the conformance harness                                           |
| `xtask`        | developer automation                                              |

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
cargo run -p plt --example figure_window        # needs an X server
cargo run -p plt --example web_figure           # an animated figure in a browser
cargo run -p plt --example widgets              # sliders and a span selector
```

The conformance spec in `spec/` was run against matplotlib 3.11 to
produce golden values and images: about 31 000 probes of functions and
artist behaviour, and some 3 200 figures, among them ports of
matplotlib's own image tests, of every example in its gallery and of
the figures of its tutorials and user guides.
Every figure matches matplotlib's in its structure and pixels; the
SVG, PDF, PostScript and PGF documents of all of them can be compared
with matplotlib's item by item:

```sh
cargo xtask spec                                # probes: op(args) -> value
cargo xtask figures                             # figure structure (limits, ticks, ...)
cargo test -p plt-spec --test figure_pixels     # figure pixels, RMS <= 1.0
cargo test -p plt-spec --test figure_vectors    # SVG/PDF/PS documents, item by item
cargo xtask render polar/                       # write renderings and diffs to spec/out/
cargo xtask render polar/rose --vectors         # ... with the case's SVG, PDF and PS
cargo xtask vectors [PREFIX...]                 # every case's SVG/PDF/PS/PGF against
                                                # documents matplotlib writes then and there
```

`cargo xtask bench [PREFIX...] --reference` times 33 large workloads
(a million-point line, 10⁵-marker scatters, 1000² meshes and contours,
4000² images, text-heavy layouts, every output format) in plt-rs and in
matplotlib with its Agg backend; plt-rs is as fast or faster on all of
them.

See `CLAUDE.md` for the coding conventions.

plt-rs ports code from matplotlib, Anti-Grain Geometry, FreeType,
HarfBuzz, Qhull, ContourPy, NumPy, CPython, dateutil, fontTools, Pillow,
libjpeg-turbo, libtiff and libwebp (and one file from libstdc++, under
GPL-3.0-or-later), and bundles fonts (including Inter) and colour data;
see `THIRD_PARTY.md` for their licences.
