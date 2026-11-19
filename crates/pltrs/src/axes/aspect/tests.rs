use super::*;
use crate::figure::{Figure, FigureState};

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
fn an_axes_the_figure_does_not_list_applies_its_aspect_in_the_figure() {
    // matplotlib's `apply_aspect` of an axes made for a figure but never
    // added (pltrs makes it removed), on a square figure: the box keeps
    // the 2:1 view's aspect, centred where it was.
    let fig = Figure::with_size(3.0, 3.0, 50.0);
    let ax = fig
        .add_axes([0.1, 0.1, 0.8, 0.8])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 2.0).expect("finite limits");
    ax.set_aspect(Aspect::Equal).unwrap();
    ax.remove();
    ax.apply_aspect().expect("the aspect applies");
    let (x, y, w, h) = ax.position().bounds();
    assert!((x - 0.1).abs() < 1e-12 && (w - 0.8).abs() < 1e-12);
    assert!(
        (y - 0.3).abs() < 1e-12 && (h - 0.4).abs() < 1e-12,
        "{y} {h}"
    );
    assert!(fig.axes().is_empty(), "still apart from the figure");
}

#[test]
fn a_given_position_replaces_the_located_one() {
    // matplotlib's `apply_aspect(position)`: a square on a 2:1 figure
    // spanning its height, centred.
    let fig = Figure::with_size(4.0, 2.0, 100.0);
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal).unwrap();
    ax.apply_aspect_at(Bbox::from_bounds(0.0, 0.0, 1.0, 1.0))
        .unwrap();
    let active = ax
        .raw()
        .with::<AxesState, _>(|a| a.aspect.active_position(&a.position));
    assert_eq!(active.bounds(), (0.25, 0.0, 0.5, 1.0));
    ax.apply_aspect().unwrap();
    let active = ax
        .raw()
        .with::<AxesState, _>(|a| a.aspect.active_position(&a.position));
    assert!(active.y0() > 0.1, "back in the subplot: {active:?}");
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
        .apply_aspect()
        .unwrap();
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
    let (x0, x1) = ax.apply_aspect().unwrap().xlim();
    assert!((x0 + 0.238_095_238_095_238_25).abs() < 1e-12);
    assert!((x1 - 1.238_095_238_095_238_1).abs() < 1e-12);
    assert_eq!(ax.ylim(), (-0.05, 1.05));
}

#[test]
fn box_aspect_fixes_the_box_shape() {
    let (_fig, ax) = axes_with_limits((0.0, 3.0), (0.0, 1.0));
    ax.set_box_aspect(Some(1.0));
    assert_eq!(ax.adjustable(), Adjustable::Datalim);
    let (_, _, w, h) = ax.position().bounds();
    assert!((w * 640.0 - h * 480.0).abs() < 1e-9);
    assert_eq!(ax.xlim(), (0.0, 3.0), "auto aspect leaves the limits");
}

#[test]
fn box_aspects_are_kept_and_fail_when_applied_unless_positive() {
    let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
    for ratio in [-1.5, 0.0] {
        ax.set_box_aspect(Some(ratio));
        assert_eq!(ax.box_aspect(), Some(ratio));
        assert_eq!(ax.apply_aspect().err(), Some(AspectError::BoxNotPositive));
    }
    ax.set_box_aspect(Some(f64::NAN));
    ax.apply_aspect().unwrap();
    assert!(
        ax.position().width().is_nan(),
        "a NaN aspect leaves no size"
    );
}

#[test]
fn twinned_axes_adjust_their_data_limits_never_their_box() {
    // matplotlib's `apply_aspect`, at the draw too: its `RuntimeError`,
    // the box left where it was; `datalim` keeps working.
    let (fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 2.0));
    let twin = ax.twinx().expect("a rectilinear axes twins");
    assert_eq!(
        ax.adjustable(),
        Adjustable::Datalim,
        "twins adjust their limits"
    );
    ax.set_aspect(Aspect::Equal).unwrap();
    ax.set_adjustable(Adjustable::Box).unwrap();
    assert_eq!(ax.apply_aspect().err(), Some(AspectError::BoxInTwinnedAxes));
    assert!(twin.apply_aspect().is_ok(), "the twin's aspect is auto");
    let Err(crate::figure::SaveError::Draw(errors)) = fig.to_png() else {
        panic!("the draw refuses the aspect");
    };
    assert_eq!(errors.0[0].kind, crate::artist::DrawErrorKind::Library);
    assert_eq!(
        errors.0[0].message,
        "Adjustable 'box' is not allowed in a twinned Axes; use 'datalim' instead"
    );
    ax.set_adjustable(Adjustable::Datalim).unwrap();
    ax.apply_aspect().expect("twins adjust their data limits");
    assert_eq!(ax.position(), ax.original_position());
}

