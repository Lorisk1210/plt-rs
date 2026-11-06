//! pltrs's application icon, "Contour": filled contour bands in viridis
//! colours rising to a lime peak, on a deep violet squircle with a soft
//! drop shadow. The design source is `crates/pltrs/assets/icon/pltrs.svg`
//! (Apple's 1024-unit icon grid), whose geometry this module holds as
//! data and draws, at every size a platform asks for, with
//! [`ColorIcon`].
//!
//! * On Apple's grid the squircle is 824 units wide, centred, and casts
//!   its shadow into the margin; elsewhere it fills the square.
//! * Its edges fall on whole pixels at every size, so that they stay
//!   crisp.
//! * At 32 pixels and less, three bolder bands stand for the six and the
//!   shadow, too small to see, is left out.

use std::sync::LazyLock;

use pltrs_core::{Path, Point, Rgba};
use pltrs_platform::{Icon, IconImage, IconLayout};
use pltrs_ui::{ColorIcon, LinearGradient, Rect, Shadow};

/// The side of the design's square: Apple's icon grid.
const GRID: f64 = 1024.0;
/// Where the squircle sits on the grid: 100 units from each edge.
const BODY_MARGIN: f64 = 100.0;
/// The squircle: continuous corners of radius 185.4 on an 824-unit
/// square.
const BODY: &str = "M383.4 100H640.6C722.2 100 763 100 806.9 113.9C854.9 131.3 892.7 169.1 \
910.1 217.1C924 261 924 301.8 924 383.4V640.6C924 722.2 924 763 910.1 806.9C892.7 854.9 \
854.9 892.7 806.9 910.1C763 924 722.2 924 640.6 924H383.4C301.8 924 261 924 217.1 \
910.1C169.1 892.7 131.3 854.9 113.9 806.9C100 763 100 722.2 100 640.6V383.4C100 301.8 100 \
261 113.9 217.1C131.3 169.1 169.1 131.3 217.1 113.9C261 100 301.8 100 383.4 100Z";
/// The squircle's gradient: from the bottom-left to the top-right.
const BODY_GRADIENT: [(f64, f64, u32); 2] =
    [(200.0, 924.0, 0x3a_0a_55), (824.0, 100.0, 0x46_32_7e)];
