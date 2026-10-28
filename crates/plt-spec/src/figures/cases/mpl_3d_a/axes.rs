//! Axes, ticks, labels, aspects, views and texts.

use plt::artist::ArtistHandle;
use plt::axes::Adjustable;
use plt::figure::Figure;
use plt::layout::{LayoutEngine, TightLayout};
use plt::mplot3d::{Aspect3D, Axes3D, Axes3DError, Axis3, AxisPosition, ProjType, Vec3};

use super::{remove_ticks_and_titles, subplots_3d};
use crate::figures::CaseResult;

/// The test module's `plot_cuboid`: the twelve edges of a cuboid with
/// side lengths `scale`, one line each.
fn plot_cuboid(ax: &Axes3D, scale: Vec3) -> Result<(), Axes3DError> {
    // itertools.product([0, 1], repeat=3)
    let corners: Vec<[u8; 3]> = (0..8u8)
        .map(|i| [(i >> 2) & 1, (i >> 1) & 1, i & 1])
        .collect();
    for (i, start) in corners.iter().enumerate() {
        for end in &corners[i + 1..] {
            let distance: u8 = start.iter().zip(end).map(|(a, b)| a.abs_diff(*b)).sum();
            if distance == 1 {
                let [x, y, z] = [0, 1, 2]
                    .map(|k| [f64::from(start[k]) * scale[k], f64::from(end[k]) * scale[k]]);
                ax.plot(x, y, z)?;
            }
        }
    }
    Ok(())
}

/// `ax.set(xlabel="x", ylabel="y", zlabel="z", title=title)`.
fn label_xyz(ax: &Axes3D, title: &str) {
    ax.set_xlabel("x").set_ylabel("y").set_zlabel("z");
    ax.set_title(title);
}

pub(super) fn invisible_axes(fig: &Figure) {
    fig.subplot_3d().set_visible(false);
}

pub(super) fn grid_off(fig: &Figure) {
    fig.subplot_3d().grid(false);
    remove_ticks_and_titles(fig);
}

pub(super) fn invisible_ticks_axis(fig: &Figure) {
    let ax = fig.subplot_3d();
    for axis in [ax.xaxis(), ax.yaxis(), ax.zaxis()] {
        axis.set_ticks(Vec::<f64>::new()).set_line_visible(false);
    }
    remove_ticks_and_titles(fig);
}

pub(super) fn axis_positions(fig: &Figure) -> CaseResult {
    let positions = [
        (AxisPosition::Upper, "upper"),
        (AxisPosition::Lower, "lower"),
        (AxisPosition::Both, "both"),
        (AxisPosition::None, "none"),
    ];
    for (ax, (position, title)) in subplots_3d(fig, 2, 2)?.iter().zip(positions) {
        for axis in [ax.xaxis(), ax.yaxis(), ax.zaxis()] {
            axis.set_label_position(position)
                .set_ticks_position(position);
        }
        label_xyz(ax, title);
    }
    Ok(())
}

pub(super) fn axis_positions_inverted(fig: &Figure) -> CaseResult {
    for (i, ax) in subplots_3d(fig, 2, 4)?.iter().enumerate() {
        // itertools.product([False, True], repeat=3)
        let inverted = [i & 4 != 0, i & 2 != 0, i & 1 != 0];
        ax.plot([0.0, 1.0, 1.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0])?;
        let mut names = Vec::new();
        for ((axis, inverted), name) in Axis3::ALL.into_iter().zip(inverted).zip(["x", "y", "z"]) {
            if inverted {
                ax.invert(axis)?;
            }
            names.push(format!("{}{name}", if inverted { "-" } else { "" }));
        }
        label_xyz(ax, &names.join(", "));
    }
    Ok(())
}

pub(super) fn aspects(fig: &Figure) -> CaseResult {
    let aspects = [
        (Aspect3D::Auto, "auto"),
        (Aspect3D::Equal, "equal"),
        (Aspect3D::EqualXY, "equalxy"),
        (Aspect3D::EqualYZ, "equalyz"),
        (Aspect3D::EqualXZ, "equalxz"),
        (Aspect3D::Equal, "equal (cube)"),
    ];
    let axs = subplots_3d(fig, 2, 3)?;
    for (i, ax) in axs.iter().enumerate() {
        let scale = if i < 5 { [1.0, 1.0, 5.0] } else { [1.0; 3] };
        plot_cuboid(ax, scale)?;
    }
    for (ax, (aspect, title)) in axs.iter().zip(aspects) {
        ax.set_title(title);
        ax.set_box_aspect(Some([3.0, 4.0, 5.0]), 1.0)?;
        ax.set_aspect(aspect, Adjustable::Datalim)?;
    }
    Ok(())
}

