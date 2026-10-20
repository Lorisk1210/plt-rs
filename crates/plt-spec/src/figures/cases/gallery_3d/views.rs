//! Wireframes, projections and views: `wire3d`, `wire3d_zero_stride`,
//! `axlim_clip`, `projections` and `view_planes_3d`.

use std::collections::HashMap;

use plt::collections::CollectionProps;
use plt::figure::Figure;
use plt::gridspec::GridSpec;
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::mplot3d::{
    Axes3D, Axes3DError, Axes3DOptions, Axis3, ProjType, Sampling, WireframeOptions, test_data,
};
use plt::numeric::{Grid, arange, meshgrid};
use plt::text::{HorizontalAlignment, VerticalAlignment};

use super::label_xyz;
use crate::figures::cases::common::subplots_3d;
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/wire3d", wire3d),
    FigureCase::new("gallery_3d/wire3d_zero_stride", wire3d_zero_stride).sized(8.0, 12.0, 100.0),
    FigureCase::new("gallery_3d/axlim_clip", axlim_clip),
    FigureCase::new("gallery_3d/projections", projections),
    FigureCase::new("gallery_3d/view_planes_3d", view_planes_3d).sized(12.0, 8.5, 100.0),
];

/// `ax.plot_wireframe(*get_test_data(0.05), rstride=rows, cstride=cols)`.
fn test_wireframe(ax: &Axes3D, rows: usize, cols: usize) -> Result<(), Axes3DError> {
    let (x, y, z) = test_data(0.05);
    let options = WireframeOptions {
        sampling: Sampling::Strides { rows, cols },
        ..WireframeOptions::default()
    };
    ax.plot_wireframe(&x, &y, &z, options).map(drop)
}

fn wire3d(fig: &Figure) -> CaseResult {
    test_wireframe(&fig.subplot_3d(), 10, 10)?;
    Ok(())
}

fn wire3d_zero_stride(fig: &Figure) -> CaseResult {
    let axs = subplots_3d(fig, 2, 1)?;
    test_wireframe(&axs[0], 10, 0)?;
    axs[0].set_title("Column (x) stride set to 0");
    test_wireframe(&axs[1], 0, 10)?;
    axs[1].set_title("Row (y) stride set to 0");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn axlim_clip(fig: &Figure) -> CaseResult {
    let ax = &subplots_3d(fig, 1, 1)?[0];
    let axis = arange(-5.0, 5.0, 0.5);
    let (x, y) = meshgrid(&axis, &axis);
    let z = Grid::from_fn(x.rows(), x.cols(), |i, j| {
        (x[(i, j)].powi(2) + y[(i, j)].powi(2)).sqrt().sin()
    });
    for (color, axlim_clip) in [("C0", false), ("C1", true)] {
        let options = WireframeOptions {
            props: CollectionProps::default().color(color),
            axlim_clip,
            ..WireframeOptions::default()
        };
        ax.plot_wireframe(&x, &y, &z, options)?;
    }
    ax.set_xlim(0.0, 10.0)?
        .set_ylim(-5.0, 5.0)?
        .set_zlim(-1.0, 0.5)?;
    ax.legend(LegendOptions::default().labels(["axlim_clip=False (default)", "axlim_clip=True"]))?;
    Ok(())
}

fn projections(fig: &Figure) -> CaseResult {
    let axs = subplots_3d(fig, 1, 3)?;
    for ax in &axs {
        test_wireframe(ax, 10, 10)?;
    }
    let views = [
        (ProjType::Ortho, "'ortho'\nfocal_length = ∞"),
        (
            ProjType::Persp { focal_length: 1.0 },
            "'persp'\nfocal_length = 1 (default)",
        ),
        (
            ProjType::Persp { focal_length: 0.2 },
            "'persp'\nfocal_length = 0.2",
        ),
    ];
    for (ax, (proj_type, title)) in axs.iter().zip(views) {
        ax.set_proj_type(proj_type)?;
        ax.set_title(title).set_fontsize(10.0);
    }
    Ok(())
}

/// `view_planes_3d`'s label in the middle of the box.
fn annotate_axes(ax: &Axes3D, text: &str, fontsize: f64) {
    ax.text(0.5, 0.5, 0.5, text, None)
        .set_valign(VerticalAlignment::Center)
        .set_halign(HorizontalAlignment::Center)
        .set_fontsize(fontsize)
        .set_color("black");
}

/// One of the primary view planes of `view_planes_3d`.
struct View {
    /// Its name, which labels it.
    plane: &'static str,
    /// Its cell of the mosaic.
    cell: (usize, usize),
    /// `(elev, azim, roll)`.
    angles: [f64; 3],
    /// The axis seen end-on, whose tick labels and label are taken off.
    hidden: Axis3,
}

const VIEWS: [View; 6] = [
    View {
        plane: "XY",
        cell: (0, 0),
        angles: [90.0, -90.0, 0.0],
        hidden: Axis3::Z,
    },
    View {
        plane: "XZ",
        cell: (1, 0),
        angles: [0.0, -90.0, 0.0],
        hidden: Axis3::Y,
    },
    View {
        plane: "YZ",
        cell: (1, 1),
        angles: [0.0, 0.0, 0.0],
        hidden: Axis3::X,
    },
    View {
        plane: "-XY",
        cell: (2, 2),
        angles: [-90.0, 90.0, 0.0],
        hidden: Axis3::Z,
    },
    View {
        plane: "-XZ",
        cell: (1, 2),
        angles: [0.0, 90.0, 0.0],
        hidden: Axis3::Y,
    },
    View {
        plane: "-YZ",
        cell: (1, 3),
        angles: [0.0, 180.0, 0.0],
        hidden: Axis3::X,
    },
];

/// The cell of the mosaic holding the title of `view_planes_3d`.
const TITLE_CELL: (usize, usize) = (0, 2);

fn view_planes_3d(fig: &Figure) -> CaseResult {
    // subplot_mosaic creates its axes in the order of their cells.
    let grid = GridSpec::new(3, 4)?;
    let mut cells: Vec<(usize, usize)> = VIEWS
        .iter()
        .map(|view| view.cell)
        .chain([TITLE_CELL])
        .collect();
    cells.sort_unstable();
    let mut axes = HashMap::new();
    for (row, col) in cells {
        let ax = fig.add_subplot_spec_3d(&grid.cell(row, col)?, Axes3DOptions::default())?;
        axes.insert((row, col), ax);
    }
    for view in &VIEWS {
        let ax = &axes[&view.cell];
        let [elev, azim, roll] = view.angles;
        label_xyz(ax, ["x", "y", "z"]);
        ax.set_proj_type(ProjType::Ortho)?;
        ax.view_init(elev, azim, roll);
        ax.set_box_aspect(None, 1.25)?;
        let label = format!("{}\n({elev}, {azim}, {roll})", view.plane);
        annotate_axes(ax, &label, 14.0);
        ax.axis(view.hidden).set_ticklabels(Vec::<String>::new())?;
        ax.set_label(view.hidden, "");
    }
    let title = &axes[&TITLE_CELL];
    annotate_axes(
        title,
        "mplot3d primary view planes\nax.view_init(elev, azim, roll)",
        18.0,
    );
    title.set_axis_off();
    Ok(())
}