#[test]
fn axes_sharing_both_axes_refuse_to_adjust_their_data_limits() {
    // matplotlib's `apply_aspect`, at the draw too: its `RuntimeError` in
    // its words, once for the axes however often the draw lays it out.
    let fig = Figure::new();
    let ax = fig.add_subplot(1, 2, 1).expect("a subplot");
    let other = fig.add_subplot(1, 2, 2).expect("a subplot");
    other.share_x(&ax).expect("rectilinear axes share");
    other.share_y(&ax).expect("rectilinear axes share");
    ax.plot([0.0, 1.0], [0.0, 2.0]).expect("finite data");
    ax.set_aspect(Aspect::Equal).unwrap();
    ax.set_adjustable(Adjustable::Datalim).unwrap();
    assert_eq!(
        ax.apply_aspect().err(),
        Some(AspectError::DatalimBothShared)
    );
    let Err(crate::figure::SaveError::Draw(errors)) = fig.to_png() else {
        panic!("the draw refuses the aspect");
    };
    assert_eq!(errors.0.len(), 1);
    assert_eq!(errors.0[0].kind, crate::artist::DrawErrorKind::Library);
    assert_eq!(
        errors.0[0].message,
        "set_aspect(..., adjustable='datalim') or axis('equal') are not allowed when both axes \
         are shared.  Try set_aspect(..., adjustable='box')."
    );
    ax.set_adjustable(Adjustable::Box).unwrap();
    ax.apply_aspect().expect("shared axes adjust their boxes");
}

#[test]
fn a_fixed_aspect_autoscales_the_view_it_shares_first_when_drawn() {
    // matplotlib's `Figure.draw` applies the aspect of each axes in turn
    // before laying them out: the datalim axes reads its view first, so
    // the shared y limits take its margins, not the tight axes' none.
    let fig = Figure::new();
    let tight = fig.add_subplot(1, 2, 1).expect("a subplot");
    let fixed = fig.add_subplot(1, 2, 2).expect("a subplot");
    fixed.share_y(&tight).expect("rectilinear axes share");
    tight.plot([0.0, 1.0], [0.0, 10.0]).expect("finite data");
    tight.autoscale(None, Some(true));
    fixed.plot([0.0, 1.0], [0.0, 10.0]).expect("finite data");
    fixed.set_aspect(Aspect::Ratio(0.5)).unwrap();
    fixed.set_adjustable(Adjustable::Datalim).unwrap();
    fig.to_png().expect("the figure draws");
    assert_eq!(tight.ylim(), (-0.5, 10.5));
    assert_eq!(fixed.ylim(), (-0.5, 10.5));
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

#[test]
fn positions_programs_locate_child_axes_at_are_in_figure_fractions() {
    // ultraplot's colorbar axes: a child covering its host, placed by a
    // locator of the program's computing the host's box in figure
    // fractions; and a child placed elsewhere.
    let fig = Figure::new();
    let host = fig
        .add_axes([0.1, 0.1, 0.8, 0.2])
        .expect("a valid rectangle");
    for bounds in [[0.1, 0.1, 0.8, 0.2], [0.2, 0.15, 0.4, 0.1]] {
        let child = fig.removed_axes(bounds).expect("a valid rectangle");
        host.add_child_axes(&child).expect("added");
        let [x, y, width, height] = bounds;
        child.set_axes_locator(Some(AxesLocatorFn::at(Some(Bbox::from_bounds(
            x, y, width, height,
        )))));
        let (px, py, pwidth, pheight) = child.position().bounds();
        for (placed, given) in [(px, x), (py, y), (pwidth, width), (pheight, height)] {
            assert!(
                (placed - given).abs() < 1e-12,
                "{bounds:?}: {placed} != {given}"
            );
        }
    }
}

#[test]
fn datalim_collapsing_the_view_warns_as_set_xbound_does() {
    // An infinite box aspect asks for an x view of no width: matplotlib's
    // `set_xbound` warns about the identical limits before the locator
    // widens them; on a log axis the non-positive bounds keep the current
    // limits first, which are not identical (values from matplotlib).
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = std::rc::Rc::clone(&seen);
    crate::warnings::set_handler(move |message| sink.borrow_mut().push(message.to_owned()));
    let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
    ax.set_aspect(Aspect::Ratio(1.5))
        .unwrap()
        .set_adjustable(Adjustable::Datalim)
        .unwrap()
        .set_box_aspect(Some(f64::INFINITY))
        .apply_aspect()
        .unwrap();
    let identical = "Attempting to set identical low and high xlims makes transformation \
                     singular; automatically expanding.";
    assert_eq!(*seen.borrow(), [identical]);
    seen.borrow_mut().clear();
    let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
    ax.set_xscale("log").expect("a known scale");
    ax.set_aspect(Aspect::Ratio(1.5))
        .unwrap()
        .set_adjustable(Adjustable::Datalim)
        .unwrap()
        .set_box_aspect(Some(f64::INFINITY))
        .apply_aspect()
        .unwrap();
    crate::warnings::clear_handler();
    assert!(seen.borrow().iter().all(|message| message != identical));
}

#[test]
fn an_aspect_that_cannot_be_applied_warns_once_for_each_draw() {
    let (fig, ax) = axes_with_limits((-1e308, 1e308), (-1e308, 1e308));
    ax.set_aspect(Aspect::Ratio(1.0))
        .unwrap()
        .set_adjustable(Adjustable::Datalim)
        .unwrap();
    let overflows = |work: &dyn Fn()| {
        let ((), warned) = crate::warnings::capturing_runtime(work);
        (warned.iter())
            .filter(|warning| *warning == "overflow encountered in scalar subtract")
            .count()
    };
    // Measured, then drawn: matplotlib's draw raises at the first
    // application, whose two sizes overflow.
    let drawn = overflows(&|| {
        fig.state_cell().work_on(FigureState::prepare);
        assert!(fig.canvas().draw().is_err());
    });
    assert_eq!(drawn, 2);
    assert_eq!(
        overflows(&|| assert!(fig.canvas().draw().is_err())),
        2,
        "again"
    );
}
