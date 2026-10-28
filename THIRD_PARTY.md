# Third-party code and data

plt-rs uses no other crates, but to reproduce matplotlib's output exactly
it ports the code of the libraries matplotlib is built on, and it ships
some of their fonts and data. Those parts stay under their own licences.
The full texts are in [`LICENSES/`](LICENSES):

* `LICENSES/matplotlib.txt` holds matplotlib's licence followed by the
  licences of the components it bundles.
* `LICENSES/pillow.txt` does the same for Pillow and its image libraries.

## Ported code

| Project | Licence (text) | Ported into |
|---|---|---|
| matplotlib 3.11 | Matplotlib License, PSF-based (`matplotlib.txt`) | the figure model, artists, layout engines, ticker, scales, styles, colormaps, mathtext and backends (`plt`, `plt-text`, `plt-render`), the Type 1 font reader and editor of `_type1font` (`crates/plt-font/src/type1`), and `texmanager` and `dviread` (`crates/plt-text/src/tex`, with the test files `test.map` and `test.tex` of its `test_dviread.py` in `crates/plt-text/tests/fixtures/tex`) |
| Anti-Grain Geometry 2.4 | Agg 2.4 notice (`agg.txt`) | the rasterizer, stroker, dashes, compositing and image resampling (`plt-raster`, `crates/plt/src/image`) |
| FreeType 2.14 | FTL or GPL-2.0-or-later; plt-rs uses the FTL (`matplotlib.txt`, "FreeType") | the TrueType bytecode interpreter (`crates/plt-font/src/hinting`), the autohinter (`crates/plt-font/src/autofit`), glyph loading, the Type 1 driver and the Adobe charstring engine (`crates/plt-font/src/type1/face`, `crates/plt-font/src/type1/cf2`), and the monochrome rasterizer (`crates/plt-render/src/raster/mono`) |
| HarfBuzz 14 | MIT-Modern-Variant (`matplotlib.txt`, "HarfBuzz") | OpenType shaping and layout tables (`crates/plt-font/src/shape`, `crates/plt-font/src/layout`); the BCP 47 to OpenType language tag table `crates/plt-font/src/layout/ot_tags/tables.rs`, generated from HarfBuzz's `hb-ot-tag-table.hh` (itself built from the IANA Language Subtag Registry and Microsoft's OpenType language tag list) |
| libraqm | MIT (`matplotlib.txt`, "libraqm") | text run layout (`crates/plt-font/src/shape`, `plt-text`) |
| Qhull | Qhull licence (`matplotlib.txt`, "QHull") | Delaunay triangulation (`crates/plt/src/tri/delaunay`) |
| ContourPy 1.4 | BSD-3-Clause (`contourpy.txt`); its `mpl2005` algorithm comes from `gcntr.c` of the GIST package, under the Yorick licence (`matplotlib.txt`, "Yorick Colormaps") | the quad contour generators `mpl2005`, `mpl2014` and `serial` (`crates/plt/src/contour`) |
| NumPy 2.5 and Intel's x86-simd-sort | BSD-3-Clause (`numpy.txt`) | `argsort` of floats, down to the order of ties (`crates/plt/src/numeric/argsort`), and the exact results of the numpy functions matplotlib calls (`interp`, `searchsorted`, `arange`, `linspace`, quantiles and others; `crates/plt/src/numeric` and throughout `plt`) |
| libstdc++ (GCC 14) | GPL-3.0-or-later with the GCC Runtime Library Exception 3.1; the original STL code under Hewlett-Packard's and Silicon Graphics' permissive notices (`libstdc++.txt`) | `std::sort`, which x86-simd-sort falls back to (`crates/plt/src/numeric/argsort/introsort.rs`); see the note below |
| CPython 3.14 | PSF License Agreement (`cpython.txt`) | float formatting (`repr`, `%` and `str.format`, `crates/plt/src/ticker/pyfmt`), `divmod` and `round` (`crates/plt/src/ticker/pymath.rs`), and the `datetime` types (`crates/plt/src/dates`) |
| dateutil 2.9 | Apache-2.0 and BSD-3-Clause (`dateutil.txt`) | recurrence rules, `relativedelta` and `tzfile` time zones (`crates/plt/src/dates/rrule`, `relativedelta.rs`, `tz`) |
| fontTools 4.67 | MIT (`fonttools.txt`) | the glyph closure of font subsetting (`crates/plt-font/src/subset`), glyph bounds for Type 42 fonts (`crates/plt-render/src/ps/type42.rs`) and the script data of `crates/plt-font/src/shape/unicode_data.rs` |
| Pillow 12 | MIT-CMU (`pillow.txt`) | image file reading and writing (`plt-codec`, `crates/plt/src/image`) |
| libjpeg-turbo / IJG libjpeg | IJG, BSD-3-Clause and zlib (`pillow.txt`, "LIBJPEG") | the JPEG decoder and encoder (`crates/plt-codec/src/jpeg`). This software is based in part on the work of the Independent JPEG Group. |
| libtiff | libtiff licence (`pillow.txt`, "LIBTIFF") | TIFF decompression (`crates/plt-codec/src/tiff`) |
| libwebp 1.6 | BSD-3-Clause (`pillow.txt`, "LIBWEBP") | WebP decoding: the VP8 and VP8L decoders, alpha planes, chroma upsampling and the container rules (`crates/plt-codec/src/webp`) |

