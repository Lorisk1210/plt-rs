# Third-party code and data

pltrs uses no other crates, but to reproduce matplotlib's output exactly
it ports the code of the libraries matplotlib is built on, and it ships
some of their fonts and data. Those parts stay under their own licences.
The full texts are in [`LICENSES/`](LICENSES):

* `LICENSES/matplotlib.txt` holds matplotlib's licence followed by the
  licences of the components it bundles.
* `LICENSES/pillow.txt` does the same for Pillow and its image libraries.

## Ported code

| Project | Licence (text) | Ported into |
|---|---|---|
| matplotlib 3.11 | Matplotlib License, PSF-based (`matplotlib.txt`) | the figure model, artists, layout engines, ticker, scales, styles, colormaps, mathtext and backends (`pltrs`, `pltrs-text`, `pltrs-render`), the Python API of pltrs (`python/pltrs`: its signatures and docstrings, with the pure-Python `_api`, `_pylab_helpers`, `_docstring`, `_enums`, `_color_data`, `units`, `category`, `typing`, `cbook`, `_afm` and `_text_helpers` modules, the `_preprocess_data` decorator, `_get_executable_info`, the test decorators, fixtures and markers of `testing`, the sample units of `testing.jpl_units`, and the Python structure of `transforms`, `path`, `scale`, `colors`, `patheffects`, `markers`, `font_manager` (its property tables, `FontProperties`, system font discovery, `get_font` and `afmFontProperty`), the event, renderer, canvas, manager and navigation classes of `backend_bases`, the pure-Python `backend_tools`, `backend_managers`, `widgets`, `animation`, `_animation_data`, `testing.widgets` and `backends` modules (the registry, the template backend and `backend_webagg_core`; the other canvases print through `pltrs`), the Python structure of the toolkits' `axisartist` (`angle_helper`, `grid_finder`, `axislines`, `axis_artist`, `axisline_style`, `grid_helper_curvelinear`, `floating_axes`, `parasite_axes`) and `axes_grid1` (`axes_size`, `axes_divider`, `axes_grid`, `inset_locator`, `anchored_artists`, `parasite_axes`, `mpl_axes`, `axes_rgb`), of `inset` and of `axes._secondary_axes`, `axes_grid1.parasite_axes`' `host_axes` and `axes_grid1.axes_divider.make_axes_area_auto_adjustable`, the pickling of `Figure` (`__getstate__`, `__setstate__`), the simple formatters of `ticker`, the image readers and writer and the array checks of `image`, the projection registry, `mplot3d`'s `_Quaternion` and the mouse handling of its `Axes3D`, the document metadata checks of the PDF, PGF and SVG backends, the `axis()` and `margins()` argument handling of `axes._base`, the `cm` registries, `colorizer`, the multivariate colormaps, `MultiNorm` and `LightSource` of `colors`, the argument checks of `colorbar`, `imshow` and `_image.resample`, `testing.compare` with its converters, the validators and parameter table of `rcsetup`, the `style` package, the version parsing of `__init__`, the classes of `gridspec`, `layout_engine` and `legend_handler`, the argument handling of `legend`, the box geometry and the draggable helpers of `offsetbox`, the property-change callbacks, the `set` signature machinery and the clip path, picking and cursor data methods of `artist`, the `Spines` containers of `spines`, `subplot_mosaic`, `ArtistInspector` and `_blocking_input`, and the date locators, `rrulewrapper`, date formatters and unit converters of `dates`, taken nearly as they are, their numeric work done by `pltrs-python` through `pltrs`), the Type 1 font reader and editor of `_type1font` (`crates/pltrs-font/src/type1`), and `texmanager` and `dviread` (`crates/pltrs-text/src/tex`, with the test files `test.map` and `test.tex` of its `test_dviread.py` in `crates/pltrs-text/tests/fixtures/tex`) |
| matplotlib 3.11, plotting API | Matplotlib License, PSF-based (`matplotlib.txt`) | in `python/pltrs`: the argument handling and checks of the plotting methods of `axes._axes` (`axes/_plotting`), the `container` and `_style_helpers` modules and the dash helpers and `VertexSelector` of `lines`, the hatch validation of `hatch`, the style registry of `patches` (`_patch_styles`), the argument handling of `contour`, `quiver`, `streamplot`, `sankey` and `tri` (`Triangulation`, `triplot`, `tripcolor`), the tick helpers of `projections.polar` (`ThetaFormatter`, `ThetaLocator`, `RadialLocator`) and `PathCollection.legend_elements`, taken nearly as they are, their numeric work done by `pltrs-python` through `pltrs` |
| matplotlib 3.11, text API | Matplotlib License, PSF-based (`matplotlib.txt`) | in `python/pltrs`: `OffsetFrom` and the coordinate checks of `Annotation` (`text`), `Text._char_index_at`, the font lists and `addfont` of `font_manager`, the box nodes of `_mathtext`, the `_tokenize` generator and `Type1Font` interface of `_type1font`, `PsfontsMap` of `dviread`, the `PdfPages` and `_metadata_to_str` of `backend_pgf`, `_create_pdf_info_dict` and `_datetime_to_pdf` of `backend_pdf`, and the subsetting interface of `backends._backend_pdf_ps`, taken nearly as they are, their work done by `pltrs-python` through `pltrs`, `pltrs-text` and `pltrs-font` |
| matplotlib 3.11, axis API | Matplotlib License, PSF-based (`matplotlib.txt`) | in `python/pltrs`: the `Tick`, `XTick`, `YTick` and `Ticker` classes and the keyword handling of `Axis` (`_translate_tick_params`, `set_tick_params`, `grid`, `set_ticks`, `set_ticklabels`, `get_ticklabels`, `_set_converter`, `set_label_text`, the limit warnings of `_set_lim`) in `axis` and `_axis_artists`, `_axis_method_wrapper` and the tick and log-plot methods of `axes` (`grid`, `tick_params`, `ticklabel_format`, `locator_params`, `minorticks_on`, `loglog`, `semilogx`, `semilogy`; `axes/_ticks.py`), `ThetaAxis` and `RadialAxis` of `projections.polar`, the `set_bounds` checks of `spines`, and `xticks`, `yticks`, `rgrids` and `thetagrids` of `pyplot` (`_pyplot_ticks`), taken nearly as they are, their work done by `pltrs-python` through `pltrs` |
| matplotlib 3.11, mplot3d | Matplotlib License, PSF-based (`matplotlib.txt`) | in `python/pltrs/mpl_toolkits/mplot3d`: the argument handling of the plotting methods of `Axes3D` (`voxels`, `contour`, `contourf`, `tricontour`, `tricontourf`, `errorbar`, `stem`, `bar`, `fill_between`, `plot_surface`, `quiver`), and the conversion functions and the `Line3D`, `Line3DCollection`, `Patch3D`, `PathPatch3D`, `Patch3DCollection`, `Poly3DCollection` and `Text3D` classes of `art3d`, taken nearly as they are, their numeric work done by `pltrs-python` through `pltrs` |
| Anti-Grain Geometry 2.4 | Agg 2.4 notice (`agg.txt`) | the rasterizer, stroker, dashes, compositing and image resampling (`pltrs-raster`, `crates/pltrs/src/image`) |
| FreeType 2.14 | FTL or GPL-2.0-or-later; pltrs uses the FTL (`matplotlib.txt`, "FreeType")  | the TrueType bytecode interpreter (`crates/pltrs-font/src/hinting`), the autohinter (`crates/pltrs-font/src/autofit`), glyph loading, the Type 1 driver and the Adobe charstring engine (`crates/pltrs-font/src/type1/face`, `crates/pltrs-font/src/type1/cf2`), and the monochrome rasterizer (`crates/pltrs-render/src/raster/mono`) |
| HarfBuzz 14 | MIT-Modern-Variant (`matplotlib.txt`, "HarfBuzz") | OpenType shaping and layout tables (`crates/pltrs-font/src/shape`, `crates/pltrs-font/src/layout`); the BCP 47 to OpenType language tag table `crates/pltrs-font/src/layout/ot_tags/tables.rs`, generated from HarfBuzz's `hb-ot-tag-table.hh` (itself built from the IANA Language Subtag Registry and Microsoft's OpenType language tag list) |
| libraqm | MIT (`matplotlib.txt`, "libraqm") | text run layout (`crates/pltrs-font/src/shape`, `pltrs-text`) |
| Qhull | Qhull licence (`matplotlib.txt`, "QHull") | Delaunay triangulation (`crates/pltrs/src/tri/delaunay`) |
| ContourPy 1.4 | BSD-3-Clause (`contourpy.txt`); its `mpl2005` algorithm comes from `gcntr.c` of the GIST package, under the Yorick licence (`matplotlib.txt`, "Yorick Colormaps") | the quad contour generators `mpl2005`, `mpl2014` and `serial` (`crates/pltrs/src/contour`) |
| NumPy 2.5 and Intel's x86-simd-sort | BSD-3-Clause (`numpy.txt`) | `argsort` of floats, down to the order of ties (`crates/pltrs/src/numeric/argsort`), and the exact results of the numpy functions matplotlib calls (`interp`, `searchsorted`, `arange`, `linspace`, quantiles and others; `crates/pltrs/src/numeric` and throughout `pltrs`) |
| libstdc++ (GCC 14) | GPL-3.0-or-later with the GCC Runtime Library Exception 3.1; the original STL code under Hewlett-Packard's and Silicon Graphics' permissive notices (`libstdc++.txt`) | `std::sort`, which x86-simd-sort falls back to (`crates/pltrs/src/numeric/argsort/introsort.rs`); see the note below |
| CPython 3.14 | PSF License Agreement (`cpython.txt`) | float formatting (`repr`, `%` and `str.format`, `crates/pltrs-core/src/pyfmt`), `divmod` and `round` (`crates/pltrs/src/ticker/pymath.rs`), and the `datetime` types (`crates/pltrs/src/dates`) |
| dateutil 2.9 | Apache-2.0 and BSD-3-Clause (`dateutil.txt`) | recurrence rules, `relativedelta` and `tzfile` time zones (`crates/pltrs/src/dates/rrule`, `relativedelta.rs`, `tz`) |
| fontTools 4.67 | MIT (`fonttools.txt`) | the glyph closure of font subsetting (`crates/pltrs-font/src/subset`), glyph bounds for Type 42 fonts (`crates/pltrs-render/src/ps/type42.rs`) and the script data of `crates/pltrs-font/src/shape/unicode_data.rs` |
| Pillow 12 | MIT-CMU (`pillow.txt`) | image file reading and writing (`pltrs-codec`, `crates/pltrs/src/image`) |
| libjpeg-turbo / IJG libjpeg | IJG, BSD-3-Clause and zlib (`pillow.txt`, "LIBJPEG") | the JPEG decoder and encoder (`crates/pltrs-codec/src/jpeg`). This software is based in part on the work of the Independent JPEG Group. |
| libtiff | libtiff licence (`pillow.txt`, "LIBTIFF") | TIFF decompression (`crates/pltrs-codec/src/tiff`) |
| libwebp 1.6 | BSD-3-Clause (`pillow.txt`, "LIBWEBP") | WebP decoding: the VP8 and VP8L decoders, alpha planes, chroma upsampling and the container rules (`crates/pltrs-codec/src/webp`) |

