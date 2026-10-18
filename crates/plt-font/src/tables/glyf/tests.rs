use super::*;
use crate::outline::OutlinePoint;

/// Encodes a simple glyph with uncompressed coordinates.
pub(crate) fn simple_glyph(contours: &[&[(i16, i16, bool)]]) -> Vec<u8> {
    instructed_glyph(contours, &[])
}

/// Encodes a simple glyph with uncompressed coordinates and a program.
pub(crate) fn instructed_glyph(contours: &[&[(i16, i16, bool)]], instructions: &[u8]) -> Vec<u8> {
    let points: Vec<_> = contours.iter().flat_map(|c| c.iter().copied()).collect();
    let mut out = (contours.len() as i16).to_be_bytes().to_vec();
    let xs = points.iter().map(|p| p.0);
    let ys = points.iter().map(|p| p.1);
    for v in [xs.clone().min(), ys.clone().min(), xs.max(), ys.max()] {
        out.extend_from_slice(&v.unwrap_or(0).to_be_bytes());
    }
    let mut end = 0u16;
    for contour in contours {
        end += contour.len() as u16;
        out.extend_from_slice(&(end - 1).to_be_bytes());
    }
    out.extend_from_slice(&(instructions.len() as u16).to_be_bytes());
    out.extend_from_slice(instructions);
    out.extend(points.iter().map(|p| u8::from(p.2)));
    let mut previous = (0i16, 0i16);
    let mut ys = Vec::new();
    for p in &points {
        out.extend_from_slice(&(p.0 - previous.0).to_be_bytes());
        ys.extend_from_slice(&(p.1 - previous.1).to_be_bytes());
        previous = (p.0, p.1);
    }
    out.extend_from_slice(&ys);
    out
}

/// Builds `(loca, glyf)` in long format from glyph records.
pub(crate) fn tables(glyphs: &[Vec<u8>]) -> (Vec<u8>, Vec<u8>) {
    let mut loca = Vec::new();
    let mut glyf = Vec::new();
    for glyph in glyphs {
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        glyf.extend_from_slice(glyph);
    }
    loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
    (loca, glyf)
}

fn load(glyf: &Glyf<'_>, glyph: u16, scaling: Scaling) -> Result<Outline> {
    let mut out = Outline::default();
    glyf.load(GlyphId(glyph), scaling, &|_| 0, &mut out)?;
    Ok(out)
}

fn square() -> Vec<u8> {
    simple_glyph(&[&[
        (0, 0, true),
        (100, 0, true),
        (100, 100, false),
        (0, 100, true),
    ]])
}

#[test]
fn simple_glyph_points() {
    let (loca, data) = tables(&[Vec::new(), square()]);
    let glyf = Glyf::new(&loca, &data, true);
    assert_eq!(load(&glyf, 0, Scaling::Units).unwrap().points, Vec::new());
    let outline = load(&glyf, 1, Scaling::Units).unwrap();
    assert_eq!(outline.contour_ends, vec![4]);
    assert_eq!(
        outline.points[2],
        OutlinePoint {
            x: 100.0,
            y: 100.0,
            kind: PointKind::Quadratic
        }
    );
    let (header, _) = glyf.record(GlyphId(1)).unwrap().unwrap();
    assert_eq!(
        header.bbox,
        Rect {
            x_min: 0,
            y_min: 0,
            x_max: 100,
            y_max: 100
        }
    );
    let scaled = load(
        &glyf,
        1,
        Scaling::Fixed {
            x_scale: 0x8000,
            y_scale: 0x1_0000,
        },
    )
    .unwrap();
    assert_eq!((scaled.points[1].x, scaled.points[2].y), (50.0, 100.0));
}

#[test]
fn compressed_flags_and_coordinates() {
    // One contour, three points; flags use repeat, short and same bits.
    let mut glyph = vec![0, 1, 0, 0, 0, 0, 0, 10, 0, 10, 0, 2, 0, 0];
    glyph.extend_from_slice(&[
        ON_CURVE | X_SHORT | Y_SHORT | X_SAME_OR_POSITIVE | Y_SAME_OR_POSITIVE | REPEAT,
        1,
    ]);
    glyph.push(ON_CURVE | X_SAME_OR_POSITIVE | Y_SHORT);
    glyph.extend_from_slice(&[5, 5, 5, 5, 3]);
    let (loca, data) = tables(&[glyph]);
    let glyf = Glyf::new(&loca, &data, true);
    let outline = load(&glyf, 0, Scaling::Units).unwrap();
    let xy: Vec<_> = outline.points.iter().map(|p| (p.x, p.y)).collect();
    assert_eq!(xy, vec![(5.0, 5.0), (10.0, 10.0), (10.0, 7.0)]);
}

