//! The bar charts of the section: grouped, coloured, horizontal, labelled,
//! stacked and broken bars, categorical plotting, gradient bars, a hat
//! graph and a horizontal distribution chart.

use std::rc::Rc;

use plt::axes::{
    Axes, BandAlign, BarContainer, BarLabelFormat, BarLabelOptions, BarLabelType, BarOptions,
    ErrorbarOptions, GroupedBarData, GroupedBarOptions, ImshowOptions,
};
use plt::colors::colormaps;
use plt::figure::Share;
use plt::image::{ColorizerOptions, Extent, Interpolation};
use plt::layout::LayoutEngine;
use plt::legend::{BboxToAnchor, LegendOptions};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::rc::Aspect;

use crate::figures::cases::common::padded;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The penguin species of the bar examples.
const SPECIES: [&str; 3] = ["Adelie", "Chinstrap", "Gentoo"];

/// The people of the horizontal bar examples.
const PEOPLE: [&str; 5] = ["Tom", "Dick", "Harry", "Slim", "Jim"];

fn barchart(fig: &Figure) -> CaseResult {
    let penguin_means = GroupedBarData::Named(vec![
        ("Bill Depth".into(), vec![18.35, 18.43, 14.98]),
        ("Bill Length".into(), vec![38.79, 48.83, 47.50]),
        ("Flipper Length".into(), vec![189.95, 195.82, 217.19]),
    ]);
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    let options = GroupedBarOptions {
        tick_labels: SPECIES.map(String::from).to_vec(),
        group_spacing: 1.0,
        ..GroupedBarOptions::default()
    };
    for container in ax.grouped_bar(penguin_means, options)? {
        ax.bar_label(&container, &padded(3.0))?;
    }
    ax.set_ylabel("Length (mm)");
    ax.set_title("Penguin attributes by species");
    ax.legend(LegendOptions::default().loc("upper left").ncols(3))?;
    ax.set_ylim(0.0, 250.0);
    Ok(())
}

fn bar_colors(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = BarOptions {
        bar_labels: ["red", "blue", "_red", "orange"].map(String::from).to_vec(),
        color: ["tab:red", "tab:blue", "tab:red", "tab:orange"]
            .map(Color::from)
            .to_vec(),
        ..BarOptions::default()
    };
    ax.bar(
        ["apple", "blueberry", "cherry", "orange"],
        [40.0, 100.0, 30.0, 55.0],
        options,
    )?;
    ax.set_ylabel("fruit supply");
    ax.set_title("Fruit supply by kind and color");
    ax.legend(LegendOptions::default().title("Fruit color"))?;
    Ok(())
}

/// Horizontal bars with `xerr` error bars.
fn with_xerr(error: Vec<f64>) -> BarOptions {
    BarOptions {
        errors: ErrorbarOptions {
            xerr: Some(error.into()),
            ..ErrorbarOptions::default()
        },
        ..BarOptions::default()
    }
}

fn barh(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.barh(
        PEOPLE.to_vec(),
        [5.0, 7.0, 6.0, 4.0, 9.0],
        with_xerr(vec![0.2, 0.4, 0.3, 0.6, 0.2]),
    )?;
    ax.yaxis().set_inverted(true);
    ax.set_xlabel("Performance");
    ax.set_title("How fast do you want to go today?");
    Ok(())
}

/// `bar_label` options putting the labels in the middle of the bars.
fn centred() -> BarLabelOptions {
    BarLabelOptions {
        label_type: BarLabelType::Center,
        ..BarLabelOptions::default()
    }
}

/// Bars of `thickness` at `categories`, stacked series by series on the
/// previous ones; each series is labelled and gets its own container.
fn stacked(
    ax: &Axes,
    categories: &[&str],
    series: &[(&str, [f64; 3])],
    thickness: f64,
) -> Result<Vec<BarContainer>, Box<dyn std::error::Error>> {
    let mut bottom = [0.0; 3];
    let mut containers = Vec::new();
    for (label, counts) in series {
        let options = BarOptions {
            thickness: vec![thickness],
            base: bottom.to_vec(),
            label: Some((*label).to_owned()),
            ..BarOptions::default()
        };
        containers.push(ax.bar(categories.to_vec(), counts, options)?);
        for (b, count) in bottom.iter_mut().zip(counts) {
            *b += count;
        }
    }
    Ok(containers)
}

fn bar_label_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let series = [("Male", [73.0, 34.0, 61.0]), ("Female", [73.0, 34.0, 58.0])];
    for container in stacked(&ax, &SPECIES, &series, 0.6)? {
        ax.bar_label(&container, &centred())?;
    }
    ax.set_title("Number of penguins by sex");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

