# pltrs-text

Text layout, mathtext and usetex for pltrs, as matplotlib does them.

* lines of text laid out into glyph runs as matplotlib's FreeType and
  HarfBuzz pipeline lays them out
* multi-line text blocks: stacking, alignment and rotation
* mathtext, matplotlib's TeX-like math typesetting
* `text.usetex`: typesetting with LaTeX and reading its DVI, virtual
  font, TFM and font map files

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its text layout
alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The
code and data it takes from other projects keep their own licences:
`THIRD_PARTY.md` lists them, with their texts in `LICENSES/`.
