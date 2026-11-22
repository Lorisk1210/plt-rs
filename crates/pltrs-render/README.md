# pltrs-render

`pltrs-render` has the `Renderer` trait and the output backends of pltrs.

Contents:

* The `Renderer` trait, with a graphics context for strokes and clipping
* A raster backend that writes PNG. It gives the same pixels as the Agg
  backend of matplotlib.
* SVG, PDF, PostScript, EPS and PGF backends, with embedded fonts and
  font subsets
* Drawing of usetex text from its DVI file

## Use

Most programs need the crate [`pltrs`](https://crates.io/crates/pltrs).
It has figures, all output formats and windows. Use `pltrs-render` directly
only if you need the backends alone.

All pltrs crates have the same version. Each crate requires the same
version of the other pltrs crates.

## License

You can use this crate under the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`).

The code and data from other projects keep their licenses. `THIRD_PARTY.md`
lists them, and `LICENSES/` has the license texts.