The FreeType parts are based in part on the work of the FreeType Team
(https://freetype.org).

`crates/pltrs/src/numeric/argsort/introsort.rs` is the one part under a
copyleft licence. The GCC Runtime Library Exception covers programs
compiled with GCC, not translations of libstdc++'s source, so that file
stays under GPL-3.0-or-later. It only decides the order of ties in
arrays with NaN or in degenerate partitions; replacing it with a
permissively licensed sort would change only that order.

As matplotlib's licence asks, the changes in summary: pltrs rewrites
matplotlib and the code above in Rust. It keeps their algorithms and
output, but replaces their APIs with typed Rust ones, and adds its own UI
engine, windowing layer and conformance suite.

## Bundled fonts and data

| Item | Licence (text) | Location |
|---|---|---|
| DejaVu fonts | Bitstream Vera / DejaVu licence | `crates/pltrs-font/fonts/LICENSE_DEJAVU` |
| STIX fonts | SIL OFL 1.1 | `crates/pltrs-font/fonts/LICENSE_STIX` |
| BaKoMa Computer Modern fonts (`cm*.ttf`) and their metrics | BaKoMa Fonts Licence (`matplotlib.txt`, "BaKoMa Fonts") | `crates/pltrs-font/fonts`, `crates/pltrs-text/src/mathtext/data` |
| Adobe Core 14 font metrics (`*.afm`) | Adobe's notice in `readme.txt`: copy and distribute with the notices kept and the readme alongside | `crates/pltrs-font/fonts/pdfcorefonts` |
| Last Resort HE font | SIL OFL 1.1 (`matplotlib.txt`, "Last Resort fonts") | `crates/pltrs-font/fonts` |
| Inter font (Regular, Italic, SemiBold, Bold, Bold Italic), the typeface of pltrs' modern look  | SIL OFL 1.1 | `crates/pltrs-font/fonts/LICENSE_INTER` |
| Computer Modern Type 1 font `cmr10.pfb` by the American Mathematical Society, a test fixture (from matplotlib's test data) | SIL OFL 1.1 (`amsfonts.txt`) | `crates/pltrs-font/tests/fixtures/type1` (with `LICENSE_AMSFONTS`) |
| Courier 10 Pitch BT Bold Type 1 font `Courier10PitchBT-Bold.pfb` by Bitstream, a test fixture (from matplotlib's test data) | Bitstream's licence for its donated Charter and Courier Type 1 fonts: use, copy, modify and redistribute with the notice kept (`courierten.txt`) | `crates/pltrs-font/tests/fixtures/type1` (with `LICENSE_COURIERTEN`) |
| ColorBrewer colour schemes | Apache-2.0 (`matplotlib.txt`, "ColorBrewer Color Schemes") | `crates/pltrs/src/colors/colormaps` |
| Yorick colormaps (`gist_*`) | Yorick licence (`matplotlib.txt`, "Yorick Colormaps") | `crates/pltrs/src/colors/colormaps` |
| Solarized palette (`Solarize_Light2`) | MIT (`matplotlib.txt`, "Solarized") | `crates/pltrs/src/style` |
| Unicode Character Database 17.0 | Unicode License V3 (`unicode.txt`) | `crates/pltrs-font/src/shape/ucd_tables.rs` |
| matplotlib's WebAgg page, scripts and toolbar icons, the toolbar icons also those of the figure windows in matplotlib's look | Matplotlib License | `crates/pltrs/src/web/assets` (see its `LICENSE`), embedded for the windows by `crates/pltrs/src/gui/images.rs` |