The FreeType parts are based in part on the work of the FreeType Team
(https://freetype.org).

`crates/plt/src/numeric/argsort/introsort.rs` is the one part under a
copyleft licence. The GCC Runtime Library Exception covers programs
compiled with GCC, not translations of libstdc++'s source, so that file
stays under GPL-3.0-or-later. It only decides the order of ties in
arrays with NaN or in degenerate partitions; replacing it with a
permissively licensed sort would change only that order.

As matplotlib's licence asks, the changes in summary: plt-rs rewrites
matplotlib and the code above in Rust. It keeps their algorithms and
output, but replaces their APIs with typed Rust ones, and adds its own UI
engine, windowing layer and conformance suite.

## Bundled fonts and data

| Item | Licence (text) | Location |
|---|---|---|
| DejaVu fonts | Bitstream Vera / DejaVu licence | `crates/plt-font/fonts/LICENSE_DEJAVU` |
| STIX fonts | SIL OFL 1.1 | `crates/plt-font/fonts/LICENSE_STIX` |
| BaKoMa Computer Modern fonts (`cm*.ttf`) and their metrics | BaKoMa Fonts Licence (`matplotlib.txt`, "BaKoMa Fonts") | `crates/plt-font/fonts`, `crates/plt-text/src/mathtext/data` |
| Adobe Core 14 font metrics (`*.afm`) | Adobe's notice in `readme.txt`: copy and distribute with the notices kept and the readme alongside | `crates/plt-font/fonts/pdfcorefonts` |
| Last Resort HE font | SIL OFL 1.1 (`matplotlib.txt`, "Last Resort fonts") | `crates/plt-font/fonts` |
| Inter font (Regular, Italic, SemiBold, Bold, Bold Italic), the typeface of plt-rs' modern look | SIL OFL 1.1 | `crates/plt-font/fonts/LICENSE_INTER` |
| Computer Modern Type 1 font `cmr10.pfb` by the American Mathematical Society, a test fixture (from matplotlib's test data) | SIL OFL 1.1 (`amsfonts.txt`) | `crates/plt-font/tests/fixtures/type1` (with `LICENSE_AMSFONTS`) |
| Courier 10 Pitch BT Bold Type 1 font `Courier10PitchBT-Bold.pfb` by Bitstream, a test fixture (from matplotlib's test data) | Bitstream's licence for its donated Charter and Courier Type 1 fonts: use, copy, modify and redistribute with the notice kept (`courierten.txt`) | `crates/plt-font/tests/fixtures/type1` (with `LICENSE_COURIERTEN`) |
| ColorBrewer colour schemes | Apache-2.0 (`matplotlib.txt`, "ColorBrewer Color Schemes") | `crates/plt/src/colors/colormaps` |
| Yorick colormaps (`gist_*`) | Yorick licence (`matplotlib.txt`, "Yorick Colormaps") | `crates/plt/src/colors/colormaps` |
| Solarized palette (`Solarize_Light2`) | MIT (`matplotlib.txt`, "Solarized") | `crates/plt/src/style` |
| Unicode Character Database 17.0 | Unicode License V3 (`unicode.txt`) | `crates/plt-font/src/shape/ucd_tables.rs` |
| matplotlib's WebAgg page, scripts and toolbar icons, the toolbar icons also those of the figure windows in matplotlib's look | Matplotlib License | `crates/plt/src/web/assets` (see its `LICENSE`), embedded for the windows by `crates/plt/src/gui/images.rs` |