/// The horizontal bars with errors that two figures of `bar_label_demo`
/// label: the axes, the bars and the errors.
fn performance_bars(
    fig: &Figure,
) -> Result<(Axes, BarContainer, Vec<f64>), Box<dyn std::error::Error>> {
    let mut rng = SpecRng::new(19_680_801);
    let y_pos = arange(0.0, 5.0, 1.0);
    let performance: Vec<f64> = rng
        .uniform(5, 0.0, 1.0)
        .iter()
        .map(|u| 3.0 + 10.0 * u)
        .collect();
    let error = rng.uniform(5, 0.0, 1.0);
    let ax = fig.subplot();
    let hbars = ax.barh(&y_pos, performance, with_xerr(error.clone()))?;
    ax.set_yticks(y_pos).set_yticklabels(PEOPLE)?;
    ax.yaxis().set_inverted(true);
    ax.set_xlabel("Performance");
    ax.set_title("How fast do you want to go today?");
    Ok((ax, hbars, error))
}

fn bar_label_demo_2(fig: &Figure) -> CaseResult {
    let (ax, hbars, _) = performance_bars(fig)?;
    let options = BarLabelOptions {
        fmt: BarLabelFormat::Template("%.2f".to_owned()),
        ..BarLabelOptions::default()
    };
    ax.bar_label(&hbars, &options)?;
    ax.set_xlim(ax.xlim().0, 15.0);
    Ok(())
}

fn bar_label_demo_3(fig: &Figure) -> CaseResult {
    let (ax, hbars, error) = performance_bars(fig)?;
    let options = BarLabelOptions {
        labels: error.iter().map(|e| format!("±{e:.2}")).collect(),
        ..padded(8.0)
    };
    for label in ax.bar_label(&hbars, &options)? {
        label.set_color("b").set_fontsize(14.0);
    }
    ax.set_xlim(ax.xlim().0, 16.0);
    Ok(())
}

/// One bar per name, labelled through `fmt`, with the axes set up by
/// `ylabel`, `title` and `ytop`.
fn labelled_sales(
    fig: &Figure,
    names: &[&str],
    values: &[f64],
    (ylabel, title, ytop): (&str, &str, f64),
    fmt: BarLabelFormat,
) -> CaseResult {
    let ax = fig.subplot();
    let bars = ax.bar(names.to_vec(), values, BarOptions::default())?;
    ax.set_ylabel(ylabel);
    ax.set_title(title);
    ax.set_ylim(0.0, ytop);
    let options = BarLabelOptions {
        fmt,
        ..BarLabelOptions::default()
    };
    ax.bar_label(&bars, &options)?;
    Ok(())
}

fn bar_label_demo_4(fig: &Figure) -> CaseResult {
    labelled_sales(
        fig,
        &["Coffee", "Salted Caramel", "Pistachio"],
        &[4000.0, 2000.0, 7000.0],
        ("pints sold", "Gelato sales by flavor", 8000.0),
        BarLabelFormat::Template("{:,.0f}".to_owned()),
    )
}

fn bar_label_demo_5(fig: &Figure) -> CaseResult {
    labelled_sales(
        fig,
        &["Lion", "Gazelle", "Cheetah"],
        &[50.0, 60.0, 75.0],
        ("speed in MPH", "Running speeds", 80.0),
        BarLabelFormat::Func(Rc::new(|x| format!("{:.1} km/h", x * 1.61))),
    )
}

fn bar_stacked(fig: &Figure) -> CaseResult {
    let species = [
        "Adelie\n $\\mu=$3700.66g",
        "Chinstrap\n $\\mu=$3733.09g",
        "Gentoo\n $\\mu=5076.02g$",
    ];
    let series = [("Below", [70.0, 31.0, 58.0]), ("Above", [82.0, 37.0, 66.0])];
    let ax = fig.subplot();
    stacked(&ax, &species, &series, 0.5)?;
    ax.set_title("Number of penguins with above average body mass");
    ax.legend(LegendOptions::default().loc("upper right"))?;
    Ok(())
}

/// `np.column_stack([starts, np.full(n, width)])`.
fn ranges(starts: impl IntoIterator<Item = f64>, width: f64) -> Vec<(f64, f64)> {
    starts.into_iter().map(|start| (start, width)).collect()
}

