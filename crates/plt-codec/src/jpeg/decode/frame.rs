//! The frame being decoded: each component's geometry as libjpeg derives
//! it (`initial_setup` in `jdinput.c`) and its whole-image buffer of
//! quantized coefficients, which every scan adds to.

use super::error::DecodeError;
use super::markers::{ComponentSpec, FrameHeader, QuantTables};
use crate::raster::MAX_PIXELS;

/// libjpeg's largest image side (`JPEG_MAX_DIMENSION`).
const MAX_DIMENSION: u16 = 65_500;

/// The quantized DCT coefficients of an 8×8 block, in natural order.
pub(super) type Block = [i16; 64];

/// One component of the frame.
#[derive(Clone, Debug)]
pub(super) struct Component {
    /// As the frame header declares it.
    pub(super) spec: ComponentSpec,
    /// The quantization table, latched by the first scan of the component.
    pub(super) quant: Option<[u16; 64]>,
    /// Blocks covering the component's samples across.
    pub(super) width_in_blocks: usize,
    /// Blocks covering the component's samples down.
    pub(super) height_in_blocks: usize,
    /// The component's samples across: the image width scaled by its
    /// horizontal sampling.
    pub(super) sample_width: usize,
    /// The component's samples down.
    pub(super) sample_height: usize,
    /// Blocks per row of the buffer, padded to whole MCUs.
    blocks_per_line: usize,
    /// The coefficient buffer, padded to whole MCUs; empty until the
    /// first scan.
    blocks: Vec<Block>,
    /// The successive approximation bit each coefficient is known to, or
    /// −1 before any scan carried it (libjpeg's `coef_bits`).
    pub(super) coef_bits: [i8; 64],
}

impl Component {
    /// The block at `row` and `column` of the padded buffer.
    pub(super) fn block_mut(&mut self, row: usize, column: usize) -> &mut Block {
        &mut self.blocks[row * self.blocks_per_line + column]
    }

    /// The block at `row` and `column` of the padded buffer.
    pub(super) fn block(&self, row: usize, column: usize) -> &Block {
        &self.blocks[row * self.blocks_per_line + column]
    }
}

/// The frame: image size, sampling and components.
#[derive(Clone, Debug)]
pub(super) struct Frame {
    /// Whether scans are progressive.
    pub(super) progressive: bool,
    /// The image width.
    pub(super) width: usize,
    /// The image height.
    pub(super) height: usize,
    /// The largest horizontal sampling factor.
    pub(super) max_h: usize,
    /// The largest vertical sampling factor.
    pub(super) max_v: usize,
    /// MCUs across in an interleaved scan.
    pub(super) mcus_across: usize,
    /// MCUs down in an interleaved scan.
    pub(super) mcus_down: usize,
    /// The components, in frame order.
    pub(super) components: Vec<Component>,
}

impl Frame {
    /// The frame of a header. Sampling ratios must be whole numbers, as
    /// libjpeg's upsampler requires.
    pub(super) fn new(header: &FrameHeader) -> Result<Self, DecodeError> {
        let (width, height) = (header.width, header.height);
        let too_large = DecodeError::TooLarge { width, height };
        if width > MAX_DIMENSION
            || height > MAX_DIMENSION
            || u64::from(width) * u64::from(height) > MAX_PIXELS
        {
            return Err(too_large);
        }
        let (width, height) = (usize::from(width), usize::from(height));
        let max_h = header.components.iter().map(|c| c.h).max().unwrap_or(1);
        let max_v = header.components.iter().map(|c| c.v).max().unwrap_or(1);
        if header
            .components
            .iter()
            .any(|c| max_h % c.h != 0 || max_v % c.v != 0)
        {
            return Err(DecodeError::InvalidSampling);
        }
        let (mcus_across, mcus_down) = (width.div_ceil(8 * max_h), height.div_ceil(8 * max_v));
        let components = header
            .components
            .iter()
            .map(|&spec| {
                let sample_width = (width * spec.h).div_ceil(max_h);
                let sample_height = (height * spec.v).div_ceil(max_v);
                Component {
                    spec,
                    quant: None,
                    width_in_blocks: sample_width.div_ceil(8),
                    height_in_blocks: sample_height.div_ceil(8),
                    sample_width,
                    sample_height,
                    blocks_per_line: mcus_across * spec.h,
                    blocks: Vec::new(),
                    coef_bits: [-1; 64],
                }
            })
            .collect();
        Ok(Self {
            progressive: header.progressive,
            width,
            height,
            max_h,
            max_v,
            mcus_across,
            mcus_down,
            components,
        })
    }