/// The contour bands, outermost first, in viridis colours.
const BANDS: [(&str, u32); 6] = [
    (
        "M772.1 389C799.2 441.4 806.8 534.2 776.8 589.9C746.9 645.7 660.4 693.2 592.6 \
723.4C524.8 753.6 431.5 786.1 370.1 771C308.6 756 244.7 688.3 223.9 633C203.2 577.8 217.9 \
500.6 245.5 439.5C273 378.4 327.8 294 389.2 266.6C450.6 239.3 550 255.1 613.9 275.5C677.7 \
295.9 744.9 336.6 772.1 389Z",
        0x3b_52_8b,
    ),
    (
        "M752.3 385.6C773.3 426.6 766.7 502.8 740.7 548.6C714.7 594.4 651.2 634.9 596.4 \
660.5C541.5 686.1 460.7 714.8 411.8 702.4C362.9 689.9 320.1 630.8 302.8 585.7C285.5 540.6 \
286 480.1 307.8 431.7C329.6 383.3 382.5 316.6 433.7 295.1C484.8 273.5 561.5 287.4 614.6 \
302.5C667.7 317.6 731.3 344.6 752.3 385.6Z",
        0x2c_72_8e,
    ),
    (
        "M727.7 387.4C742.9 418.7 728.3 477.5 707.1 514C685.8 550.5 642.5 585.9 600.3 \
606.4C558 626.8 491.3 647.1 453.4 636.9C415.6 626.7 387.5 580.3 373.2 545.3C358.9 510.2 \
350.9 463.4 367.9 426.7C384.8 390 433.7 341.7 475 325C516.3 308.3 573.4 315.9 615.5 \
326.3C657.6 336.8 712.4 356.2 727.7 387.4Z",
        0x21_91_8c,
    ),
    (
        "M696.7 394.3C707.1 417.2 691.4 459.1 675.5 486.5C659.6 513.9 631.9 543.8 601.3 \
558.8C570.7 573.9 519.9 584.7 492 576.8C464 568.9 444.8 536.8 433.6 511.5C422.5 486.1 \
412.3 450.9 425 424.6C437.8 398.4 478.8 366.7 510.2 354.2C541.6 341.6 582.5 342.6 613.6 \
349.3C644.7 355.9 686.4 371.5 696.7 394.3Z",
        0x28_ae_80,
    ),
    (
        "M665.2 403.6C671.6 419 659.7 445.7 649 464.3C638.3 483 621.3 505.7 601 515.6C580.6 \
525.5 546.1 529.2 527 523.8C507.8 518.5 494 499.8 486.2 483.3C478.3 466.9 470.8 442.3 \
479.9 425.1C489 407.9 518.9 389.2 540.8 380.4C562.6 371.5 590.3 368 611 371.9C631.7 375.7 \
658.9 388.2 665.2 403.6Z",
        0x5e_c9_62,
    ),
    (
        "M631.4 415.1C634.7 423.7 628.9 437.3 623.2 447.6C617.5 457.8 608.2 471.4 597.2 \
476.6C586.1 481.8 567.6 481.9 556.9 479C546.1 476 537.1 467.7 532.7 459C528.3 450.4 525.1 \
436.2 530.4 427C535.7 417.7 552.4 408.6 564.6 403.5C576.8 398.4 592.5 394.3 603.6 \
396.2C614.7 398.2 628.1 406.6 631.4 415.1Z",
        0xad_dc_30,
    ),
];
/// The bands of the small rendition: the outermost band's shape, the
/// middle one's and the second innermost's, the last in the peak's lime.
const SMALL_BANDS: [(usize, u32); 3] = [(0, 0x2c_72_8e), (2, 0x28_ae_80), (4, 0xad_dc_30)];
/// The widest icon drawn in the small rendition, in pixels.
const SMALL: u32 = 32;
/// The squircle's rim: a white line at 10%, 4 units wide.
const RIM: (f64, f64) = (4.0, 0.10);
/// The shadow: SVG's `feGaussianBlur` (14 units) of the squircle's alpha
/// (black at 30%), 12 units down, faded to 30%; under it, as the
/// filter's merge puts it, the squircle itself in black at 30%.
const SHADOW: (f64, f64, f64) = (14.0, 12.0, 0.3);

/// An opaque colour from its `0xRRGGBB` value.
fn hex(rgb: u32) -> Rgba {
    let [_, r, g, b] = rgb.to_be_bytes();
    Rgba::from_rgba8([r, g, b, 255])
}

/// A path of the design, which the tests check reads.
fn path(data: &str) -> Path {
    Path::from_svg(data).expect("the icon's path data is valid")
}

/// The icon at `side` pixels: the bands of its size on the squircle, with
/// the shadow on Apple's grid.
fn design(side: u32, layout: IconLayout) -> ColorIcon {
    let body = path(BODY);
    let [(x0, y0, from), (x1, y1, to)] = BODY_GRADIENT;
    let gradient = LinearGradient::new(Point::new(x0, y0), Point::new(x1, y1))
        .with_stop(0.0, hex(from))
        .with_stop(1.0, hex(to));
    let small = side <= SMALL;
    let mut icon = ColorIcon::new();
    if layout == IconLayout::AppleGrid && !small {
        let (blur, drop, opacity) = SHADOW;
        let shadow = Shadow {
            blur,
            offset: Point::new(0.0, drop),
            color: Rgba::BLACK.with_alpha(opacity * opacity),
        };
        icon = icon
            .shadow(body.clone(), shadow)
            .fill(body.clone(), Rgba::BLACK.with_alpha(opacity));
    }
    icon = icon.fill(body.clone(), gradient);
    if small {
        for (band, color) in SMALL_BANDS {
            icon = icon.fill(path(BANDS[band].0), hex(color));
        }
    } else {
        for (data, color) in BANDS {
            icon = icon.fill(path(data), hex(color));
        }
    }
    let (width, opacity) = RIM;
    icon.stroke(body, width, Rgba::WHITE.with_alpha(opacity))
}

