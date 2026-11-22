# pltrs-codec

`pltrs-codec` has compression and image codecs. It has no dependencies and
no `unsafe` code.

Contents:

* DEFLATE and zlib. The encoder writes the same bytes as zlib-ng 2.3.
* PNG and APNG
* JPEG, TIFF and GIF, with the same output as Pillow
* WebP: a decoder and a lossless encoder
* AVIF: an AV1 encoder for still images
* CRC-32, Adler-32, SHA-1, SHA-256 and base64

## Use

Most programs need the crate [`pltrs`](https://crates.io/crates/pltrs).
It has figures, all output formats and windows. Use `pltrs-codec` directly
only if you need the codecs alone.

All pltrs crates have the same version. Each crate requires the same
version of the other pltrs crates.

## License

You can use this crate under the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`).

The code and data from other projects keep their licenses. `THIRD_PARTY.md`
lists them, and `LICENSES/` has the license texts.
