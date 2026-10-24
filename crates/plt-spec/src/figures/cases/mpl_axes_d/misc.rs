//! Single colours across plot kinds, bar labels on log axes, images with
//! date extents, preset clip paths, `ecdf`, orientation keywords, a
//! colorizer shared by every kind of mappable and axes positions.

use plt::artist::ClipPathSpec;
use plt::axes::{
    Axes, AxisName, BarLabelOptions, BarLabelType, BoxplotOptions, BxpOptions, EcdfOptions,
    HexGridSize, HexbinOptions, HistPlotOptions, ImshowOptions, Limits, Mesh, MeshCoords,
    PcolorOptions, PcolorfastOptions, ScatterColors, ScatterOptions, ViolinDrawOptions,
    ViolinplotOptions,
};
use plt::colors::{Colorize, ColorizerOptions, Colormap, Normalize, colormaps};
use plt::contour::{ContourOptions, QuadField};
use plt::dates::{DateFormatter, Datetime64};
use plt::figure::Figure;
use plt::image::{Extent, FigimageOptions, Origin};
use plt::lines::{LineStyle, StepWhere};
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::patches::{PatchProps, Polygon};
use plt::patheffects::{PathEffect, TickedStroke};
use plt::prelude::{ArtistHandle, BarOptions, LineProps, arange};
use plt::text::{AnnotateOptions, ArrowProps, LegacyArrowProps};
use plt::transforms::Point;
use plt::tri::Triangulation;

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `test_2dcolor_plot`: one RGB colour for a line, markers, steps, a
/// histogram and bars.
fn color_2d_plot(fig: &Figure) -> CaseResult {
    let color = [0.1, 0.2, 0.3];
    let axs = fig.subplots(5, 1)?.into_vec();
    axs[0].plot_with([1.0, 2.0], [1.0, 2.0], LineProps::default().color(color))?;
    let markers = ScatterOptions {
        colors: ScatterColors::Single(color.into()),
        ..ScatterOptions::default()
    };
    axs[1].scatter_with([1.0, 2.0], [1.0, 2.0], markers)?;
    axs[2].step(
        [1.0, 2.0],
        [1.0, 2.0],
        StepWhere::Pre,
        LineProps::default().color(color),
    )?;
    let hist = HistPlotOptions {
        color: vec![color.into()],
        ..HistPlotOptions::default()
    };
    axs[3].hist(&[arange(0.0, 10.0, 1.0)], hist)?;
    let bars = BarOptions {
        color: vec![color.into()],
        ..BarOptions::default()
    };
    axs[4].bar(arange(0.0, 10.0, 1.0), arange(0.0, 10.0, 1.0), bars)?;
    Ok(())
}

/// `2020-01-01` and `2020-01-11` as `datetime64[D]` values.
fn january_2020() -> [Datetime64; 2] {
    const DAY: i64 = 86_400;
    const NEW_YEAR_2020: i64 = 18_262 * DAY;
    [
        Datetime64::new(NEW_YEAR_2020, 0),
        Datetime64::new(NEW_YEAR_2020 + 10 * DAY, 0),
    ]
}

/// The first and last dates of the test as numbers of the axis `name` of
/// `ax`, which becomes a date axis (matplotlib's `_process_unit_info`).
fn january_2020_on(ax: &Axes, name: AxisName) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    match ax.axis_of(name).convert_units(january_2020().to_vec())[..] {
        [first, last] => Ok((first, last)),
        _ => Err("two dates convert to two numbers".into()),
    }
}

