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
| matplotlib 3.11 | Matplotlib License, PSF-based (`matplotlib.txt`) | the figure model, artists, layout engines, ticker, scales, styles, colormaps, mathtext and backends (`plt`, `plt-text`, `plt-render`) |
| Anti-Grain Geometry 2.4 | Agg 2.4 notice (`agg.txt`) | the rasterizer, stroker, dashes, compositing and image resampling (`plt-raster`, `crates/plt/src/image`) |
| FreeType 2.14 | FTL or GPL-2.0-or-later; plt-rs uses the FTL (`matplotlib.txt`, "FreeType") | the TrueType bytecode interpreter (`crates/plt-font/src/hinting`), the autohinter (`crates/plt-font/src/autofit`) and glyph loading |
| HarfBuzz 14 | MIT-Modern-Variant (`matplotlib.txt`, "HarfBuzz") | OpenType shaping and layout tables (`crates/plt-font/src/shape`, `crates/plt-font/src/layout`); the BCP 47 to OpenType language tag table `crates/plt-font/src/layout/ot_tags/tables.rs`, generated from HarfBuzz's `hb-ot-tag-table.hh` (itself built from the IANA Language Subtag Registry and Microsoft's OpenType language tag list) |
| libraqm | MIT (`matplotlib.txt`, "libraqm") | text run layout (`crates/plt-font/src/shape`, `plt-text`) |
| Qhull | Qhull licence (`matplotlib.txt`, "QHull") | Delaunay triangulation (`crates/plt/src/tri/delaunay`) |
| Pillow 12 | MIT-CMU (`pillow.txt`) | image file reading and writing (`plt-codec`, `crates/plt/src/image`) |
| libjpeg-turbo / IJG libjpeg | IJG, BSD-3-Clause and zlib (`pillow.txt`, "LIBJPEG") | the JPEG decoder and encoder (`crates/plt-codec/src/jpeg`). This software is based in part on the work of the Independent JPEG Group. |
| libtiff | libtiff licence (`pillow.txt`, "LIBTIFF") | TIFF decompression (`crates/plt-codec/src/tiff`) |

The FreeType parts are based in part on the work of the FreeType Team
(https://freetype.org).

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
| ColorBrewer colour schemes | Apache-2.0 (`matplotlib.txt`, "ColorBrewer Color Schemes") | `crates/plt/src/colors/colormaps` |
| Yorick colormaps (`gist_*`) | Yorick licence (`matplotlib.txt`, "Yorick Colormaps") | `crates/plt/src/colors/colormaps` |
| Solarized palette (`Solarize_Light2`) | MIT (`matplotlib.txt`, "Solarized") | `crates/plt/src/style` |
| Unicode Character Database 17.0 | Unicode License V3 (`unicode.txt`) | `crates/plt-font/src/shape/ucd_tables.rs` |
| matplotlib's WebAgg page, scripts and toolbar icons | Matplotlib License | `crates/plt/src/web/assets` (see its `LICENSE`) |
