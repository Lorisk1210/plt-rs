# pltrs-font

`pltrs-font` reads fonts and finds the font for a text. It includes the
fonts that matplotlib includes, and Inter.

Contents:

* TrueType, OpenType (CFF) and Type 1 fonts, and font collections:
  metrics, character maps, glyph outlines, names and kerning
* Glyph scaling and hinting with the same results as FreeType
* Text shaping with the same results as HarfBuzz and libraqm:
  bidirectional text, Arabic, Hebrew, OpenType features and fallback fonts
* A font database that selects fonts as the matplotlib function
  `findfont` selects them, from the included fonts and the system fonts

## Use

Most programs need the crate [`pltrs`](https://crates.io/crates/pltrs).
It has figures, all output formats and windows. Use `pltrs-font` directly
only if you need the fonts alone.

All pltrs crates have the same version. Each crate requires the same
version of the other pltrs crates.

## License

You can use this crate under the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`).

The code and data from other projects keep their licenses. `THIRD_PARTY.md`
lists them, and `LICENSES/` has the license texts.
