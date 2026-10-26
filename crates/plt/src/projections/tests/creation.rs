//! Axes of registered projections made with options, and wherever axes
//! are made: subplot grids, mosaics, insets, twins and pyplot.

use super::{Disc, Sheared};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxisName, PlotError, ProjectionKind};
use crate::figure::{Figure, Mosaic, MosaicError, MosaicOptions, Share};
use crate::projections::{
    Projection, ProjectionError, ProjectionSpec, ProjectionView, register, register_with,
};
use crate::pyplot;
use crate::transforms::{Affine2D, ResolvedTransform};

/// Rectilinear axes whose data keeps `pad` off every edge.
#[derive(Clone, Debug)]
struct Padded {
    pad: f64,
}

/// The options [`Padded`] is made with.
#[derive(Debug, PartialEq)]
struct Pad(f64);

impl Default for Pad {
    fn default() -> Self {
        Self(0.05)
    }
}

impl Projection for Padded {
    fn name(&self) -> &'static str {
        "padded"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        let scale = 1.0 - 2.0 * self.pad;
        let inset = Affine2D::scaling(scale, scale).translate(self.pad, self.pad);
        view.rectilinear().then(ResolvedTransform::affine(inset))
    }
}

/// `projection="padded"` with `pad`.
fn padded(pad: f64) -> ProjectionSpec {
    let kind = register_with(|&Pad(pad): &Pad| Padded { pad }).unwrap();
    kind.with_options(Pad(pad))
}

/// The pad of `ax`, if it is padded.
fn pad_of(ax: &Axes) -> Option<f64> {
    ax.with_projection(|padded: &Padded| padded.pad)
}

#[test]
fn options_reach_the_projection_when_axes_are_made() {
    let fig = Figure::new();
    let ax = fig
        .add_subplot_with_projection(1, 2, 1, padded(0.2))
        .unwrap();
    assert_eq!(pad_of(&ax), Some(0.2));
    let plain = fig
        .add_subplot_with_projection(1, 2, 2, padded(0.2).kind())
        .unwrap();
    assert_eq!(pad_of(&plain), Some(0.05), "made with the default options");
    assert_eq!(ax.made_with(), padded(0.2));
    ax.clear();
    assert_eq!(
        (pad_of(&ax), ax.made_with()),
        (Some(0.2), padded(0.2)),
        "clearing keeps the projection and its options"
    );
    let refused =
        fig.add_axes_with_projection([0.1, 0.1, 0.2, 0.2], padded(0.2).kind().with_options(2_u8));
    assert!(matches!(
        refused,
        Err(ProjectionError::Options {
            projection: "padded",
            given: "u8",
            ..
        })
    ));
    let sheared = register(Sheared::default).unwrap();
    assert!(matches!(
        fig.subplot_with_projection(sheared.with_options(Pad(0.1))),
        Err(ProjectionError::Options { expected: "()", .. })
    ));
    assert_eq!(fig.axes().len(), 2, "refused options add no axes");
}

#[test]
fn grids_take_registered_projections() {
    let fig = Figure::new();
    let axs = fig
        .subplots_with_projection(2, 2, Share::All, Share::All, padded(0.1))
        .unwrap();
    assert!(axs.as_slice().iter().all(|ax| pad_of(ax) == Some(0.1)));
    axs[(0, 0)].set_xlim(0.0, 4.0).unwrap();
    assert_eq!(axs[(1, 1)].xlim(), (0.0, 4.0));
    let labels = |ax: &Axes| ax.xaxis().ticklabels(false).unwrap().len();
    assert_eq!(labels(&axs[(0, 1)]), 0, "inner labels of rectangles go");
    assert!(labels(&axs[(1, 1)]) > 0);

    let polar = Figure::new()
        .subplots_with_projection(2, 1, Share::All, Share::None, ProjectionKind::Polar)
        .unwrap();
    assert!(labels(&polar[(0, 0)]) > 0, "curved outlines keep theirs");

    let disc = register(|| Disc).unwrap();
    let discs = Figure::new();
    assert_eq!(
        discs.subplots_with_projection(2, 1, Share::All, Share::None, disc),
        Err(ProjectionError::Share(PlotError::FixedLimits(AxisName::X))),
        "fixed limits cannot be shared"
    );
    assert_eq!(
        discs.axes().len(),
        1,
        "the axes failing to share is removed"
    );

    let nowhere = ProjectionKind::Registered("nowhere");
    assert_eq!(
        fig.subplots_with_projection(1, 2, Share::None, Share::None, nowhere),
        Err(ProjectionError::Unknown("nowhere".into()))
    );
    assert_eq!(fig.axes().len(), 4, "unknown projections add no axes");
}

