# Third-party code and data in pltrs-raster

<!-- Written by python/tools/crate_notices.py from the repository's THIRD_PARTY.md: edit that file, then run the tool. -->

pltrs-raster is part of pltrs. pltrs has two licenses, and you can use
it under either one: the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`). The parts in the tables come from other
projects and keep their licenses. The license texts are in `LICENSES/`,
or next to the files that they apply to. The tables are the rows of
`THIRD_PARTY.md` in the pltrs repository that apply to this crate. The
paths are paths in the pltrs repository.

## Ported code

| Project | License (text) | Ported into |
|---|---|---|
| Anti-Grain Geometry 2.4 | Agg 2.4 notice (`agg.txt`) | the rasterizer, stroker, dashes, compositing and image resampling (`pltrs-raster`, `crates/pltrs/src/image`) |
| FreeType 2.14 | FTL or GPL-2.0-or-later; pltrs takes the FTL (`matplotlib.txt`, "FreeType") | the TrueType bytecode interpreter and glyph loader (`crates/pltrs-font/src/hinting`), the autohinter (`crates/pltrs-font/src/autofit`), the CFF driver (`crates/pltrs-font/src/cff`), font variations (`crates/pltrs-font/src/var`), the Type 1 driver (`crates/pltrs-font/src/type1/face`), the Adobe charstring engine (`crates/pltrs-font/src/psaux`), the fixed-point arithmetic, scaling, font matrices and outline decomposition (`crates/pltrs-font/src`: `fixed.rs`, `scale.rs`, `matrix.rs`, `outline.rs`), the smooth rasterizer (`crates/pltrs-raster/src/raster/grays.rs`, with its outline flattening in `crates/pltrs-render/src/raster/glyph.rs`) and the monochrome rasterizer (`crates/pltrs-render/src/raster/mono`) |

The FreeType parts are based in part on the work of the FreeType Team
(https://freetype.org).

The matplotlib license requires a summary of the changes. pltrs
rewrites matplotlib and the code in the table in Rust. It keeps the
algorithms and the output. It replaces the APIs with typed Rust APIs. It
adds a user interface, window code and conformance tests.