/// `test_extent_units`: images whose extents are dates on either axis.
fn extent_units(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let arr = Grid::from_fn(10, 10, |j, i| (i + j) as f64);
    let plasma = || -> Result<ImshowOptions, Box<dyn std::error::Error>> {
        Ok(ImshowOptions {
            origin: Some(Origin::Lower),
            colorizer: ColorizerOptions::default().cmap(colormaps::get("plasma")?),
            ..ImshowOptions::default()
        })
    };
    let day = || DateFormatter::new("%d");

    let ax = &axs[(0, 0)];
    ax.set_title("Date extents on y axis");
    let (first, last) = january_2020_on(ax, AxisName::Y)?;
    let extent = Extent::new(1.0, 11.0, first, last);
    ax.imshow(
        arr.clone(),
        ImshowOptions {
            extent: Some(extent),
            ..plasma()?
        },
    )?;

    let ax = &axs[(0, 1)];
    ax.set_title("Date extents on x axis (Day of Jan 2020)");
    let (first, last) = january_2020_on(ax, AxisName::X)?;
    let extent = Extent::new(first, last, 1.0, 11.0);
    ax.imshow(
        arr.clone(),
        ImshowOptions {
            extent: Some(extent),
            ..plasma()?
        },
    )?;
    ax.xaxis().set_major_formatter(day());

    let ax = &axs[(1, 0)];
    let (x0, x1) = january_2020_on(ax, AxisName::X)?;
    let (y0, y1) = january_2020_on(ax, AxisName::Y)?;
    let extent = Extent::new(x0, x1, y0, y1);
    ax.imshow(
        arr.clone(),
        ImshowOptions {
            extent: Some(extent),
            ..plasma()?
        },
    )?;
    ax.xaxis().set_major_formatter(day());
    ax.set_xlabel("Day of Jan 2020");

    let ax = &axs[(1, 1)];
    let image = ax.imshow(arr, plasma()?)?;
    let (x0, x1) = january_2020_on(ax, AxisName::X)?;
    let (y0, y1) = january_2020_on(ax, AxisName::Y)?;
    image.set_extent(Extent::new(x1, x0, y1, y0));
    ax.xaxis().set_major_formatter(day());
    ax.set_xlabel("Day of Jan 2020");
    Ok(())
}

/// A polygon through `points`.
fn polygon(points: &[(f64, f64)]) -> Polygon {
    Polygon::new(points.iter().map(|&(x, y)| Point::new(x, y)), true)
}