#[test]
fn mosaics_take_registered_projections_per_label() {
    let disc = register(|| Disc).unwrap();
    let options = MosaicOptions {
        projection: padded(0.1),
        per_subplot: vec![("B".into(), disc.into()), ("C".into(), padded(0.3))],
        ..MosaicOptions::default()
    };
    let axes = Figure::new()
        .subplot_mosaic("AB;CC".parse::<Mosaic>().unwrap(), options)
        .unwrap();
    let ax = |label| axes.get(label).unwrap();
    assert_eq!((pad_of(ax("A")), pad_of(ax("C"))), (Some(0.1), Some(0.3)));
    assert_eq!(ax("B").projection(), disc);
    let wrong = MosaicOptions {
        per_subplot: vec![("A".into(), disc.with_options(Pad(0.1)))],
        ..MosaicOptions::default()
    };
    let fig = Figure::new();
    assert!(matches!(
        fig.subplot_mosaic("AB".parse::<Mosaic>().unwrap(), wrong),
        Err(MosaicError::Projection(ProjectionError::Options { .. }))
    ));
    assert_eq!(fig.axes(), []);
}

#[test]
fn insets_and_twins_take_registered_projections() {
    let fig = Figure::new();
    let host = fig.subplot_with_projection(padded(0.1)).unwrap();
    let inset = host
        .inset_axes_with_projection([0.6, 0.6, 0.3, 0.3], padded(0.2))
        .unwrap();
    assert_eq!((pad_of(&inset), inset.zorder()), (Some(0.2), 5.0));
    assert_eq!(host.child_axes(), [inset]);

    let twin = host
        .twin_with_projection(AxisName::X, 1.0, padded(0.3))
        .unwrap();
    assert_eq!(pad_of(&twin), Some(0.3));
    assert_eq!(twin.zorder(), host.zorder() + 1.0);
    host.set_xlim(2.0, 3.0).unwrap();
    assert_eq!(twin.xlim(), (2.0, 3.0));
    assert!(!twin.patch_visible() && host.patch_visible());

    let disc = register(|| Disc).unwrap();
    for (from, kind) in [
        (&host, disc),
        (
            &fig.subplot_with_projection(disc).unwrap(),
            padded(0.1).kind(),
        ),
    ] {
        assert!(matches!(
            from.twin_with_projection(AxisName::Y, 0.0, kind),
            Err(ProjectionError::Aspect(_))
        ));
    }
    assert_eq!(fig.axes().len(), 3, "refused twins are not made");
}

#[test]
fn pyplot_reuses_axes_made_with_the_same_options() {
    pyplot::close_all();
    let ax = pyplot::subplot_with_projection(1, 1, 1, padded(0.1)).unwrap();
    assert_eq!(
        pyplot::subplot_with_projection(1, 1, 1, padded(0.1)).unwrap(),
        ax
    );
    let other = pyplot::subplot_with_projection(1, 1, 1, padded(0.2)).unwrap();
    assert_ne!(other, ax);
    let plain = pyplot::subplot_with_projection(1, 1, 1, padded(0.1).kind()).unwrap();
    assert!(
        plain != ax && plain != other,
        "no options are other options"
    );
    let (fig, axs) =
        pyplot::subplots_with_projection(1, 2, Share::None, Share::All, padded(0.3)).unwrap();
    assert_eq!((pyplot::gcf(), pad_of(&axs[(0, 1)])), (fig, Some(0.3)));
    pyplot::close_all();
}
