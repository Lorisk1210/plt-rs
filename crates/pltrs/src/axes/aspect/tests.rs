use super::*;
use crate::figure::Figure;

fn axes_with_limits(x: (f64, f64), y: (f64, f64)) -> (Figure, Axes) {
    let fig = Figure::new();
    let ax = fig
        .add_axes([0.125, 0.11, 0.775, 0.77])
        .expect("a valid rectangle");
    ax.set_xlim(x.0, x.1)
        .and_then(|ax| ax.set_ylim(y.0, y.1))
        .expect("finite limits");
    (fig, ax)
}

#[test]
fn box_adjustable_shrinks_and_centres_the_axes() {
    let (_fig, ax) = axes_with_limits((0.0, 10.0), (0.0, 10.0));
    ax.set_aspect(Aspect::Equal).unwrap();
    let (x, y, w, h) = ax.position().bounds();
    assert!((w * 640.0 - h * 480.0).abs() < 1e-9, "square on screen");
    assert!((h - 0.77).abs() < 1e-12);
    assert!((x + w / 2.0 - (0.125 + 0.775 / 2.0)).abs() < 1e-12);
    assert_eq!(y, 0.11);
    assert_eq!(ax.original_position().bounds(), (0.125, 0.11, 0.775, 0.77));
}

#[test]
fn locators_shrink_and_shift_with_matplotlibs_arithmetic() {
    // A horizontal colorbar of a 5-inch figure with 5% extensions:
    // matplotlib's `shrunk(...).translated(...)` puts the extension tip
    // at 62.49999999999999 pixels, which snapping rounds down.
    let locator = AxesLocator {
        shrink: (1.0 / 1.1, 1.0),
        offset: (0.05 / 1.1, 0.0),
        box_aspect: None,
    };
    let located = locator.locate(&Bbox::from_bounds(0.125, 0.11, 0.775, 0.07));
    let (x0, width) = (located.x0() * 500.0, located.width() * 500.0);
    assert_eq!(x0 - 0.05 * width, 62.499_999_999_999_99);
    assert_eq!(located.y0(), 0.11);
}

#[test]
fn anchors_place_the_shrunken_box() {
    let (_fig, ax) = axes_with_limits((0.0, 10.0), (0.0, 10.0));
    ax.set_aspect(Aspect::Equal).unwrap().set_anchor(Anchor::W);
    assert_eq!(ax.position().x0(), 0.125);
    ax.set_anchor(Anchor::E);
    assert!((ax.position().x1() - 0.9).abs() < 1e-12);
}

#[test]
fn datalim_widens_the_view() {
    let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
    ax.set_aspect(Aspect::Equal)
        .unwrap()
        .set_adjustable(Adjustable::Datalim)
        .unwrap()
        .apply_aspect();
    // Values from matplotlib: without data, y shrinks about its centre.
    assert_eq!(ax.xlim(), (0.0, 1.0));
    assert_eq!(
        ax.ylim(),
        (0.127_419_354_838_709_7, 0.872_580_645_161_290_3)
    );
    assert_eq!(ax.position(), ax.original_position());
    let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
    ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
    ax.set_autoscale_on(true);
    ax.set_aspect(Aspect::Equal)
        .unwrap()
        .set_adjustable(Adjustable::Datalim)
        .unwrap();
    let (x0, x1) = ax.apply_aspect().xlim();
    assert!((x0 + 0.238_095_238_095_238_25).abs() < 1e-12);
    assert!((x1 - 1.238_095_238_095_238_1).abs() < 1e-12);
    assert_eq!(ax.ylim(), (-0.05, 1.05));
}

#[test]
fn box_aspect_fixes_the_box_shape() {
    let (_fig, ax) = axes_with_limits((0.0, 3.0), (0.0, 1.0));
    ax.set_box_aspect(Some(1.0)).unwrap();
    assert_eq!(ax.adjustable(), Adjustable::Datalim);
    let (_, _, w, h) = ax.position().bounds();
    assert!((w * 640.0 - h * 480.0).abs() < 1e-9);
    assert_eq!(ax.xlim(), (0.0, 3.0), "auto aspect leaves the limits");
}