/// `test_preset_clip_paths`: lines, patches and an annotation clipped to
/// a diamond, and a figure-level patch clipped to it too.
fn preset_clip_paths(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let diamond = ax.add_patch(polygon(&[(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)]));
    diamond.update_props(
        PatchProps::default()
            .facecolor("#ddffdd")
            .edgecolor("#00ff00")
            .linewidth(2.0)
            .alpha(0.5),
    );
    // Like the test, the clip paths are set before the artists are added,
    // so the axes do not clip them to their box.
    let clip = || ClipPathSpec::Artist(diamond.id());
    let ticked = PathEffect::with_ticked_stroke(TickedStroke::default());
    ax.add_line(
        &[-1.0, 1.0],
        &[0.5, 0.5],
        LineProps::default().clip_path(clip()),
    )?
    .set_path_effects(vec![ticked]);
    ax.add_line(
        &[-1.0, 1.0],
        &[-0.5, -0.5],
        LineProps::default().color("r").clip_path(clip()),
    )?;
    ax.add_artist_with(
        polygon(&[(-1.0, 1.0), (0.0, 1.0), (0.0, -0.25)]),
        PatchProps::default()
            .facecolor("#beefc0")
            .alpha(0.3)
            .edgecolor("#faded0")
            .linewidth(2.0)
            .clip_path(clip()),
    );
    let arrow = LegacyArrowProps {
        patch: PatchProps::default().color("k"),
        ..LegacyArrowProps::default()
    };
    ax.annotate(
        "Annotation",
        Point::new(-0.75, -0.75),
        AnnotateOptions {
            xytext: Some(Point::new(0.1, 0.75).into()),
            arrowprops: Some(ArrowProps::Legacy(arrow)),
            ..AnnotateOptions::default()
        },
    )?
    .set_clip_on(true)
    .set_clip_path(Some(clip()));
    fig.add_artist(polygon(&[(0.0, 0.0), (0.0, 0.5), (0.5, 0.5), (0.5, 0.0)]))
        .update_props(
            PatchProps::default()
                .facecolor("g")
                .edgecolor("y")
                .linewidth(2.0)
                .alpha(0.3),
        )
        .set_clip_on(true)
        .set_clip_path(Some(clip()));
    ax.set_xlim(-1.0, 1.0)?.set_ylim(-1.0, 1.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_ecdf`: plain and complementary compressed ECDFs with infinite
/// values, both ways round.
fn ecdf(fig: &Figure) -> CaseResult {
    let data = [
        0.0,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        1.0,
        1.0,
        2.0,
    ];
    let weights: Vec<f64> = (0..7).map(f64::from).collect();
    let axs = fig.subplots(1, 2)?.into_vec();
    for (ax, orientation) in axs
        .iter()
        .zip([Orientation::Vertical, Orientation::Horizontal])
    {
        ax.ecdf(
            &data,
            EcdfOptions {
                orientation,
                ..EcdfOptions::default()
            },
        )?;
        ax.ecdf(
            &data,
            EcdfOptions {
                weights: Some(weights.clone()),
                complementary: true,
                orientation,
                compress: true,
                line: LineProps::default().linestyle(LineStyle::Dotted),
            },
        )?;
    }
    Ok(())
}

/// The orientation tests' four normal samples of 100 with std 6 to 9.
fn orientation_data() -> Vec<Vec<f64>> {
    let mut stream = SpecRng::new(19_680_801);
    (6..10)
        .map(|std| stream.normal(100, 0.0, f64::from(std)))
        .collect()
}

/// `RdBu` from 4 to 20, the test's shared colorizer.
fn colorizer() -> Result<ColorizerOptions, Box<dyn std::error::Error>> {
    Ok(ColorizerOptions::default()
        .cmap(colormaps::get("RdBu")?)
        .norm(Normalize::new(4.0, 20.0)))
}

/// `test_use_colorizer_keyword`: one colorizer for scatter, hexbin,
/// images, meshes, contours and a figure image (plt-rs has no shared
/// colorizer object: every artist gets the same norm and colormap).
fn use_colorizer_keyword(fig: &Figure) -> CaseResult {
    let mut stream = SpecRng::new(0);
    let rand_x = stream.uniform(100, 0.0, 1.0);
    let rand_y = stream.uniform(100, 0.0, 1.0);
    let c = Grid::from_fn(5, 5, |r, col| (r * 5 + col) as f64);
    let flat = c.as_slice().to_vec();
    let transposed = c.transposed().as_slice().to_vec();
    let cmap: Colormap = colormaps::get("RdBu")?;
    let axes = fig.subplots(3, 4)?;

    let mapped = plt::axes::MappedColors::new(flat.clone())
        .cmap(cmap.clone())
        .norm(Normalize::new(4.0, 20.0));
    axes[(0, 0)].scatter_with(
        flat.clone(),
        flat.clone(),
        ScatterOptions {
            colors: ScatterColors::from(mapped),
            ..ScatterOptions::default()
        },
    )?;
    axes[(0, 1)].hexbin(
        &rand_x,
        &rand_y,
        HexbinOptions {
            gridsize: HexGridSize::Shape(2, 2),
            colorizer: colorizer()?,
            ..HexbinOptions::default()
        },
    )?;
    axes[(0, 2)].imshow(
        c.clone(),
        ImshowOptions {
            colorizer: colorizer()?,
            ..ImshowOptions::default()
        },
    )?;
    let pcolor = || -> Result<PcolorOptions, Box<dyn std::error::Error>> {
        Ok(PcolorOptions {
            colorizer: colorizer()?,
            ..PcolorOptions::default()
        })
    };
    axes[(0, 3)].pcolor(Mesh::new(c.clone()), pcolor()?)?;
    axes[(1, 0)].pcolormesh(Mesh::new(c.clone()), pcolor()?)?;
    let fast = || -> Result<PcolorfastOptions, Box<dyn std::error::Error>> {
        Ok(PcolorfastOptions {
            colorizer: colorizer()?,
            ..PcolorfastOptions::default()
        })
    };
    axes[(1, 1)].pcolorfast(None, c.clone(), fast()?)?;
    let edges = (
        MeshCoords::from(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]),
        MeshCoords::from(vec![0.0, 1.0, 2.0, 3.0, 5.0, 6.0]),
    );
    axes[(1, 2)].pcolorfast(Some(edges), c.clone(), fast()?)?;
    let corner = Grid::from_fn(4, 4, |r, col| c[(r, col)]);
    let grids = (
        MeshCoords::from(c.transposed()),
        MeshCoords::from(c.clone()),
    );
    axes[(1, 3)].pcolorfast(Some(grids), corner, fast()?)?;
    let contour = || ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(cmap.clone()),
            vmin: Some(4.0),
            vmax: Some(20.0),
            ..ColorizerOptions::default()
        },
        ..ContourOptions::default()
    };
    axes[(2, 0)].contour(&QuadField::new(c.clone()), contour())?;
    axes[(2, 1)].contourf(&QuadField::new(c.clone()), contour())?;
    let tri = Triangulation::delaunay(&transposed, &flat)?;
    axes[(2, 2)].tricontour(&tri, &flat, contour())?;
    axes[(2, 3)].tricontourf(&tri, &flat, contour())?;
    let big = Grid::from_fn(75, 75, |r, col| c[(r / 15, col / 15)]);
    fig.figimage(
        big,
        FigimageOptions {
            colorizer: colorizer()?,
            ..FigimageOptions::default()
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_axes_d/2dcolor_plot", color_2d_plot),
    FigureCase::new("mpl_axes_d/centered_bar_label_nonlinear", |fig| {
        let ax = fig.subplot();
        let bars = ax.barh(
            vec!["c".to_owned(), "b".to_owned(), "a".to_owned()],
            [1000.0, 5000.0, 7000.0],
            BarOptions::default(),
        )?;
        ax.set_xscale("log")?;
        ax.set_limits_with(
            AxisName::X,
            Limits {
                low: Some(1.0),
                ..Limits::default()
            },
        )?;
        ax.bar_label(
            &bars,
            BarLabelOptions {
                label_type: BarLabelType::Center,
                ..BarLabelOptions::default()
            },
        )?;
        ax.set_axis_off();
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/bar_all_nan", |fig| {
        let ax = fig.subplot();
        ax.bar([f64::NAN], [f64::NAN], BarOptions::default())?;
        ax.bar([1.0], [1.0], BarOptions::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/extent_units", extent_units),
    FigureCase::new("mpl_axes_d/preset_clip_paths", preset_clip_paths),
    classic("mpl_axes_d/ecdf", ecdf),
    classic("mpl_axes_d/violinplot_orientation", |fig| {
        let options = ViolinplotOptions {
            draw: ViolinDrawOptions {
                orientation: Orientation::Horizontal,
                ..ViolinDrawOptions::default()
            },
            ..ViolinplotOptions::default()
        };
        fig.subplot().violinplot(&orientation_data(), options)?;
        Ok(())
    }),
    classic("mpl_axes_d/boxplot_orientation", |fig| {
        let options = BoxplotOptions {
            draw: BxpOptions {
                orientation: Orientation::Horizontal,
                ..BxpOptions::default()
            },
            ..BoxplotOptions::default()
        };
        fig.subplot().boxplot(&orientation_data(), options)?;
        Ok(())
    }),
    classic("mpl_axes_d/use_colorizer_keyword", use_colorizer_keyword),
    classic(
        "mpl_axes_d/axes_set_position_external_bbox_unchanged",
        |fig| {
            fig.add_axes([0.0, 0.0, 1.0, 1.0])?
                .set_position([0.25, 0.25, 0.5, 0.5]);
            Ok(())
        },
    ),
];
