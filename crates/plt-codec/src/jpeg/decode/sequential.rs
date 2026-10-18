//! Decoding the blocks of a sequential (baseline or extended) scan:
//! whole blocks, DC differences then run-length coded AC coefficients
//! (libjpeg's `decode_mcu`).

use super::bits::BitReader;
use super::error::DecodeError;
use super::frame::Block;
use super::huffman::HuffmanTable;
use super::scan::BlockCoder;
use crate::jpeg::tables::ZIGZAG;

/// The tables and DC predictions of a sequential scan's components.
#[derive(Debug)]
pub(super) struct Sequential {
    /// DC and AC tables of each component, in scan order.
    tables: Vec<(HuffmanTable, HuffmanTable)>,
    /// The last DC value of each component.
    last_dc: Vec<i32>,
}

impl Sequential {
    /// A coder with the scan components' DC and AC tables.
    pub(super) fn new(tables: Vec<(HuffmanTable, HuffmanTable)>) -> Self {
        let last_dc = vec![0; tables.len()];
        Self { tables, last_dc }
    }
}

impl BlockCoder for Sequential {
    fn decode(
        &mut self,
        reader: &mut BitReader<'_>,
        slot: usize,
        block: &mut Block,
    ) -> Result<(), DecodeError> {
        let (dc, ac) = &self.tables[slot];
        let size = dc.decode(reader)?;
        let difference = reader.extend(size)?;
        // libjpeg lets the prediction wrap, and stores a 16-bit
        // coefficient.
        self.last_dc[slot] = self.last_dc[slot].wrapping_add(difference);
        block[0] = self.last_dc[slot] as i16;
        let mut k = 1;
        while k < 64 {
            let symbol = ac.decode(reader)?;
            let (run, size) = (usize::from(symbol >> 4), symbol & 15);
            if size == 0 {
                if run != 15 {
                    break;
                }
                k += 15;
            } else {
                k += run;
                // Corrupt runs past the block land on its last coefficient,
                // as with libjpeg's padded `jpeg_natural_order`.
                block[ZIGZAG[k.min(63)]] = reader.extend(size)? as i16;
            }
            k += 1;
        }
        Ok(())
    }

    fn restart(&mut self) {
        self.last_dc.fill(0);
    }
}
