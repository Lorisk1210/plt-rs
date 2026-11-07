//! Layout of offset-box trees against values worked out from
//! matplotlib's packing rules.

use pltrs_core::{Affine2D, Bbox, Point};

use super::*;
use crate::artist::{ArtistHandle, LayoutContext};
use crate::figure::Figure;

type Tree = OffsetBox<ArtistId>;

/// Distinct artist ids to tag the leaves with.
fn tags(fig: &Figure) -> (ArtistId, ArtistId) {
    (fig.text(0.0, 0.0, "1").id(), fig.text(0.0, 0.0, "2").id())
}

fn area(width: f64, height: f64, ydescent: f64, tag: ArtistId) -> Tree {
    DrawingArea::new(width, height, 0.0, ydescent)
        .with_artist(tag)
        .into()
}

fn two_areas(fig: &Figure) -> Vec<Tree> {
    let (one, two) = tags(fig);
    vec![area(10.0, 5.0, 1.0, one), area(20.0, 8.0, 2.0, two)]
}

/// Runs `f` with a layout context of `fig`.
fn with_layout<R>(fig: &Figure, f: impl FnOnce(LayoutContext<'_>) -> R) -> R {
    let state = fig.raw().figure_state().borrow();
    f(LayoutContext::new(&state))
}

/// The translation of an area placement.
fn area_origin(placement: &Placement<ArtistId>) -> Point {
    match placement {
        Placement::Artists {
            transform: Transform::Affine(affine),
            ..
        } => affine.translation_part(),
        other => panic!("not an area placement: {other:?}"),
    }
}

#[test]
fn horizontal_packing_aligns_baselines() {
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let (one, two) = tags(&fig);
    let children = vec![area(10.0, 5.0, 1.0, one), area(20.0, 8.0, 2.0, two)];
    let packer = OffsetBox::HPacker(Packer::new(1.0, 2.0, PackAlign::Baseline, children));
    with_layout(&fig, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_extents(-1.0, -3.0, 33.0, 7.0))
        );
        let placed = packer.place(ctx, Point::new(100.0, 50.0)).unwrap();
        assert_eq!(
            placed,
            vec![
                Placement::Artists {
                    artists: vec![one],
                    transform: Transform::Affine(Affine2D::translation(100.0, 50.0)),
                    clip: None,
                },
                Placement::Artists {
                    artists: vec![two],
                    transform: Transform::Affine(Affine2D::translation(112.0, 50.0)),
                    clip: None,
                },
            ]
        );
    });
}

#[test]
fn vertical_packing_stacks_from_the_top() {
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let packer = OffsetBox::VPacker(Packer::new(0.0, 2.0, PackAlign::Center, two_areas(&fig)));
    with_layout(&fig, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_bounds(0.0, -11.0, 20.0, 15.0))
        );
        let origins: Vec<Point> = packer
            .place(ctx, Point::ORIGIN)
            .unwrap()
            .iter()
            .map(area_origin)
            .collect();
        assert_eq!(origins, [Point::new(5.0, 0.0), Point::new(0.0, -9.0)]);
    });
}

#[test]
fn a_vertical_width_expands_horizontal_children() {
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let mut row = Packer::new(0.0, 0.0, PackAlign::Baseline, two_areas(&fig));
    row.mode = PackMode::Expand;
    let mut column = OffsetBox::VPacker(Packer::new(
        0.0,
        0.0,
        PackAlign::Left,
        vec![OffsetBox::HPacker(row)],
    ));
    column.set_width(Some(50.0));
    with_layout(&fig, |ctx| {
        assert_eq!(column.bbox(ctx).unwrap().width(), 50.0);
        let placed = column.place(ctx, Point::ORIGIN).unwrap();
        assert_eq!(area_origin(&placed[1]).x, 30.0);
    });
}

