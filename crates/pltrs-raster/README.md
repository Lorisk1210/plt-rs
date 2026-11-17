# pltrs-raster

An anti-aliased scanline rasterizer, stroker and compositor reproducing
the parts of Anti-Grain Geometry (Agg) matplotlib's Agg backend uses,
pixel for pixel.

* RGBA pixmaps with straight alpha and gray coverage masks
* Agg's scanline rasterizer, curve flattening, dashing and stroking
* compositing with clip boxes and masks, hatches, Gouraud triangles and
  marker stamps
* matplotlib's image resampler and its interpolation kernels

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its rasterizer
alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The
code and data it takes from other projects keep their own licences:
`THIRD_PARTY.md` lists them, with their texts in `LICENSES/`.
