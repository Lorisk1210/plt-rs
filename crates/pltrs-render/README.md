# pltrs-render

The renderer abstraction of pltrs and its backends: raster (PNG), SVG,
PDF, PostScript and PGF.

* a `Renderer` trait with a graphics context for strokes and clipping
* a raster backend that reproduces matplotlib's Agg backend pixel for
  pixel and writes PNG
* SVG, PDF, PostScript/EPS and PGF backends writing the documents of
  matplotlib's renderers, with font embedding and subsetting
* usetex text drawn from its DVI file

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its backends
alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The
code and data it takes from other projects keep their own licences:
`THIRD_PARTY.md` lists them, with their texts in `LICENSES/`.