pub(super) fn aspects_adjust_box(fig: &Figure) -> CaseResult {
    let aspects = [
        (Aspect3D::Auto, "auto"),
        (Aspect3D::Equal, "equal"),
        (Aspect3D::EqualXY, "equalxy"),
        (Aspect3D::EqualYZ, "equalyz"),
        (Aspect3D::EqualXZ, "equalxz"),
    ];
    for (ax, (aspect, title)) in subplots_3d(fig, 1, 5)?.iter().zip(aspects) {
        plot_cuboid(ax, [4.0, 3.0, 5.0])?;
        ax.set_title(title);
        ax.set_aspect(aspect, Adjustable::Box)?;
    }
    Ok(())
}

pub(super) fn primary_views(fig: &Figure) -> CaseResult {
    let views = [
        (90.0, -90.0),
        (0.0, -90.0),
        (0.0, 0.0),
        (-90.0, 90.0),
        (0.0, 90.0),
        (0.0, 180.0),
    ];
    fig.set_layout_engine(Some(LayoutEngine::Tight(TightLayout::default())))?;
    for (ax, (elev, azim)) in subplots_3d(fig, 2, 3)?.iter().zip(views) {
        ax.set_xlabel("x").set_ylabel("y").set_zlabel("z");
        ax.set_proj_type(ProjType::Ortho)?;
        ax.view_init(elev, azim, 0.0);
    }
    remove_ticks_and_titles(fig);

    Ok(())
}

/// The texts of the `text3d` tests: anchor, direction (`zdir`) and how
/// the test prints the direction.
const TEXTS: [(Vec3, Option<Vec3>, &str); 6] = [
    ([2.0, 6.0, 4.0], None, "None"),
    ([6.0, 4.0, 2.0], Some([1.0, 0.0, 0.0]), "x"),
    ([4.0, 8.0, 5.0], Some([0.0, 1.0, 0.0]), "y"),
    ([9.0, 7.0, 6.0], Some([0.0, 0.0, 1.0]), "z"),
    ([7.0, 2.0, 1.0], Some([1.0, 1.0, 0.0]), "(1, 1, 0)"),
    ([2.0, 2.0, 7.0], Some([1.0, 1.0, 1.0]), "(1, 1, 1)"),
];

/// `"(x, y, z), dir=zdir"`.
fn text_label([x, y, z]: Vec3, zdir: &str) -> String {
    format!("({x}, {y}, {z}), dir={zdir}")
}

/// `set_xlim3d(0, 10)` and the same for y and z.
fn limit_to_ten(ax: &Axes3D) -> Result<(), Axes3DError> {
    ax.set_xlim(0.0, 10.0)?
        .set_ylim(0.0, 10.0)?
        .set_zlim(0.0, 10.0)?;
    Ok(())
}

pub(super) fn text3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    for (position, direction, zdir) in TEXTS {
        let [x, y, z] = position;
        ax.text_along(
            x,
            y,
            z,
            text_label(position, zdir),
            direction.unwrap_or_default(),
        );
    }
    ax.text(1.0, 1.0, 1.0, "red", None).set_color("red");
    ax.text2d(0.05, 0.95, "2D Text");
    limit_to_ten(&ax)?;
    ax.set_xlabel("X axis")
        .set_ylabel("Y axis")
        .set_zlabel("Z axis");
    Ok(())
}

pub(super) fn text3d_modification(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    limit_to_ten(&ax)?;
    for (position, direction, zdir) in TEXTS {
        let text = ax.text(0.0, 0.0, 0.0, text_label(position, zdir), None);
        ax.set_text_position(&text, position, direction)?;
    }
    Ok(())
}

pub(super) fn tight_layout_text(fig: &Figure) -> CaseResult {
    fig.subplot_3d().text(0.5, 0.5, 0.5, "some string", None);
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}
