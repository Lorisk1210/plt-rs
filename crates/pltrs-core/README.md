# pltrs-core

Geometry, paths, affine transforms and colours shared by every pltrs
crate, mirroring matplotlib's `Affine2D`, `Bbox` and `Path`, with the
float semantics of Python and numpy that matplotlib's results depend on.

* `Point`, `Affine2D` (matplotlib's composition order), `Bbox` and
  `Path`, with matplotlib's path conditioning (NaN removal, clipping,
  snapping, simplification, curve flattening, sketching)
* Bézier curves, hatch patterns and `Rgba` colours
* Python's float formatting (`repr`, `%`, `str.format`) and numpy's
  float semantics, as the binaries pltrs reproduces compute them
* archives (values written to bytes and read back exactly), JSON, and a
  reproducible `SplitMix64` generator

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its geometry and
numerics alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The
code and data it takes from other projects keep their own licences:
`THIRD_PARTY.md` lists them, with their texts in `LICENSES/`.
