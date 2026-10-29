//! `test_bbox_tight.py`: figures saved with `bbox_inches="tight"`, whose
//! tight box must take in legends, tables, super titles, long tick
//! labels, clipped artists, rasterized lines, fixed aspects and insets.

use pltrs::artist::ClipPathSpec;
use pltrs::axes::ImshowOptions;
use pltrs::axes_grid1::{InsetOptions, SizeSpec, inset_axes};
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::legend::LegendOptions;
use pltrs::numeric::Grid;
use pltrs::offsetbox::AnchorLoc;
use pltrs::orientation::Orientation;
use pltrs::patches::Rectangle;
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::table::{TableLoc, TableOptions};
use pltrs::ticker::FuncFormatter;
use pltrs::transforms::Transform;
use pltrs_core::{Affine2D, Path};

use super::{TIGHT_BBOX, classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_figure/bbox_inches_tight", bbox_inches_tight).with_rc(&[TIGHT_BBOX]),
    FigureCase::new(
        "mpl_figure/bbox_inches_tight_suptile_legend",
        suptile_legend,
    )
    .with_rc(&[TIGHT_BBOX]),
    FigureCase::new("mpl_figure/bbox_inches_tight_suptile_non_default", |fig| {
        fig.subplot();
        fig.suptitle("Booo").set_x(0.5).set_y(1.1);
        Ok(())
    })
    .with_rc(&[TIGHT_BBOX]),
    classic("mpl_figure/bbox_inches_tight_clipping", clipping)
        .with_rc_setup(|rc| Ok(rc.set("savefig.bbox", "tight")?)),
    FigureCase::new("mpl_figure/bbox_inches_tight_raster", |fig| {
        fig.subplot().plot_y([1.0, 2.0]).set_rasterized(true);
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(&[TIGHT_BBOX]),
    FigureCase::new("mpl_figure/bbox_inches_fixed_aspect", |fig| {
        let ax = fig.subplot();
        ax.plot_y([0.0, 1.0]);
        ax.set_xlim(0.0, 1.0)?;
        ax.set_aspect(Aspect::Equal)?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(&[TIGHT_BBOX, ("figure.constrained_layout.use", "True")]),
    FigureCase::new("mpl_figure/bbox_inches_inset_rasterized", |fig| {
        let ax = fig.subplot();
        let arr = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64);
        let image = ax.imshow(arr, ImshowOptions::default())?;
        let options = InsetOptions {
            loc: AnchorLoc::UpperLeft,
            bbox_to_anchor: Some((0.045, 0.0, 1.0, 1.0).into()),
            bbox_transform: Some(Transform::Axes(ax.id())),
            ..InsetOptions::default()
        };
        let inset = inset_axes(
            &ax,
            SizeSpec::Percent(10.0),
            SizeSpec::Percent(30.0),
            options,
        );
        fig.colorbar(
            &image,
            ColorbarOptions {
                axes: ColorbarAxes::Cax(inset),
                orientation: Some(Orientation::Horizontal),
                ..ColorbarOptions::default()
            },
        )?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(&[TIGHT_BBOX]),
];

fn bbox_inches_tight(fig: &Figure) -> CaseResult {
    const DATA: [[f64; 5]; 5] = [
        [66_386.0, 174_296.0, 75_131.0, 577_908.0, 32_015.0],
        [58_230.0, 381_139.0, 78_045.0, 99_308.0, 160_454.0],
        [89_135.0, 80_552.0, 152_558.0, 497_981.0, 603_535.0],
        [78_415.0, 81_858.0, 150_656.0, 193_263.0, 69_638.0],
        [139_361.0, 331_509.0, 343_164.0, 781_380.0, 52_269.0],
    ];
    let col_labels = ["Freeze", "Wind", "Flood", "Quake", "Hail"];
    let row_labels = [100, 50, 20, 10, 5].map(|x| format!("{x} year"));

    let ind: Vec<f64> = (0..col_labels.len()).map(|i| i as f64 + 0.3).collect();
    let mut cell_text = Vec::new();
    let mut yoff = [0.0; 5];
    let ax = fig.subplot();
    for row in DATA {
        let options = BarOptions {
            thickness: 0.4.into(),
            base: yoff.to_vec().into(),
            align: BarAlign::Edge,
            ..BarOptions::default()
        };
        ax.bar(ind.clone(), row, options)?;
        for (offset, value) in yoff.iter_mut().zip(row) {
            *offset += value;
        }
        cell_text.push(yoff.map(|x| format!("{:.1}", x / 1000.0)).to_vec());
    }
    ax.set_xticks(Vec::<f64>::new());
    ax.set_xlim(0.0, 5.0)?;
    ax.legend(
        LegendOptions::default()
            .labels(["1", "2", "3", "4", "5"])
            .loc((1.2, 0.2)),
    )?;
    fig.legend(
        LegendOptions::default()
            .labels(["a", "b", "c", "d", "e"])
            .bbox_to_anchor((0.0, 0.2))
            .loc(AnchorLoc::LowerLeft),
    )?;
    cell_text.reverse();
    ax.table(TableOptions {
        row_labels: Some(row_labels.to_vec()),
        col_labels: Some(col_labels.map(String::from).to_vec()),
        loc: TableLoc::Bottom,
        ..TableOptions::with_text(cell_text)
    })?;
    remove_ticks_and_titles(fig);
    Ok(())
}

fn suptile_legend(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(
        arange(0.0, 10.0, 1.0),
        arange(0.0, 10.0, 1.0),
        LineProps::default().label("a straight line"),
    )?;
    ax.legend(
        LegendOptions::default()
            .bbox_to_anchor((0.9, 1.0))
            .loc(AnchorLoc::UpperLeft),
    )?;
    ax.set_title("Axis title");
    fig.suptitle("Figure title");

    // An extra long y tick label, to see that the box accounts for it;
    // the others as Python's `str(float)`.
    ax.yaxis().set_major_formatter(FuncFormatter::new(|y, _| {
        if (4.0..5.0).contains(&y) {
            "The number 4".to_owned()
        } else {
            format!("{y:?}")
        }
    }));

    ax.set_xlabel("X axis");
    Ok(())
}

fn clipping(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.scatter(arange(0.0, 10.0, 1.0), arange(0.0, 10.0, 1.0))?;
    ax.set_xlim(0.0, 5.0)?;
    ax.set_ylim(0.0, 5.0)?;

    // A massive rectangle clipped by a small star.
    let patch = fig.add_patch(Rectangle::new((-50.0, -50.0), 100.0, 100.0));
    patch.set_facecolor("blue").set_alpha(Some(0.5));
    patch.set_transform(Transform::Data(ax.id()));
    let star = Path::unit_regular_star(5, 0.5).transformed(&Affine2D::IDENTITY.scale(0.25, 0.25));
    patch.set_clip_path(Some(ClipPathSpec::Path {
        path: star,
        transform: Transform::Axes(ax.id()),
    }));
    remove_ticks_and_titles(fig);
    Ok(())
}
