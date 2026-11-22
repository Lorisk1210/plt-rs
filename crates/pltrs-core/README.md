# pltrs-core

`pltrs-core` has the geometry and number types that all pltrs crates use.

Contents:

* `Point`, `Affine2D`, `Bbox` and `Path`, with the same results as the
  matplotlib classes
* The path operations of matplotlib: NaN removal, clipping, snapping,
  simplification, curve flattening and sketching
* Bézier curves, hatch patterns and `Rgba` colors
* Number formatting with the same output as Python (`repr`, `%` and
  `str.format`), and float operations with the same results as numpy
* Archives that write values to bytes and read them back, JSON, and the
  random number generator `SplitMix64`

## Use

Most programs need the crate [`pltrs`](https://crates.io/crates/pltrs).
It has figures, all output formats and windows. Use `pltrs-core` directly
only if you need the geometry and number types alone.

All pltrs crates have the same version. Each crate requires the same
version of the other pltrs crates.

## License

You can use this crate under the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`).

The code and data from other projects keep their licenses. `THIRD_PARTY.md`
lists them, and `LICENSES/` has the license texts.
