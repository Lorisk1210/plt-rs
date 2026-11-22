# pltrs-text

`pltrs-text` lays out text, mathtext and usetex text. It gives the same
results as matplotlib.

Contents:

* Layout of a line of text into positioned glyphs
* Text with more than one line: line spacing, alignment and rotation
* Mathtext, the math syntax of matplotlib
* `text.usetex`: pltrs runs LaTeX and reads the DVI, virtual font, TFM
  and font map files

## Use

Most programs need the crate [`pltrs`](https://crates.io/crates/pltrs).
It has figures, all output formats and windows. Use `pltrs-text` directly
only if you need the text layout alone.

All pltrs crates have the same version. Each crate requires the same
version of the other pltrs crates.

## License

You can use this crate under the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`).

The code and data from other projects keep their licenses. `THIRD_PARTY.md`
lists them, and `LICENSES/` has the license texts.
