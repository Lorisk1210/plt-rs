//! The connection setup reply, mutated from the captured one: refusals,
//! lying counts and lengths, odd vendors, pixmap formats and visuals;
//! the pixel formats the visuals describe, from the reply and with
//! random colour masks, then converting images with them and splitting
//! regions into requests.

use super::word;
use crate::frame::OwnedFrame;
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::geometry::{PhysicalRect, PhysicalSize};
use crate::x11::fixtures::SETUP;
use crate::x11::image::PixelFormat;
use crate::x11::setup::{self, Format, ImageByteOrder, Setup, Visual};

/// Checks what a pixel format does with images: converting a frame and
/// back, short image data, and tiles that cover `region` exactly within
/// `max_bytes` each.
fn exercise(format: &PixelFormat, region: PhysicalRect, max_bytes: usize) {
    let frame =
        OwnedFrame::new(3, 2, (0..24).map(|i| (i * 37) as u8).collect()).expect("a 3 × 2 frame");
    let mut data = Vec::new();
    format.encode(&frame.as_frame(), PhysicalRect::new(0, 0, 3, 2), &mut data);
    assert_eq!(data.len(), format.stride(3) * 2);
    let decoded = format
        .decode(&data, frame.size())
        .expect("encoded data decodes");
    assert_eq!(decoded.size(), frame.size());
    assert!(
        format
            .decode(&data[..data.len() - 1], frame.size())
            .is_err()
    );
    let tiles = format.tiles(region, max_bytes);
    let mut covered = 0;
    for tile in &tiles {
        assert!(!tile.is_empty());
        assert!(tile.x >= region.x && tile.x + tile.width <= region.x + region.width);
        assert!(tile.y >= region.y && tile.y + tile.height <= region.y + region.height);
        let bytes = format.stride(tile.width) * tile.height as usize;
        assert!(
            bytes <= max_bytes || (tile.width, tile.height) == (1, 1),
            "{tile:?} needs {bytes} bytes, more than {max_bytes}"
        );
        covered += u64::from(tile.width) * u64::from(tile.height);
    }
    assert_eq!(
        covered,
        u64::from(region.width) * u64::from(region.height),
        "the tiles cover {region:?}"
    );
}

/// What a backend does with a setup: picks each screen's visual and the
/// pixel format of its depth.
fn use_setup(setup: &Setup, region: PhysicalRect, max_bytes: usize) {
    for screen in &setup.screens {
        let Some((depth, visual)) = screen.true_color_visual() else {
            continue;
        };
        assert_eq!(screen.visual(visual.id).map(|(d, _)| d), Some(depth));
        let Some(format) = setup.format(depth) else {
            continue;
        };
        if let Ok(format) = PixelFormat::new(format, &visual, setup.image_byte_order) {
            exercise(&format, region, max_bytes);
        }
    }
}

/// A region to paint, small or as wide and tall as windows get, and the
/// most image data a request may carry: from what the smallest request
/// length servers announce leaves (beside a 28-byte header) to plenty.
fn region(g: &mut Gen) -> (PhysicalRect, usize) {
    let side = |g: &mut Gen| g.pick(&[0, 1, 2, 7, 64, 1000, 32_767]);
    let region = PhysicalRect::new(g.below(100) as u32, g.below(100) as u32, side(g), side(g));
    let smallest = usize::from(setup::MIN_MAXIMUM_REQUEST_LENGTH) * 4 - 28;
    let max_bytes = g.pick(&[smallest, smallest + 1, 262_112, 1 << 24, 1 << 34]);
    (region, max_bytes)
}

#[test]
fn setup_replies() {
    fuzz(
        "x11 setup",
        3_000,
        |g| {
            let corruption: Corruption = g.corrupt(SETUP);
            (corruption, region(g))
        },
        |(corruption, (region, max_bytes))| {
            let reply = corruption.apply(SETUP);
            if let Some(header) = reply.first_chunk::<{ setup::REPLY_HEADER_LEN }>() {
                assert!(setup::reply_body_len(*header) <= 4 * usize::from(u16::MAX));
            }
            if let Ok(setup) = setup::parse_reply(&reply) {
                assert_ne!(setup.resource_id_mask, 0);
                use_setup(&setup, *region, *max_bytes);
            }
        },
    );
}

/// A colour mask: a run of up to 16 bits anywhere mostly, a longer run
/// (up to all 32 bits), or anything.
fn mask(g: &mut Gen) -> u32 {
    let bits = match g.below(10) {
        0..7 => g.below(17) as u32,
        7 => 17 + g.below(16) as u32,
        _ => return word(g),
    };
    let shift = g.below(32) as u32;
    u32::MAX
        .checked_shr(32 - bits)
        .unwrap_or(0)
        .wrapping_shl(shift)
}

/// A visual and a pixmap format as a server may describe them.
#[derive(Debug)]
struct Described {
    format: Format,
    visual: Visual,
    order: ImageByteOrder,
}

impl Described {
    fn generate(g: &mut Gen) -> Self {
        Self {
            format: Format {
                depth: g.pick(&[1, 8, 15, 16, 24, 30, 32]),
                bits_per_pixel: g.pick(&[16, 24, 32, 16, 24, 32, 0, 1, 8, 64]),
                scanline_pad: g.pick(&[8, 16, 32, 8, 16, 32, 0, 7, 64]),
            },
            visual: Visual {
                id: word(g),
                class: setup::VISUAL_CLASS_TRUE_COLOR,
                red_mask: mask(g),
                green_mask: mask(g),
                blue_mask: mask(g),
            },
            order: g.pick(&[ImageByteOrder::LsbFirst, ImageByteOrder::MsbFirst]),
        }
    }
}

#[test]
fn pixel_formats() {
    fuzz(
        "x11 pixel formats",
        3_000,
        |g| (Described::generate(g), region(g)),
        |(described, (region, max_bytes))| {
            if let Ok(format) =
                PixelFormat::new(described.format, &described.visual, described.order)
            {
                exercise(&format, *region, *max_bytes);
            }
        },
    );
}

#[test]
fn image_data() {
    fuzz(
        "x11 images",
        1_000,
        |g| {
            let size = PhysicalSize::new(g.below(40) as u32, g.below(40) as u32);
            let len = g.below(5_000);
            (Described::generate(g), size, len)
        },
        |(described, size, len)| {
            if let Ok(format) =
                PixelFormat::new(described.format, &described.visual, described.order)
            {
                let data: Vec<u8> = (0..*len).map(|i| (i * 131 % 251) as u8).collect();
                if let Ok(frame) = format.decode(&data, *size) {
                    assert_eq!(frame.size(), *size);
                }
            }
        },
    );
}