#[test]
fn auto_restores_the_original_position() {
    let (_fig, ax) = axes_with_limits((0.0, 10.0), (0.0, 1.0));
    ax.set_aspect(Aspect::Ratio(2.0)).unwrap();
    assert_ne!(ax.position(), ax.original_position());
    ax.set_aspect(Aspect::Auto).unwrap();
    assert_eq!(ax.position(), ax.original_position());
    assert_eq!(ax.aspect(), Aspect::Auto);
    ax.set_position([0.2, 0.2, 0.5, 0.5]);
    assert_eq!(ax.position(), Bbox::from_bounds(0.2, 0.2, 0.5, 0.5));
}

#[test]
fn invalid_ratios_are_rejected() {
    let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(ax.set_aspect(Aspect::Ratio(bad)).is_err(), "{bad}");
        assert!(ax.set_box_aspect(Some(bad)).is_err(), "{bad}");
    }
    assert_eq!(ax.aspect(), Aspect::Auto);
    assert!(
        AspectError::NotPositive(0.0)
            .to_string()
            .contains("positive")
    );
}

#[test]
fn data_ratio_uses_scaled_bounds() {
    let (_fig, ax) = axes_with_limits((0.0, 4.0), (2.0, 0.0));
    assert_eq!(ax.data_ratio(), 0.5);
}

#[test]
fn a_shared_aspect_reaches_the_axes_sharing_an_axis() {
    // matplotlib's test_shared_with_aspect_1.
    let fig = Figure::new();
    let axs = fig
        .subplots_shared(2, 1, crate::figure::Share::All, crate::figure::Share::None)
        .unwrap();
    let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
    top.set_aspect(Aspect::Ratio(2.0)).unwrap();
    assert_eq!(bottom.aspect(), Aspect::Auto, "unshared by default");
    top.set_shared_aspect(Aspect::Ratio(2.0), Adjustable::Datalim)
        .unwrap();
    assert_eq!(
        (bottom.aspect(), bottom.adjustable()),
        (Aspect::Ratio(2.0), Adjustable::Datalim)
    );
    assert!(
        top.set_shared_aspect(Aspect::Ratio(-1.0), Adjustable::Box)
            .is_err()
    );
}

#[test]
fn reset_position_undoes_the_aspect_and_leaves_the_layout() {
    let (_fig, ax) = axes_with_limits((0.0, 10.0), (0.0, 1.0));
    ax.set_aspect(Aspect::Ratio(2.0)).unwrap();
    assert_ne!(ax.position(), ax.original_position());
    ax.reset_position();
    let state = |ax: &Axes| {
        ax.raw
            .with::<AxesState, _>(|a| a.aspect.active_position(&a.position))
    };
    assert_eq!(state(&ax), ax.original_position());
    assert!(!ax.in_layout());
}

#[test]
fn axes_locators_place_the_axes_before_its_aspect() {
    let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
    assert!(ax.axes_locator().is_none());
    let locator = AxesLocatorFn::new(|original: Bbox| {
        let (x0, y0, width, height) = original.bounds();
        Bbox::from_bounds(x0 + width / 2.0, y0, width / 2.0, height / 2.0)
    });
    ax.set_axes_locator(Some(locator.clone()));
    assert_eq!(ax.axes_locator(), Some(locator));
    let (x0, y0, width, height) = ax.position().bounds();
    assert!((x0 - (0.125 + 0.775 / 2.0)).abs() < 1e-12 && (y0 - 0.11).abs() < 1e-12);
    assert!((width - 0.775 / 2.0).abs() < 1e-12 && (height - 0.385).abs() < 1e-12);
    ax.set_aspect(Aspect::Equal).unwrap();
    let (_, _, width, height) = ax.position().bounds();
    assert!(
        (width * 640.0 - height * 480.0).abs() < 1e-9,
        "the aspect inside it"
    );
    ax.set_axes_locator(None);
    ax.set_aspect(Aspect::Auto).unwrap();
    assert_eq!(ax.position(), ax.original_position());
}
