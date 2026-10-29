//! Decoding one scan: checking its header against the frame and the
//! coding process (`per_scan_setup`, `start_pass_phuff_decoder`), then
//! walking its MCUs with restart markers between intervals.

use super::bits::BitReader;
use super::error::DecodeError;
use super::frame::{Block, Frame};
use super::huffman::HuffmanTables;
use super::markers::{QuantTables, ScanHeader};
use super::progressive::{AcBand, Progressive};
use super::sequential::Sequential;

/// Most blocks an interleaved MCU may hold (`D_MAX_BLOCKS_IN_MCU`).
const MAX_BLOCKS_IN_MCU: usize = 10;
/// The largest successive approximation bit libjpeg accepts.
const MAX_AL: u8 = 13;

/// Decodes the blocks of a scan, one at a time.
pub(super) trait BlockCoder {
    /// Decodes the next block, of the scan's component `slot`, into
    /// `block`.
    fn decode(
        &mut self,
        reader: &mut BitReader<'_>,
        slot: usize,
        block: &mut Block,
    ) -> Result<(), DecodeError>;

    /// Resets the predictions at a restart marker.
    fn restart(&mut self);
}

/// The tables a scan may refer to.
#[derive(Clone, Copy, Debug)]
pub(super) struct Tables<'a> {
    /// The Huffman tables defined so far.
    pub(super) huffman: &'a HuffmanTables,
    /// The quantization tables defined so far.
    pub(super) quant: &'a QuantTables,
    /// The restart interval in MCUs, 0 for none.
    pub(super) restart_interval: u16,
}

/// Decodes the scan whose entropy-coded data starts at `start` of
/// `data` into the frame's coefficients; returns where its data ends.
pub(super) fn decode_scan(
    frame: &mut Frame,
    header: &ScanHeader,
    tables: Tables<'_>,
    data: &[u8],
    start: usize,
) -> Result<usize, DecodeError> {
    if header.components.len() > 1 {
        let blocks: usize = header
            .components
            .iter()
            .map(|c| frame.components[c.index].spec.h * frame.components[c.index].spec.v)
            .sum();
        if blocks > MAX_BLOCKS_IN_MCU {
            return Err(DecodeError::InvalidSampling);
        }
    }
    for component in &header.components {
        frame.latch_quant_table(component.index, tables.quant)?;
    }
    let mut reader = BitReader::new(data, start);
    let interval = usize::from(tables.restart_interval);
    if frame.progressive {
        let mut coder = progressive_coder(frame, header, tables.huffman)?;
        frame.allocate(data.len() - start)?;
        decode_mcus(frame, header, &mut coder, &mut reader, interval)?;
    } else {
        let coders = header
            .components
            .iter()
            .map(|c| {
                Ok((
                    tables.huffman.dc(c.dc_table)?,
                    tables.huffman.ac(c.ac_table)?,
                ))
            })
            .collect::<Result<_, DecodeError>>()?;
        frame.allocate(data.len() - start)?;
        decode_mcus(
            frame,
            header,
            &mut Sequential::new(coders),
            &mut reader,
            interval,
        )?;
    }
    Ok(reader.end())
}

/// The coder of a progressive scan, after libjpeg's checks of its
/// spectral band and bit positions (inconsistencies between scans are
/// only warnings to libjpeg, and are accepted).
fn progressive_coder(
    frame: &mut Frame,
    header: &ScanHeader,
    huffman: &HuffmanTables,
) -> Result<Progressive, DecodeError> {
    let &ScanHeader { ss, se, ah, al, .. } = header;
    let dc_band = ss == 0;
    let band_ok = if dc_band {
        se == 0
    } else {
        ss <= se && se <= 63 && header.components.len() == 1
    };
    if !band_ok || (ah != 0 && al + 1 != ah) || al > MAX_AL {
        return Err(DecodeError::InvalidScan);
    }
    for component in &header.components {
        let bits = &mut frame.components[component.index].coef_bits;
        bits[usize::from(ss)..=usize::from(se)].fill(al as i8);
    }
    let first = header.components[0];
    Ok(match (dc_band, ah == 0) {
        (true, true) => Progressive::DcFirst {
            tables: header
                .components
                .iter()
                .map(|c| huffman.dc(c.dc_table))
                .collect::<Result<_, _>>()?,
            last_dc: vec![0; header.components.len()],
            al,
        },
        (true, false) => Progressive::DcRefine { al },
        (false, true) => Progressive::AcFirst(AcBand::new(huffman.ac(first.ac_table)?, ss, se, al)),
        (false, false) => {
            Progressive::AcRefine(AcBand::new(huffman.ac(first.ac_table)?, ss, se, al))
        }
    })
}

/// Walks the scan's MCUs in order: one block each when the scan has a
/// single component (only the blocks covering its samples), otherwise
/// each component's `h × v` blocks of every MCU of the frame.
fn decode_mcus(
    frame: &mut Frame,
    header: &ScanHeader,
    coder: &mut impl BlockCoder,
    reader: &mut BitReader<'_>,
    interval: usize,
) -> Result<(), DecodeError> {
    let mut next_restart = 0;
    let mut restart = |mcu: usize, coder: &mut dyn BlockCoder, reader: &mut BitReader<'_>| {
        if interval > 0 && mcu > 0 && mcu.is_multiple_of(interval) {
            reader.restart(next_restart)?;
            next_restart = (next_restart + 1) % 8;
            coder.restart();
        }
        Ok::<(), DecodeError>(())
    };
    if let [only] = header.components[..] {
        let component = &mut frame.components[only.index];
        let across = component.width_in_blocks;
        for mcu in 0..across * component.height_in_blocks {
            restart(mcu, coder, reader)?;
            coder.decode(reader, 0, component.block_mut(mcu / across, mcu % across))?;
        }
        return Ok(());
    }
    let across = frame.mcus_across;
    for mcu in 0..across * frame.mcus_down {
        restart(mcu, coder, reader)?;
        let (row, column) = (mcu / across, mcu % across);
        for (slot, scan_component) in header.components.iter().enumerate() {
            let component = &mut frame.components[scan_component.index];
            let (h, v) = (component.spec.h, component.spec.v);
            for y in 0..v {
                for x in 0..h {
                    let block = component.block_mut(row * v + y, column * h + x);
                    coder.decode(reader, slot, block)?;
                }
            }
        }
    }
    Ok(())
}