/// The part of the design that fills a `side`-pixel square in `layout`:
/// the squircle alone, or Apple's grid with its margin rounded to whole
/// pixels, so that the squircle's edges fall on pixel edges.
fn view(side: u32, layout: IconLayout) -> Rect {
    let body = GRID - 2.0 * BODY_MARGIN;
    match layout {
        IconLayout::AppleGrid => {
            let side = f64::from(side);
            let margin = (side * BODY_MARGIN / GRID).round();
            let scale = (side - 2.0 * margin) / body;
            let origin = BODY_MARGIN - margin / scale;
            Rect::new(origin, origin, side / scale, side / scale)
        }
        // Full bleed, and layouts to come.
        _ => Rect::new(BODY_MARGIN, BODY_MARGIN, body, body),
    }
}

/// The icon `side` pixels wide in `layout`, as the platforms ask for it.
fn render(side: u32, layout: IconLayout) -> Option<IconImage> {
    let rgba = design(side, layout).render(side, view(side, layout)).ok()?;
    IconImage::from_rgba(side, side, rgba).ok()
}

/// The icon of pltrs's windows (which macOS shows in the Dock), drawn at
/// each size and in each layout a platform asks for: one renderer, whose
/// clones the platforms recognise as the icon they show already.
pub fn app_icon() -> Icon {
    static ICON: LazyLock<Icon> = LazyLock::new(|| Icon::from_renderer(render));
    ICON.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The design source.
    const DESIGN: &str = include_str!("../../assets/icon/pltrs.svg");

    fn image(side: u32, layout: IconLayout) -> IconImage {
        let images = app_icon().images(&[side], layout);
        assert_eq!(images.len(), 1, "{side} pixels, {layout:?}");
        images.into_iter().next().unwrap()
    }

    fn pixel(image: &IconImage, x: u32, y: u32) -> [u8; 4] {
        let side = image.size().width;
        let at = ((y * side + x) * 4) as usize;
        image.rgba()[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn the_geometry_is_the_design_sources() {
        let squeezed = |data: &str| data.split_whitespace().collect::<Vec<_>>().join(" ");
        let source = squeezed(DESIGN);
        assert_eq!(
            source.matches(&format!("d=\"{}\"", squeezed(BODY))).count(),
            3
        );
        for (data, color) in BANDS {
            let band = format!("d=\"{}\" fill=\"#{color:06X}\"", squeezed(data));
            assert!(source.contains(&band), "{band}");
            assert!(!path(data).is_empty());
        }
        let [(x0, y0, from), (x1, y1, to)] = BODY_GRADIENT;
        assert!(source.contains(&format!("x1=\"{x0}\" y1=\"{y0}\" x2=\"{x1}\" y2=\"{y1}\"")));
        assert!(source.contains(&format!("<stop stop-color=\"#{from:06X}\"/>")));
        assert!(source.contains(&format!("<stop offset=\"1\" stop-color=\"#{to:06X}\"/>")));
        let (blur, drop, opacity) = SHADOW;
        assert!(source.contains(&format!(
            "stdDeviation=\"{blur}\"/><feOffset dy=\"{drop}\"/>"
        )));
        assert!(source.contains(&format!("slope=\"{opacity:.2}\"")));
        assert!(source.contains(&format!("fill-opacity=\"{opacity}\" filter=")));
        let (width, rim) = RIM;
        assert!(source.contains(&format!(
            "stroke=\"rgba(255,255,255,{rim:.2})\" stroke-width=\"{width}\""
        )));
        let bounds = path(BODY).extents();
        assert_eq!((bounds.x0(), bounds.y0()), (BODY_MARGIN, BODY_MARGIN));
        assert_eq!(
            (bounds.x1(), bounds.y1()),
            (GRID - BODY_MARGIN, GRID - BODY_MARGIN)
        );
    }

    #[test]
    fn every_size_is_drawn_square() {
        for side in [16, 20, 24, 32, 40, 48, 64, 128, 256, 1024] {
            for layout in [IconLayout::FullBleed, IconLayout::AppleGrid] {
                let image = image(side, layout);
                assert_eq!(image.size().width, side);
                assert_eq!(image.size().height, side);
            }
        }
    }

    #[test]
    fn apple_grid_icons_have_transparent_margins_and_a_shadow_below() {
        let icon = image(1024, IconLayout::AppleGrid);
        assert_eq!(pixel(&icon, 0, 0)[3], 0, "corner");
        assert_eq!(pixel(&icon, 512, 40)[3], 0, "above the body");
        assert_eq!(pixel(&icon, 40, 512)[3], 0, "left of the body");
        assert_eq!(pixel(&icon, 512, 512)[3], 255, "opaque body");
        assert_eq!(pixel(&icon, 512, 100)[3], 255, "the body's top edge");
        let below = pixel(&icon, 512, 930);
        let above = pixel(&icon, 512, 94);
        assert!(below[3] > 10, "shadow below: {below:?}");
        assert!(above[3] < below[3] / 2, "fainter above: {above:?}");
        assert_eq!(below[..3], [0, 0, 0], "black");
        // The body's edges fall on whole pixels at every size: 6.25
        // pixels of margin at 64 become 6.
        let small = image(64, IconLayout::AppleGrid);
        assert_eq!(pixel(&small, 32, 6)[3], 255);
        assert!(pixel(&small, 32, 5)[3] < 16);
        assert!(pixel(&small, 32, 58)[3] > 0, "the shadow below");
    }

    #[test]
    fn the_gradient_runs_between_its_end_colours() {
        // On the grid at 1024 pixels, a pixel is a unit.
        let icon = image(1024, IconLayout::AppleGrid);
        let [(x0, y0, from), (x1, y1, to)] = BODY_GRADIENT;
        let (dx, dy) = (x1 - x0, y1 - y0);
        // Near the gradient's ends, clear of the bands.
        for (x, y) in [(174, 850), (850, 200)] {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let t = ((px - x0) * dx + (py - y0) * dy) / (dx * dx + dy * dy);
            let [a, b] = [hex(from), hex(to)].map(Rgba::to_rgba8);
            let expected: Vec<f64> = (0..3)
                .map(|i| f64::from(a[i]) + (f64::from(b[i]) - f64::from(a[i])) * t)
                .collect();
            let got = pixel(&icon, x, y);
            for (channel, want) in got.iter().zip(&expected) {
                assert!(
                    (f64::from(*channel) - want).abs() <= 2.0,
                    "({x}, {y}): {got:?} vs {expected:?}"
                );
            }
        }
    }

    #[test]
    fn full_bleed_icons_fill_their_square() {
        let icon = image(256, IconLayout::FullBleed);
        assert_eq!(pixel(&icon, 128, 0)[3], 255, "top edge");
        assert_eq!(pixel(&icon, 0, 128)[3], 255, "left edge");
        assert_eq!(pixel(&icon, 0, 0)[3], 0, "rounded corner");
        let peak = pixel(&icon, 149, 105);
        assert!(peak[1] > 180 && peak[2] < 100, "the lime peak: {peak:?}");
    }

    #[test]
    fn small_icons_have_bolder_bands_and_no_shadow() {
        for side in [16, 32] {
            let icon = image(side, IconLayout::AppleGrid);
            let last = side - 1;
            assert_eq!(pixel(&icon, side / 2, last)[3], 0, "no shadow");
            // The peak takes the second innermost band's place, in lime.
            let peak = pixel(&icon, side * 9 / 16, side * 7 / 16);
            assert!(peak[1] > 170 && peak[2] < 100, "{side}: {peak:?}");
        }
    }

    #[test]
    fn the_icon_is_one_renderer() {
        assert_eq!(app_icon(), app_icon());
    }
}