fn broken_barh(fig: &Figure) {
    let mut rng = SpecRng::new(19_680_801);
    let cpu_3 = ranges(rng.uniform(61, 0.0, 10.0), 0.05);
    let network = ranges(rng.uniform(10, 0.0, 10.0), 0.05);
    let ax = fig.subplot();
    let cpus = [
        ("CPU 1", vec![(0.0, 3.0), (3.5, 1.0), (5.0, 5.0)]),
        ("CPU 2", ranges(linspace(0.0, 9.0, 10), 0.5)),
        ("CPU 3", cpu_3),
        ("CPU 4", vec![(2.0, 1.7), (7.0, 1.2)]),
    ];
    for (name, xranges) in cpus {
        ax.broken_barh(&xranges, (name, 0.4), BandAlign::Center);
    }
    // `color=` colours the faces and the edges.
    let devices = [
        ("disk", vec![(1.0, 1.5)], "tab:orange"),
        ("network", network, "tab:green"),
    ];
    for (name, xranges, color) in devices {
        ax.broken_barh(&xranges, (name, 0.4), BandAlign::Center)
            .set_facecolor(color)
            .set_edgecolor(color);
    }
    ax.set_xlim(0.0, 10.0);
    ax.yaxis().set_inverted(true);
    ax.set_title("Resource usage");
}

fn categorical_variables_1(fig: &Figure) -> CaseResult {
    let names = ["apple", "orange", "lemon", "lime"];
    let values = [10.0, 15.0, 5.0, 20.0];
    let axs = fig.subplots_shared(1, 3, Share::None, Share::All)?;
    axs[(0, 0)].bar(names.to_vec(), values, BarOptions::default())?;
    axs[(0, 1)].scatter(names.to_vec(), values.to_vec())?;
    axs[(0, 2)].plot(names.to_vec(), values)?;
    fig.suptitle("Categorical Plotting");
    Ok(())
}

fn categorical_variables_2(fig: &Figure) -> CaseResult {
    let cat = ["bored", "happy", "bored", "bored", "happy", "bored"];
    let dog = ["happy", "happy", "happy", "happy", "bored", "bored"];
    let activity = [
        "combing", "drinking", "feeding", "napping", "playing", "washing",
    ];
    let ax = fig.subplot();
    ax.plot_with(
        activity.to_vec(),
        dog.to_vec(),
        LineProps::default().label("dog"),
    )?;
    ax.plot_with(
        activity.to_vec(),
        cat.to_vec(),
        LineProps::default().label("cat"),
    )?;
    ax.legend(LegendOptions::default())?;
    Ok(())
}

/// The example's `gradient_image`: a 2 × 2 gradient in `direction` (0
/// vertical, 1 horizontal) spanning `cmap_range` of the colormap, drawn
/// bicubically over `options.extent`.
fn gradient_image(
    ax: &Axes,
    direction: f64,
    (a, b): (f64, f64),
    cmap: &str,
    options: ImshowOptions,
) -> CaseResult {
    let phi = direction * std::f64::consts::PI / 2.0;
    let (vx, vy) = (phi.cos(), phi.sin());
    let corners = [[vx, vx + vy], [0.0, vy]];
    let max = corners
        .iter()
        .flatten()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let x = Grid::from_fn(2, 2, |r, c| a + (b - a) / max * corners[r][c]);
    let options = ImshowOptions {
        colors: ColorizerOptions::default()
            .cmap(colormaps::get(cmap)?)
            .limits(Some(0.0), Some(1.0)),
        interpolation: Some(Interpolation::Bicubic),
        aspect: Some(Aspect::Auto),
        ..options
    };
    ax.imshow(x, options)?;
    Ok(())
}

fn gradient_bar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 1.0);
    let background = ImshowOptions {
        extent: Some(Extent::new(0.0, 1.0, 0.0, 1.0)),
        transform: Some(ax.trans_axes()),
        alpha: Some(0.5.into()),
        ..ImshowOptions::default()
    };
    gradient_image(&ax, 1.0, (0.2, 0.8), "RdYlGn", background)?;
    let tops = SpecRng::new(19_680_801).uniform(10, 0.0, 1.0);
    for (left, top) in (0..10).map(|i| f64::from(i) + 0.15).zip(tops) {
        let bar = ImshowOptions {
            extent: Some(Extent::new(left, left + 0.7, 0.0, top)),
            ..ImshowOptions::default()
        };
        gradient_image(&ax, 0.3, (0.0, 0.8), "Blues_r", bar)?;
    }
    Ok(())
}

