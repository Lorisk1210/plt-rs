# pltrs-font

Font parsing, outlines, hinting, shaping and font management for pltrs,
with the fonts matplotlib ships bundled.

* TrueType, OpenType (CFF), collections and Type 1 fonts: metrics,
  character maps, glyph outlines, names and kerning
* FreeType-compatible scaling, TrueType bytecode hinting and FreeType's
  autohinter
* HarfBuzz/libraqm-style shaping: bidirectional text, Arabic and Hebrew,
  OpenType features and languages, per-character fallback
* a font database with matplotlib's `findfont` scoring, over the bundled
  fonts (DejaVu, STIX, Computer Modern, Inter) and the system's

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its fonts alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The
code and data it takes from other projects keep their own licences:
`THIRD_PARTY.md` lists them, with their texts in `LICENSES/`.
