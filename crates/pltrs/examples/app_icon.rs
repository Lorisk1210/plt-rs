//! Writes pltrs's application icon as PNGs, to review it by eye: on
//! Apple's icon grid (as macOS's Dock shows it) and full bleed (as
//! Windows and Linux desktops show it), at 1024 down to 16 pixels, and a
//! sheet of them all on light and dark backgrounds, the small sizes also
//! magnified eight times.
//!
//! ```text
//! cargo run -p pltrs --example app_icon -- DIR
//! ```
//!
//! The files are `DIR/macos-SIDE.png`, `DIR/full-SIDE.png` and
//! `DIR/sheet.png`.

#![expect(
    clippy::print_stdout,
    reason = "the example reports where the images went"
)]

use std::error::Error;
use std::path::Path;

use pltrs_codec::png::{self, ImageRef};
use pltrs_platform::{IconImage, IconLayout};

/// The sides drawn, in pixels.
const SIDES: [u32; 7] = [1024, 512, 256, 128, 64, 32, 16];
/// The sides also shown magnified on the sheet.
const MAGNIFIED: [u32; 2] = [32, 16];
/// How many times the magnified sides are magnified.
const ZOOM: u32 = 8;
/// The space around and between the icons on the sheet.
const GAP: u32 = 32;
/// The sheet's backgrounds: macOS's light and dark window colours.
const BACKGROUNDS: [[u8; 3]; 2] = [[0xec, 0xec, 0xec], [0x1e, 0x1e, 0x1e]];

/// An opaque RGB image being composed.
struct Sheet {
    width: u32,
    rgb: Vec<u8>,
}

impl Sheet {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            rgb: vec![0; (width * height * 3) as usize],
        }
    }

    /// Fills a band of rows with `color`.
    fn fill_rows(&mut self, top: u32, height: u32, color: [u8; 3]) {
        let start = (top * self.width * 3) as usize;
        let end = ((top + height) * self.width * 3) as usize;
        self.rgb[start..end].as_chunks_mut::<3>().0.fill(color);
    }

    /// Draws `image` magnified `zoom` times (nearest neighbour) with its
    /// top-left corner at `(left, top)`, over what is there.
    fn draw(&mut self, image: &IconImage, (left, top): (u32, u32), zoom: u32) {
        let side = image.size().width;
        for y in 0..side * zoom {
            for x in 0..side * zoom {
                let source = (((y / zoom) * side + x / zoom) * 4) as usize;
                let [r, g, b, a] = image.rgba()[source..source + 4] else {
                    continue;
                };
                let target = (((top + y) * self.width + left + x) * 3) as usize;
                let alpha = u32::from(a);
                for (out, color) in self.rgb[target..target + 3].iter_mut().zip([r, g, b]) {
                    let mixed = u32::from(color) * alpha + u32::from(*out) * (255 - alpha);
                    *out = ((mixed + 127) / 255) as u8;
                }
            }
        }
    }
}

fn save(path: &Path, image: ImageRef<'_>) -> Result<(), Box<dyn Error>> {
    std::fs::write(path, png::encode(image)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args()
        .nth(1)
        .ok_or("usage: app_icon DIR (where to write the images)")?;
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir)?;
    let icon = pltrs::gui::app_icon();
    let layouts = [
        ("macos", IconLayout::AppleGrid),
        ("full", IconLayout::FullBleed),
    ];
    let rows: Vec<Vec<IconImage>> = layouts
        .iter()
        .map(|&(name, layout)| {
            let images = icon.images(&SIDES, layout);
            for image in &images {
                let side = image.size().width;
                let file = dir.join(format!("{name}-{side}.png"));
                save(&file, ImageRef::rgba8(side, side, image.rgba())?)?;
            }
            Ok(images)
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    let magnified: u32 = MAGNIFIED.iter().map(|side| side * ZOOM + GAP).sum();
    let width = GAP + SIDES.iter().map(|side| side + GAP).sum::<u32>() + magnified;
    let row_height = SIDES[0] + 2 * GAP;
    let height = row_height * (layouts.len() * BACKGROUNDS.len()) as u32;
    let mut sheet = Sheet::new(width, height);
    let mut top = 0;
    for images in &rows {
        for background in BACKGROUNDS {
            sheet.fill_rows(top, row_height, background);
            let mut left = GAP;
            for image in images {
                let side = image.size().width;
                sheet.draw(image, (left, top + GAP + (SIDES[0] - side) / 2), 1);
                left += side + GAP;
            }
            for image in images
                .iter()
                .filter(|i| MAGNIFIED.contains(&i.size().width))
            {
                let side = image.size().width * ZOOM;
                sheet.draw(image, (left, top + GAP + (SIDES[0] - side) / 2), ZOOM);
                left += side + GAP;
            }
            top += row_height;
        }
    }
    let file = dir.join("sheet.png");
    save(&file, ImageRef::rgb8(width, height, &sheet.rgb)?)?;
    println!("wrote the icon's images and {}", file.display());
    Ok(())
}