#[test]
fn areas_scale_with_the_resolution_and_hidden_boxes_are_skipped() {
    let fig = Figure::with_size(4.0, 3.0, 144.0);
    let (one, two) = tags(&fig);
    let mut children = vec![area(10.0, 5.0, 1.0, one), area(20.0, 8.0, 2.0, two)];
    children[0].set_visible(false);
    let packer = OffsetBox::HPacker(Packer::new(1.0, 2.0, PackAlign::Baseline, children));
    assert_eq!(packer.artists(), vec![&two]);
    with_layout(&fig, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_extents(-2.0, -6.0, 42.0, 14.0))
        );
        let placed = packer.place(ctx, Point::ORIGIN).unwrap();
        assert_eq!(
            placed,
            vec![Placement::Artists {
                artists: vec![two],
                transform: Transform::Affine(Affine2D::scaling(2.0, 2.0)),
                clip: None,
            }]
        );
    });
}

#[test]
fn empty_packers_are_just_their_padding() {
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let packer: Tree = OffsetBox::VPacker(Packer::new(3.0, 1.0, PackAlign::Center, Vec::new()));
    with_layout(&fig, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_extents(-3.0, -3.0, 3.0, 3.0))
        );
    });
}

#[test]
fn clipped_areas_clip_their_artists_to_the_area() {
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let (one, _) = tags(&fig);
    let mut clipped = DrawingArea::new(4.0, 3.0, 0.0, 0.0).with_artist(one);
    clipped.clip = true;
    let tree: Tree = clipped.into();
    with_layout(&fig, |ctx| {
        let placed = tree.place(ctx, Point::ORIGIN).unwrap();
        let Placement::Artists {
            clip: Some(outline),
            ..
        } = &placed[0]
        else {
            panic!("a clipped area clips");
        };
        assert_eq!(outline.extents(), Bbox::from_extents(0.0, 0.0, 4.0, 3.0));
    });
}

#[test]
fn padded_boxes_frame_their_padded_child() {
    let fig = Figure::with_size(4.0, 3.0, 144.0);
    let (one, frame) = tags(&fig);
    let tree: Tree = PaddedBox {
        pad: 1.0,
        child: Box::new(area(10.0, 5.0, 0.0, one)),
        frame,
        visible: true,
    }
    .into();
    assert_eq!(tree.artists(), vec![&frame, &one], "the frame comes first");
    with_layout(&fig, |ctx| {
        assert_eq!(
            tree.bbox(ctx),
            Ok(Bbox::from_extents(-2.0, -2.0, 22.0, 12.0))
        );
        let placed = tree.place(ctx, Point::new(5.0, 5.0)).unwrap();
        assert_eq!(
            placed[0],
            Placement::Frame {
                frame,
                bbox: Bbox::from_extents(3.0, 3.0, 27.0, 17.0),
                mutation_scale: None,
            }
        );
        assert_eq!(area_origin(&placed[1]), Point::new(5.0, 5.0));
    });
}

#[test]
fn aux_boxes_are_as_large_as_their_measured_artists() {
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let (one, _) = tags(&fig);
    let mut aux = AuxTransformBox::new(Transform::Display).with_artist(one);
    aux.set_extent(Bbox::from_extents(10.0, 20.0, 40.0, 30.0), true);
    let unmeasurable: Tree = aux.clone().into();
    aux.set_extent(Bbox::from_extents(10.0, 20.0, 40.0, 30.0), false);
    let tree: Tree = aux.into();
    with_layout(&fig, |ctx| {
        assert_eq!(unmeasurable.bbox(ctx), Err(OffsetBoxError::NoFigure));
        assert_eq!(tree.bbox(ctx), Ok(Bbox::from_extents(0.0, 0.0, 30.0, 10.0)));
        let placed = tree.place(ctx, Point::new(1.0, 2.0)).unwrap();
        let Placement::Artists { transform, .. } = &placed[0] else {
            panic!("an aux box places its artists");
        };
        let resolved = ctx.resolve(transform).unwrap();
        assert_eq!(
            resolved.transform_point(Point::new(10.0, 20.0)),
            Point::new(1.0, 2.0),
            "the extent's corner lands on the offset"
        );
    });
}