    /// Allocates the coefficient buffers before the first scan. Every
    /// block costs at least one bit of entropy-coded data, so a frame
    /// with more blocks than `8 × remaining` bits of input is refused
    /// before anything is allocated.
    pub(super) fn allocate(&mut self, remaining: usize) -> Result<(), DecodeError> {
        if self.components.iter().any(|c| !c.blocks.is_empty()) {
            return Ok(());
        }
        let coded: usize = self
            .components
            .iter()
            .map(|c| c.width_in_blocks * c.height_in_blocks)
            .sum();
        if coded > remaining.saturating_mul(8) {
            return Err(DecodeError::TooLarge {
                width: u16::try_from(self.width).expect("checked by new"),
                height: u16::try_from(self.height).expect("checked by new"),
            });
        }
        let mcus_down = self.mcus_down;
        for component in &mut self.components {
            let rows = mcus_down * component.spec.v;
            component.blocks = vec![[0; 64]; component.blocks_per_line * rows];
        }
        Ok(())
    }

    /// Latches the quantization table of component `index` at its first
    /// scan, as libjpeg's `latch_quant_tables` does: later redefinitions
    /// of the table do not affect it.
    pub(super) fn latch_quant_table(
        &mut self,
        index: usize,
        tables: &QuantTables,
    ) -> Result<(), DecodeError> {
        let component = &mut self.components[index];
        if component.quant.is_none() {
            component.quant = Some(tables.get(component.spec.quant_table)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg::marker::SOF0;

    fn frame(width: u16, height: u16, sampling: &[u8]) -> Result<Frame, DecodeError> {
        let mut payload = vec![8];
        payload.extend_from_slice(&height.to_be_bytes());
        payload.extend_from_slice(&width.to_be_bytes());
        payload.push(u8::try_from(sampling.len()).unwrap());
        for (id, &factors) in (1..).zip(sampling) {
            payload.extend_from_slice(&[id, factors, 0]);
        }
        Frame::new(&FrameHeader::parse(SOF0, &payload)?)
    }

    #[test]
    fn geometry_follows_libjpeg() {
        let f = frame(17, 9, &[0x22, 0x11, 0x11]).unwrap();
        assert_eq!((f.mcus_across, f.mcus_down), (2, 1));
        let [y, cb, _] = &f.components[..] else {
            panic!("three components")
        };
        assert_eq!((y.sample_width, y.sample_height), (17, 9));
        assert_eq!((y.width_in_blocks, y.height_in_blocks), (3, 2));
        assert_eq!(y.blocks_per_line, 4);
        assert_eq!((cb.sample_width, cb.sample_height), (9, 5));
        assert_eq!((cb.width_in_blocks, cb.height_in_blocks), (2, 1));
    }

    #[test]
    fn fractional_sampling_and_huge_sizes_are_refused() {
        assert_eq!(
            frame(8, 8, &[0x31, 0x21, 0x11]).err(),
            Some(DecodeError::InvalidSampling)
        );
        assert!(matches!(
            frame(65_501, 1, &[0x11]),
            Err(DecodeError::TooLarge { .. })
        ));
        assert!(matches!(
            frame(65_000, 65_000, &[0x11]),
            Err(DecodeError::TooLarge { .. })
        ));
    }

    #[test]
    fn buffers_need_enough_input() {
        let mut f = frame(800, 800, &[0x11]).unwrap();
        assert!(matches!(f.allocate(100), Err(DecodeError::TooLarge { .. })));
        assert_eq!(f.components[0].blocks.len(), 0, "nothing allocated");
        f.allocate(10_000 / 8).unwrap();
        assert_eq!(f.components[0].blocks.len(), 100 * 100);
    }
}
