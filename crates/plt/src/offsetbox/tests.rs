//! Layout of offset-box trees against values worked out from
//! matplotlib's packing rules.

use plt_core::{Affine2D, Bbox, Point};

use super::*;
use crate::artist::{ArtistHandle, LayoutContext};
use crate::figure::Figure;

type Tree = OffsetBox<u8>;

fn area(width: f64, height: f64, ydescent: f64, tag: u8) -> Tree {
    let mut area = DrawingArea::new(width, height, 0.0, ydescent);
    area.artists.push(tag);
    OffsetBox::DrawingArea(area)
}

fn two_areas() -> Vec<Tree> {
    vec![area(10.0, 5.0, 1.0, 1), area(20.0, 8.0, 2.0, 2)]
}

/// Runs `f` with a layout context at `dpi`.
fn with_layout<R>(dpi: f64, f: impl FnOnce(LayoutContext<'_>) -> R) -> R {
    let fig = Figure::with_size(4.0, 3.0, dpi);
    let state = fig.raw().figure_state().borrow();
    f(LayoutContext::new(&state))
}

#[test]
fn horizontal_packing_aligns_baselines() {
    let packer = OffsetBox::HPacker(Packer::new(1.0, 2.0, PackAlign::Baseline, two_areas()));
    with_layout(72.0, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_extents(-1.0, -3.0, 33.0, 7.0))
        );
        let placed = packer.place(ctx, Point::new(100.0, 50.0)).unwrap();
        assert_eq!(
            placed,
            vec![
                Placement::Area {
                    artists: vec![1],
                    transform: Affine2D::translation(100.0, 50.0)
                },
                Placement::Area {
                    artists: vec![2],
                    transform: Affine2D::translation(112.0, 50.0)
                },
            ]
        );
    });
}

#[test]
fn vertical_packing_stacks_from_the_top() {
    let packer = OffsetBox::VPacker(Packer::new(0.0, 2.0, PackAlign::Center, two_areas()));
    with_layout(72.0, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_bounds(0.0, -11.0, 20.0, 15.0))
        );
        let origins: Vec<Point> = packer
            .place(ctx, Point::ORIGIN)
            .unwrap()
            .into_iter()
            .map(|p| match p {
                Placement::Area { transform, .. } => transform.translation_part(),
                Placement::Text { anchor, .. } => anchor,
            })
            .collect();
        assert_eq!(origins, [Point::new(5.0, 0.0), Point::new(0.0, -9.0)]);
    });
}

#[test]
fn a_vertical_width_expands_horizontal_children() {
    let mut row = Packer::new(0.0, 0.0, PackAlign::Baseline, two_areas());
    row.mode = PackMode::Expand;
    let mut column = OffsetBox::VPacker(Packer::new(
        0.0,
        0.0,
        PackAlign::Left,
        vec![OffsetBox::HPacker(row)],
    ));
    column.set_width(Some(50.0));
    with_layout(72.0, |ctx| {
        assert_eq!(column.bbox(ctx).unwrap().width(), 50.0);
        let placed = column.place(ctx, Point::ORIGIN).unwrap();
        let Placement::Area { transform, .. } = &placed[1] else {
            panic!("the second leaf is an area");
        };
        assert_eq!(transform.translation_part().x, 30.0);
    });
}

#[test]
fn areas_scale_with_the_resolution_and_hidden_boxes_are_skipped() {
    let mut children = two_areas();
    children[0].set_visible(false);
    let packer = OffsetBox::HPacker(Packer::new(1.0, 2.0, PackAlign::Baseline, children));
    assert_eq!(packer.leaves(), vec![Leaf::Area(&[2])]);
    with_layout(144.0, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_extents(-2.0, -6.0, 42.0, 14.0))
        );
        let placed = packer.place(ctx, Point::ORIGIN).unwrap();
        assert_eq!(
            placed,
            vec![Placement::Area {
                artists: vec![2],
                transform: Affine2D::scaling(2.0, 2.0)
            }]
        );
    });
}

#[test]
fn empty_packers_are_just_their_padding() {
    let packer: Tree = OffsetBox::VPacker(Packer::new(3.0, 1.0, PackAlign::Center, Vec::new()));
    with_layout(72.0, |ctx| {
        assert_eq!(
            packer.bbox(ctx),
            Ok(Bbox::from_extents(-3.0, -3.0, 3.0, 3.0))
        );
    });
}

#[test]
fn text_areas_reach_at_least_the_height_of_lp() {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let single = fig.text(0.0, 0.0, "a").id();
    let double = fig.text(0.0, 0.0, "a\nb").id();
    let state = fig.raw().figure_state().borrow();
    let ctx = LayoutContext::new(&state);
    let lp = |text: ArtistId| OffsetBox::<u8>::TextArea(TextArea::new(text, true));
    let reference = OffsetBox::<u8>::TextArea(TextArea::new(single, false))
        .bbox(ctx)
        .unwrap();
    assert!(reference.y0() < 0.0 && reference.y1() > 0.0);
    let centred = lp(double).bbox(ctx).unwrap();
    assert!(centred.height() > reference.height());
    let Placement::Text { anchor, .. } = lp(double).place(ctx, Point::ORIGIN).unwrap()[0] else {
        panic!("a text area places a text");
    };
    assert!(
        anchor.y < 0.0,
        "multi-line text is lowered to centre it on the baseline"
    );
    assert_eq!(
        lp(fig.id()).bbox(ctx),
        Err(OffsetBoxError::MissingText(fig.id()))
    );
}