#[test]
fn empty_aux_boxes_cannot_be_measured() {
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let tree: Tree = AuxTransformBox::new(Transform::Display).into();
    with_layout(&fig, |ctx| {
        assert_eq!(tree.bbox(ctx), Err(OffsetBoxError::EmptyAuxBox));
    });
    let ax = fig.subplot();
    let anchored = ax.add_anchored_box(AnchoredBox::new(
        AuxTransformBox::new(ax.trans_data()),
        AnchoredOptions::new(AnchorLoc::Center),
    ));
    anchored.raw().with_base_mut(|base| base.animated = true);
    assert!(
        fig.draw_without_rendering().is_ok(),
        "animated before the first draw, as its layout when added failed"
    );
    anchored.raw().with_base_mut(|base| base.animated = false);
    let errors = fig.draw_without_rendering().expect_err("nothing to unite");
    assert_eq!(errors.0[0].kind, crate::artist::DrawErrorKind::InvalidValue);
    assert_eq!(errors.0[0].message, "'bboxes' cannot be empty");
    anchored.raw().with_base_mut(|base| base.animated = true);
    assert!(
        fig.draw_without_rendering().is_ok(),
        "a draw leaving the box out does not lay it out"
    );
    anchored.raw().with_base_mut(|base| {
        base.animated = false;
        base.visible = false;
    });
    assert!(
        fig.draw_without_rendering().is_ok(),
        "nor one of a hidden box"
    );
}

#[test]
fn text_areas_reach_at_least_the_height_of_lp() {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let single = fig.text(0.0, 0.0, "a").id();
    let double = fig.text(0.0, 0.0, "a\nb").id();
    let state = fig.raw().figure_state().borrow();
    let ctx = LayoutContext::new(&state);
    let lp = |text: ArtistId| Tree::TextArea(TextArea::new(text, true));
    let reference = Tree::TextArea(TextArea::new(single, false))
        .bbox(ctx)
        .unwrap();
    assert!(reference.y0() < 0.0 && reference.y1() > 0.0);
    let centred = lp(double).bbox(ctx).unwrap();
    assert!(centred.height() > reference.height());
    let Placement::Text { baseline_shift, .. } = lp(double).place(ctx, Point::ORIGIN).unwrap()[0]
    else {
        panic!("a text area places a text");
    };
    assert!(
        baseline_shift < 0.0,
        "multi-line text is lowered to centre it on the baseline"
    );
    assert_eq!(
        lp(fig.id()).bbox(ctx),
        Err(OffsetBoxError::MissingText(fig.id()))
    );
}

#[test]
fn trees_map_their_artists_in_drawing_order() {
    let tree: OffsetBox<u8> = OffsetBox::VPacker(Packer::new(
        0.0,
        0.0,
        PackAlign::Center,
        vec![
            TextArea::new(1, false).into(),
            DrawingArea::new(1.0, 1.0, 0.0, 0.0)
                .with_artist(2)
                .with_artist(3)
                .into(),
        ],
    ));
    let mut seen = Vec::new();
    let mapped = tree.map(&mut |tag| {
        seen.push(tag);
        u16::from(tag) * 10
    });
    assert_eq!(seen, [1, 2, 3]);
    assert_eq!(mapped.artists(), vec![&10, &20, &30]);
}

#[test]
fn aux_boxes_measure_collections_by_their_shapes() {
    use crate::collections::{Collection, CollectionSpec};
    let fig = Figure::with_size(4.0, 3.0, 72.0);
    let lines = Collection::loose(CollectionSpec::lines(vec![
        vec![Point::new(0.0, 10.0), Point::new(5.0, 10.0)],
        vec![Point::new(18.0, 10.0), Point::new(23.0, 10.0)],
    ]));
    let aux = AuxTransformBox::new(Transform::Display)
        .with_artist(BoxItem::from_loose(lines.raw()).unwrap());
    let options = AnchoredOptions {
        pad: 0.0,
        ..AnchoredOptions::new(AnchorLoc::Center)
    };
    let anchored = fig.add_anchored_box(AnchoredBox::new(aux, options));
    fig.draw_without_rendering().unwrap();
    // Placed by the box, the lines have no extent of their own, as
    // matplotlib's collections placed by a translation have none.
    let lines = anchored.artists()[0].window_extent().unwrap();
    assert!(lines.is_null());
    let anchored = anchored.raw().window_extent().unwrap();
    assert!(anchored.is_finite());
    assert_eq!(anchored.width(), 23.0);
}
