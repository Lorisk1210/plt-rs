# Third-party code and data in pltrs-raster

<!-- Written by python/tools/crate_notices.py from the repository's THIRD_PARTY.md: edit that file, then run the tool. -->

pltrs-raster is part of pltrs, licensed under the MIT licence or the
Apache License 2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`).
The parts of it listed below come from other projects and keep their own
licences, whose texts are in `LICENSES/` (or beside the files they
cover). These are the rows of pltrs' `THIRD_PARTY.md` that concern this
crate; paths are those of pltrs' repository.

## Ported code

| Project | Licence (text) | Ported into |
|---|---|---|
| Anti-Grain Geometry 2.4 | Agg 2.4 notice (`agg.txt`) | the rasterizer, stroker, dashes, compositing and image resampling (`pltrs-raster`, `crates/pltrs/src/image`) |
| FreeType 2.14 | FTL or GPL-2.0-or-later; pltrs takes the FTL (`matplotlib.txt`, "FreeType") | the TrueType bytecode interpreter and glyph loader (`crates/pltrs-font/src/hinting`), the autohinter (`crates/pltrs-font/src/autofit`), the CFF driver (`crates/pltrs-font/src/cff`), font variations (`crates/pltrs-font/src/var`), the Type 1 driver (`crates/pltrs-font/src/type1/face`), the Adobe charstring engine (`crates/pltrs-font/src/psaux`), the fixed-point arithmetic, scaling, font matrices and outline decomposition (`crates/pltrs-font/src`: `fixed.rs`, `scale.rs`, `matrix.rs`, `outline.rs`), the smooth rasterizer (`crates/pltrs-raster/src/raster/grays.rs`, with its outline flattening in `crates/pltrs-render/src/raster/glyph.rs`) and the monochrome rasterizer (`crates/pltrs-render/src/raster/mono`) |

The FreeType parts are based in part on the work of the FreeType Team
(https://freetype.org).

As matplotlib's licence asks, the changes in summary: pltrs rewrites
matplotlib and the code above in Rust. It keeps their algorithms and
output, but replaces their APIs with typed Rust ones, and adds its own UI
engine, windowing layer and conformance suite.
