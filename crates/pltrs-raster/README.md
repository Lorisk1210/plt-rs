# pltrs-raster

`pltrs-raster` draws anti-aliased shapes into pixel buffers. It gives the
same pixels as Anti-Grain Geometry (Agg), which matplotlib uses.

Contents:

* RGBA pixel buffers and gray coverage masks
* The Agg scanline rasterizer, with curve flattening, dashes and strokes
* Compositing with clip boxes, clip masks, hatches, Gouraud triangles and
  markers
* The image resampler of matplotlib, with all interpolation kernels

## Use

Most programs need the crate [`pltrs`](https://crates.io/crates/pltrs).
It has figures, all output formats and windows. Use `pltrs-raster` directly
only if you need the rasterizer alone.

All pltrs crates have the same version. Each crate requires the same
version of the other pltrs crates.

## License

You can use this crate under the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`).

The code and data from other projects keep their licenses. `THIRD_PARTY.md`
lists them, and `LICENSES/` has the license texts.