fn hat_graph(fig: &Figure) -> CaseResult {
    let player_a = [5.0, 15.0, 22.0, 20.0, 25.0];
    let player_b = [25.0, 32.0, 34.0, 30.0, 27.0];
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    let mut colors = vec![Color::none()];
    colors.extend(
        plt::rc::params()
            .axes_prop_cycle
            .colors()
            .into_iter()
            .map(Color::from),
    );
    let options = GroupedBarOptions {
        base: player_a.to_vec(),
        tick_labels: ["I", "II", "III", "IV", "V"].map(String::from).to_vec(),
        labels: ["Player A", "Player B"].map(String::from).to_vec(),
        edgecolor: vec![Color::from("black")],
        group_spacing: 0.8,
        colors,
        ..GroupedBarOptions::default()
    };
    let rises = [player_a, player_b].map(|values| {
        values
            .iter()
            .zip(&player_a)
            .map(|(v, a)| v - a)
            .collect::<Vec<_>>()
    });
    let bars = ax.grouped_bar(rises.to_vec(), options)?;
    for (container, heights) in bars.iter().zip([player_a, player_b]) {
        let options = BarLabelOptions {
            labels: heights.iter().map(|h| format!("{h}")).collect(),
            ..padded(4.0)
        };
        ax.bar_label(container, &options)?;
    }
    ax.set_xlabel("Games");
    ax.set_ylabel("Score");
    ax.set_ylim(0.0, 60.0);
    ax.set_title("Scores by number of game and players");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn horizontal_barchart_distribution(fig: &Figure) -> CaseResult {
    let category_names = [
        "Strongly disagree",
        "Disagree",
        "Neither agree nor disagree",
        "Agree",
        "Strongly agree",
    ];
    let results: [(&str, [f64; 5]); 6] = [
        ("Question 1", [10.0, 15.0, 17.0, 32.0, 26.0]),
        ("Question 2", [26.0, 22.0, 29.0, 10.0, 13.0]),
        ("Question 3", [35.0, 37.0, 7.0, 2.0, 19.0]),
        ("Question 4", [32.0, 11.0, 9.0, 15.0, 33.0]),
        ("Question 5", [21.0, 29.0, 5.0, 5.0, 40.0]),
        ("Question 6", [8.0, 19.0, 5.0, 30.0, 38.0]),
    ];
    let labels: Vec<&str> = results.iter().map(|(label, _)| *label).collect();
    let cmap = colormaps::get("RdYlGn")?;
    let ax = fig.subplot();
    ax.yaxis().set_inverted(true);
    ax.xaxis().set_visible(false);
    let total = results
        .iter()
        .map(|(_, row)| row.iter().sum::<f64>())
        .fold(f64::NEG_INFINITY, f64::max);
    ax.set_xlim(0.0, total);
    let mut starts = [0.0; 6];
    for (i, (name, position)) in category_names
        .iter()
        .zip(linspace(0.15, 0.85, 5))
        .enumerate()
    {
        let color = cmap.sample(position);
        let widths: Vec<f64> = results.iter().map(|(_, row)| row[i]).collect();
        let options = BarOptions {
            base: starts.to_vec(),
            thickness: vec![0.5],
            label: Some((*name).to_owned()),
            color: vec![Color::from(color)],
            ..BarOptions::default()
        };
        let rects = ax.barh(labels.clone(), &widths, options)?;
        let text_color = if color.r * color.g * color.b < 0.5 {
            "white"
        } else {
            "darkgrey"
        };
        for label in ax.bar_label(&rects, &centred())? {
            label.set_color(text_color);
        }
        for (start, width) in starts.iter_mut().zip(widths) {
            *start += width;
        }
    }
    ax.legend(
        LegendOptions::default()
            .ncols(category_names.len())
            .bbox_to_anchor(BboxToAnchor::Point(0.0, 1.0))
            .loc("lower left")
            .fontsize(plt::fonts::RelativeSize::Small),
    )?;
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_lines/barchart", barchart),
    FigureCase::new("gallery_lines/bar_colors", bar_colors),
    FigureCase::new("gallery_lines/barh", barh),
    FigureCase::new("gallery_lines/bar_label_demo_1", bar_label_demo_1),
    FigureCase::new("gallery_lines/bar_label_demo_2", bar_label_demo_2),
    FigureCase::new("gallery_lines/bar_label_demo_3", bar_label_demo_3),
    FigureCase::new("gallery_lines/bar_label_demo_4", bar_label_demo_4),
    FigureCase::new("gallery_lines/bar_label_demo_5", bar_label_demo_5),
    FigureCase::new("gallery_lines/bar_stacked", bar_stacked),
    FigureCase::new("gallery_lines/broken_barh", |fig| {
        broken_barh(fig);
        Ok(())
    }),
    FigureCase::new(
        "gallery_lines/categorical_variables_1",
        categorical_variables_1,
    )
    .sized(9.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_lines/categorical_variables_2",
        categorical_variables_2,
    ),
    FigureCase::new("gallery_lines/gradient_bar", gradient_bar),
    FigureCase::new("gallery_lines/hat_graph", hat_graph),
    FigureCase::new(
        "gallery_lines/horizontal_barchart_distribution",
        horizontal_barchart_distribution,
    )
    .sized(9.2, 5.0, 100.0),
];
