use super::*;
use crate::colors::Normalized::{Fraction, Index, Masked};

#[test]
fn registries_build_every_colormap() {
    for name in multivar_colormap_names() {
        assert_eq!(multivar_colormap(name).unwrap().name(), name);
    }
    for name in bivar_colormap_names() {
        assert_eq!(bivar_colormap(name).unwrap().size(), (256, 256));
    }
    assert_eq!(
        bivar_colormap("nope"),
        Err(MultivarError::Unknown("nope".into()))
    );
}

#[test]
fn multivariate_colours_add_or_subtract() {
    let add = multivar_colormap("2VarAddA").unwrap();
    let low = add.lookup(&[Fraction(0.0), Fraction(0.0)], None).unwrap();
    assert_eq!(low.to_array(), [0.0, 0.0, 0.0, 1.0]);
    let bad = add.lookup(&[Masked, Fraction(0.5)], None).unwrap();
    assert_eq!(bad.to_array(), [0.0; 4]);
    assert!(matches!(
        add.lookup(&[Fraction(0.0)], None),
        Err(MultivarError::VariateCount {
            expected: 2,
            given: 1
        })
    ));
    let sub = multivar_colormap("2VarSubA").unwrap();
    let raw = sub.mix(&[Fraction(0.0), Fraction(0.0)]).unwrap();
    assert!(raw[0] > 0.9, "complements of white add up to white");
    assert!(
        MultivarColormap::new(
            "one",
            vec![add.colormaps()[0].clone()],
            CombinationMode::Add
        )
        .is_err()
    );
}

#[test]
fn bivariate_shapes_clip_or_mark_the_outside() {
    let square = bivar_colormap("BiOrangeBlue").unwrap();
    let clipped = square.lookup(Fraction(2.0), Fraction(-1.0), None).unwrap();
    assert_eq!(
        clipped,
        square.lookup(Fraction(1.0), Fraction(0.0), None).unwrap()
    );
    let ignoring = square
        .clone()
        .with_extremes(None, None, Some(BivarShape::Ignore), None)
        .unwrap();
    assert_eq!(
        ignoring.lookup(Fraction(2.0), Fraction(0.5), None).unwrap(),
        ignoring.outside()
    );
    assert_eq!(
        square.lookup(Index(255), Index(0), Some(0.25)).unwrap().a,
        0.25
    );
    let cone = bivar_colormap("BiCone").unwrap();
    assert_eq!(
        cone.lookup(Index(0), Fraction(0.0), None),
        Err(MultivarError::CircleNeedsFractions)
    );
    assert_eq!(
        cone.lookup(Masked, Fraction(0.9), None),
        Ok(cone.bad()),
        "masked fractions are bad, not rejected"
    );
    assert_eq!(
        cone.lut()[(0, 0)][3],
        0.0,
        "corners outside the disc are clear"
    );
}

#[test]
fn bivariate_tables_resample_reverse_and_transpose() {
    let cmap = bivar_colormap("BiOrangeBlue").unwrap();
    let small = cmap.resampled((Some(3), Some(4)), false).unwrap();
    assert_eq!(small.size(), (3, 4));
    let reversed = cmap.reversed(true, false).unwrap();
    assert_eq!(reversed.lut()[(0, 0)], cmap.lut()[(255, 0)]);
    let transposed = cmap.transposed().unwrap();
    assert_eq!(transposed.lut()[(10, 200)], cmap.lut()[(200, 10)]);
    let along = cmap.component(BivarAxis::Second).unwrap();
    assert_eq!((along.n(), along.name()), (256, "BiOrangeBlue_1"));
}

#[test]
fn combination_modes_parse_from_matplotlib_names() {
    assert_eq!("sRGB_add".parse(), Ok(CombinationMode::Add));
    assert_eq!("sRGB_sub".parse(), Ok(CombinationMode::Sub));
    assert_eq!(
        "bad".parse::<CombinationMode>(),
        Err(MultivarError::UnknownMode("bad".into()))
    );
}

#[test]
fn equality_ignores_names_and_origins() {
    let add = multivar_colormap("2VarAddA").unwrap();
    let renamed =
        MultivarColormap::new("other", add.colormaps().to_vec(), CombinationMode::Add).unwrap();
    assert_eq!(add, renamed);
    let sub =
        MultivarColormap::new("2VarAddA", add.colormaps().to_vec(), CombinationMode::Sub).unwrap();
    assert_ne!(add, sub);
    let peak = bivar_colormap("BiPeak").unwrap();
    let moved = peak
        .clone()
        .with_extremes(None, None, None, Some((0.1, 0.9)))
        .unwrap();
    assert_eq!(peak, moved);
    let darker = peak
        .clone()
        .with_extremes(Some("k".into()), None, None, None)
        .unwrap();
    assert_ne!(peak, darker);
}

#[test]
fn bytes_truncate_like_numpy() {
    let sub = multivar_colormap("2VarSubA").unwrap();
    assert_eq!(
        sub.lookup_bytes(&[Fraction(0.0), Fraction(0.0)], None)
            .unwrap(),
        [255; 4]
    );
    let square = bivar_colormap("BiOrangeBlue").unwrap();
    let half = square
        .lookup_bytes(Fraction(1.0), Fraction(1.0), Some(0.5))
        .unwrap();
    assert_eq!(half, [255, 255, 255, 127]);
    assert_eq!(
        square
            .lookup_bytes(Masked, Fraction(0.0), Some(0.5))
            .unwrap(),
        [0; 4],
        "transparent bad colours ignore alpha"
    );
}

#[test]
fn image_tables_take_rgb_and_bytes() {
    let rgb = Grid::from_fn(2, 2, |r, c| [r as f64, c as f64, 0.5]);
    let cmap = BivarColormap::from_image("rgb", rgb, BivarShape::Square, (0.0, 0.0)).unwrap();
    assert_eq!(
        cmap.lookup(Index(1), Index(0), None).unwrap().to_array(),
        [1.0, 0.0, 0.5, 1.0]
    );
    let bytes = Grid::from_fn(1, 2, |_, c| [255, 0, 51 * c as u8]);
    let cmap = BivarColormap::from_image("bytes", bytes, BivarShape::Square, (0.0, 0.0)).unwrap();
    let [r, _, b, a] = cmap.lookup(Index(0), Index(1), None).unwrap().to_array();
    assert_eq!((r, a), (1.0, 1.0));
    assert_eq!(b, f64::from(51.0_f32 / 255.0));
}
