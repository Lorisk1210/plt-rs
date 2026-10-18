//! Decoding the blocks of a progressive scan (`jdphuff.c`): the first DC
//! scan and its refinements, and per-component bands of AC coefficients
//! with end-of-band runs, first and refining.

use super::bits::BitReader;
use super::error::DecodeError;
use super::frame::Block;
use super::huffman::HuffmanTable;
use super::scan::BlockCoder;
use crate::jpeg::tables::ZIGZAG;

/// The kind of progressive scan, with its state.
#[derive(Debug)]
pub(super) enum Progressive {
    /// The first scan of DC coefficients: differences scaled by `2^al`.
    DcFirst {
        /// Each component's DC table, in scan order.
        tables: Vec<HuffmanTable>,
        /// Each component's last DC value.
        last_dc: Vec<i32>,
        /// The bit position coded.
        al: u8,
    },
    /// One more bit of each DC coefficient.
    DcRefine {
        /// The bit position coded.
        al: u8,
    },
    /// The first scan of a band of AC coefficients.
    AcFirst(AcBand),
    /// One more bit of a band of AC coefficients.
    AcRefine(AcBand),
}

/// A band of AC coefficients of one component.
#[derive(Debug)]
pub(super) struct AcBand {
    table: HuffmanTable,
    /// First and last zigzag index of the band.
    start: usize,
    end: usize,
    /// The bit position coded.
    al: u8,
    /// Blocks left in the current end-of-band run.
    eob_run: u32,
}

impl AcBand {
    /// The band `start..=end` coded at bit `al` with `table`.
    pub(super) fn new(table: HuffmanTable, start: u8, end: u8, al: u8) -> Self {
        Self {
            table,
            start: usize::from(start),
            end: usize::from(end),
            al,
            eob_run: 0,
        }
    }

    /// Reads an end-of-band run of `2^r` plus `r` extra bits.
    fn read_eob_run(reader: &mut BitReader<'_>, r: u8) -> Result<u32, DecodeError> {
        let extra = if r > 0 { reader.bits(u32::from(r))? } else { 0 };
        Ok((1 << r) + extra)
    }

    /// `decode_mcu_AC_first`.
    fn first(&mut self, reader: &mut BitReader<'_>, block: &mut Block) -> Result<(), DecodeError> {
        if self.eob_run > 0 {
            self.eob_run -= 1;
            return Ok(());
        }
        let mut k = self.start;
        while k <= self.end {
            let symbol = self.table.decode(reader)?;
            let (run, size) = (symbol >> 4, symbol & 15);
            if size != 0 {
                k += usize::from(run);
                let value = reader.extend(size)?;
                block[ZIGZAG[k.min(63)]] = ((value as u32) << self.al) as i16;
            } else if run == 15 {
                k += 15;
            } else {
                self.eob_run = Self::read_eob_run(reader, run)? - 1;
                break;
            }
            k += 1;
        }
        Ok(())
    }

    /// `decode_mcu_AC_refine`.
    fn refine(&mut self, reader: &mut BitReader<'_>, block: &mut Block) -> Result<(), DecodeError> {
        let plus = 1_i16 << self.al;
        let minus = -1_i16 << self.al;
        // A correction bit of 1 moves an already nonzero coefficient away
        // from zero by one step, unless it already has that bit.
        let correct = |reader: &mut BitReader<'_>, coefficient: &mut i16| {
            if reader.bit()? && *coefficient & plus == 0 {
                let step = if *coefficient >= 0 { plus } else { minus };
                *coefficient = coefficient.wrapping_add(step);
            }
            Ok::<(), DecodeError>(())
        };
        let mut k = self.start;
        if self.eob_run == 0 {
            while k <= self.end {
                let symbol = self.table.decode(reader)?;
                let (mut zeros, size) = (i32::from(symbol >> 4), symbol & 15);
                let mut value = 0;
                if size != 0 {
                    value = if reader.bit()? { plus } else { minus };
                } else if zeros != 15 {
                    self.eob_run = Self::read_eob_run(reader, symbol >> 4)?;
                    break;
                }
                // Skip `zeros` zero coefficients, correcting the nonzero
                // ones on the way.
                loop {
                    let coefficient = &mut block[ZIGZAG[k]];
                    if *coefficient != 0 {
                        correct(reader, coefficient)?;
                    } else {
                        zeros -= 1;
                        if zeros < 0 {
                            break;
                        }
                    }
                    k += 1;
                    if k > self.end {
                        break;
                    }
                }
                if value != 0 {
                    block[ZIGZAG[k.min(63)]] = value;
                }
                k += 1;
            }
        }
        if self.eob_run > 0 {
            // The rest of the band is in the run: only corrections remain.
            while k <= self.end {
                let coefficient = &mut block[ZIGZAG[k]];
                if *coefficient != 0 {
                    correct(reader, coefficient)?;
                }
                k += 1;
            }
            self.eob_run -= 1;
        }
        Ok(())
    }
}

impl BlockCoder for Progressive {
    fn decode(
        &mut self,
        reader: &mut BitReader<'_>,
        slot: usize,
        block: &mut Block,
    ) -> Result<(), DecodeError> {
        match self {
            Self::DcFirst {
                tables,
                last_dc,
                al,
            } => {
                let size = tables[slot].decode(reader)?;
                let difference = reader.extend(size)?;
                last_dc[slot] = last_dc[slot]
                    .checked_add(difference)
                    .ok_or(DecodeError::CoefficientOverflow)?;
                block[0] = ((last_dc[slot] as u32) << *al) as i16;
            }
            Self::DcRefine { al } => {
                if reader.bit()? {
                    block[0] |= 1 << *al;
                }
            }
            Self::AcFirst(band) => band.first(reader, block)?,
            Self::AcRefine(band) => band.refine(reader, block)?,
        }
        Ok(())
    }

    fn restart(&mut self) {
        match self {
            Self::DcFirst { last_dc, .. } => last_dc.fill(0),
            Self::DcRefine { .. } => {}
            Self::AcFirst(band) | Self::AcRefine(band) => band.eob_run = 0,
        }
    }
}
