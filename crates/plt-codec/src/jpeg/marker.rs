//! The second bytes of the JPEG markers (each follows an `0xFF`) that the
//! encoder writes or the decoder interprets.

/// Baseline sequential frame header.
pub(super) const SOF0: u8 = 0xC0;
/// Extended sequential frame header, Huffman coded.
pub(super) const SOF1: u8 = 0xC1;
/// Progressive frame header, Huffman coded.
pub(super) const SOF2: u8 = 0xC2;
/// Huffman tables.
pub(super) const DHT: u8 = 0xC4;
/// The first restart marker; `RST0 + n` is restart marker `n` (0–7).
pub(super) const RST0: u8 = 0xD0;
/// Start of image.
pub(super) const SOI: u8 = 0xD8;
/// End of image.
pub(super) const EOI: u8 = 0xD9;
/// Start of scan.
pub(super) const SOS: u8 = 0xDA;
/// Quantization tables.
pub(super) const DQT: u8 = 0xDB;
/// Define number of lines.
pub(super) const DNL: u8 = 0xDC;
/// Restart interval.
pub(super) const DRI: u8 = 0xDD;
/// Application segment 0 (JFIF).
pub(super) const APP0: u8 = 0xE0;
/// Application segment 14 (Adobe).
pub(super) const APP14: u8 = 0xEE;
/// Comment.
pub(super) const COM: u8 = 0xFE;
/// Temporary private use, a marker without a segment.
pub(super) const TEM: u8 = 0x01;