/// Encodes a composite glyph from `(flags, glyph, arguments)` components.
pub(crate) fn composite(components: &[(u16, u16, &[u8])]) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0];
    for (index, (flags, glyph, args)) in components.iter().enumerate() {
        let more = if index + 1 < components.len() {
            MORE_COMPONENTS
        } else {
            0
        };
        out.extend_from_slice(&(flags | more).to_be_bytes());
        out.extend_from_slice(&glyph.to_be_bytes());
        out.extend_from_slice(args);
    }
    out
}

#[test]
fn composite_offsets_and_transforms() {
    let comp = composite(&[
        (ARGS_ARE_XY_VALUES, 0, &[10, 0xFE]),
        (
            ARGS_ARE_XY_VALUES | ARGS_ARE_WORDS | HAVE_SCALE,
            0,
            &[0x01, 0x00, 0, 0, 0x20, 0x00],
        ),
    ]);
    let (loca, data) = tables(&[square(), comp]);
    let glyf = Glyf::new(&loca, &data, true);
    let outline = load(&glyf, 1, Scaling::Units).unwrap();
    assert_eq!(outline.contour_ends, vec![4, 8]);
    assert_eq!((outline.points[1].x, outline.points[1].y), (110.0, -2.0));
    assert_eq!((outline.points[5].x, outline.points[5].y), (306.0, 0.0));
}

#[test]
fn composite_two_by_two_and_anchor() {
    let comp = composite(&[
        (
            ARGS_ARE_XY_VALUES | HAVE_TWO_BY_TWO,
            0,
            &[0, 0, 0, 0, 0x40, 0, 0xC0, 0, 0, 0],
        ),
        (0, 0, &[1, 0]),
    ]);
    let (loca, data) = tables(&[square(), comp]);
    let glyf = Glyf::new(&loca, &data, true);
    let outline = load(&glyf, 1, Scaling::Units).unwrap();
    // Rotation by 90°: (100, 0) -> (0, 100) under [xx=0, yx=1, xy=-1, yy=0].
    assert_eq!((outline.points[1].x, outline.points[1].y), (0.0, 100.0));
    // Second component's point 0 aligned with the first's point 1.
    assert_eq!((outline.points[4].x, outline.points[4].y), (0.0, 100.0));
}

#[test]
fn malformed_glyphs_error() {
    let recursive = composite(&[(ARGS_ARE_XY_VALUES, 0, &[0, 0])]);
    let (loca, data) = tables(&[recursive]);
    let glyf = Glyf::new(&loca, &data, true);
    assert!(load(&glyf, 0, Scaling::Units).is_err());

    let bad_anchor = composite(&[(0, 0, &[9, 9])]);
    let (loca, data) = tables(&[square(), bad_anchor]);
    let glyf = Glyf::new(&loca, &data, true);
    assert!(load(&glyf, 1, Scaling::Units).is_err());

    let full = square();
    for len in 1..full.len() {
        let (loca, data) = tables(&[full[..len].to_vec()]);
        let glyf = Glyf::new(&loca, &data, true);
        assert!(load(&glyf, 0, Scaling::Units).is_err(), "len {len}");
    }
    let (loca, data) = tables(&[square()]);
    let glyf = Glyf::new(&loca[..4], &data, true);
    assert!(load(&glyf, 0, Scaling::Units).is_err());
}

#[test]
fn exponential_composites_are_rejected() {
    // Glyph n + 1 places glyph n eight times: 8^16 loads at full depth.
    let eight = |glyph: u16| composite(&[(ARGS_ARE_XY_VALUES, glyph, &[0u8, 0][..]); 8]);
    let mut empties = vec![Vec::new()];
    empties.extend((0..16).map(eight));
    let (loca, data) = tables(&empties);
    let glyf = Glyf::new(&loca, &data, true);
    assert!(load(&glyf, 16, Scaling::Units).is_err());
    // With a square at the bottom the points run out first.
    let mut squares = vec![square()];
    squares.extend((0..16).map(eight));
    let (loca, data) = tables(&squares);
    let glyf = Glyf::new(&loca, &data, true);
    assert!(load(&glyf, 16, Scaling::Units).is_err());
    assert_eq!(load(&glyf, 2, Scaling::Units).unwrap().points.len(), 4 * 64);
}

#[test]
fn decreasing_contour_ends_are_rejected() {
    let mut glyph = simple_glyph(&[&[(0, 0, true), (1, 1, true)], &[(2, 2, true)]]);
    glyph[10..14].copy_from_slice(&[0, 2, 0, 1]);
    let (loca, data) = tables(&[glyph]);
    let glyf = Glyf::new(&loca, &data, true);
    assert!(load(&glyf, 0, Scaling::Units).is_err());
}

#[test]
fn short_loca_format() {
    let glyph = square();
    let loca = [0, 0, 0, (glyph.len() / 2) as u8];
    let glyf = Glyf::new(&loca, &glyph, false);
    assert_eq!(load(&glyf, 0, Scaling::Units).unwrap().points.len(), 4);
}
