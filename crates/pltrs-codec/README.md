# pltrs-codec

Self-contained compression and image codecs, written for pltrs: no
dependencies and no `unsafe` code.

* DEFLATE and zlib, writing zlib-ng 2.3's bytes, with a strictly
  validating decompressor
* PNG and APNG; JPEG, TIFF and GIF as Pillow writes and reads them; WebP
  decoding and lossless encoding
* an intra-only AV1 encoder and AVIF files, as Pillow writes them
  through libavif
* CRC-32, Adler-32, SHA-1, SHA-256 and base64

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its codecs alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The
code and data it takes from other projects keep their own licences:
`THIRD_PARTY.md` lists them, with their texts in `LICENSES/`.
