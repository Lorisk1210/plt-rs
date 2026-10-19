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
