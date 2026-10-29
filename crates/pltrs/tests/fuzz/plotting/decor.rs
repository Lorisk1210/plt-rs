//! Decorations: texts with boxes, annotations with arrows, legends,
//! colorbars, tables, titles and labels, and bar labels.

use pltrs::axes::{Axes, BarLabelOptions, BarLabelType};
use pltrs::colorbar::ColorbarOptions;
use pltrs::figure::Figure;
use pltrs::fonts::FontSize;
use pltrs::legend::{LegendLoc, LegendMode, LegendOptions};
use pltrs::orientation::Orientation;
use pltrs::patches::{ArrowStyle, BoxStyle, ConnectionStyle};
use pltrs::table::TableOptions;
use pltrs::text::{AnnotateOptions, ArrowProps, FancyArrowProps, TextBoxProps};
use pltrs::ticker::LabelFormat;
use pltrs::transforms::Point;

use super::Mappables;
use super::axis_decor::TEMPLATES;
use super::data::count;
use crate::harness::Gen;

/// Texts, box styles, arrow and connection styles and legend locations
/// the generators pick from.
const TEXTS: [&str; 7] = [
    "label",
    "$\\sqrt{x}$",
    "",
    "multi\nline\ntext",
    "\u{1F600}",
    "a$b$c$",
    "wrap me please",
];
const BOXES: [&str; 6] = [
    "round,pad=0.3",
    "square",
    "sawtooth,pad=1e-9",
    "roundtooth",
    "larrow",
    "circle,pad=5",
];
const ARROWS: [&str; 6] = [
    "->",
    "-|>",
    "fancy",
    "simple,head_width=2",
    "wedge,tail_width=0",
    "]-[",
];
const CONNECTIONS: [&str; 4] = [
    "arc3,rad=0.3",
    "angle3",
    "bar,fraction=-1e9",
    "arc,armA=0,armB=1e9",
];
const LOCATIONS: [&str; 5] = ["best", "upper right", "center", "lower left", "right"];

/// A decoration.
#[derive(Debug)]
pub(crate) enum Decor {
    Text {
        at: (f64, f64),
        text: String,
        size: f64,
        rotation: f64,
        bbox: Option<String>,
        wrap: bool,
    },
    Annotate {
        text: String,
        xy: (f64, f64),
        xytext: (f64, f64),
        arrow: String,
        connection: String,
    },
    Legend {
        loc: String,
        ncols: usize,
        fontsize: f64,
        title: bool,
        numpoints: usize,
        markerscale: f64,
        expand: bool,
    },
    Colorbar {
        horizontal: bool,
        shrink: f64,
        aspect: f64,
        pad: f64,
        fraction: f64,
        drawedges: bool,
    },
    Table {
        rows: usize,
        columns: usize,
        fontsize: f64,
    },
    Titles {
        title: String,
        xlabel: String,
        ylabel: String,
    },
    BarLabels {
        template: String,
        padding: f64,
        center: bool,
    },
}

impl Decor {
    /// A random decoration.
    pub(crate) fn generate(g: &mut Gen) -> Self {
        match g.below(7) {
            0 => Self::Text {
                at: (g.number_in(-0.5, 1.5), g.number_in(-0.5, 1.5)),
                text: g.pick(&TEXTS).to_owned(),
                size: g.number_in(1.0, 40.0),
                rotation: g.number_in(-360.0, 360.0),
                bbox: g
                    .chance(0.4)
                    .then(|| g.text(&BOXES, &[",", "pad=", "1e300", "-1"])),
                wrap: g.chance(0.2),
            },
            1 => Self::Annotate {
                text: g.pick(&TEXTS).to_owned(),
                xy: (g.number_in(-0.5, 1.5), g.number_in(-0.5, 1.5)),
                xytext: (g.number_in(-0.5, 1.5), g.number_in(-0.5, 1.5)),
                arrow: g.text(&ARROWS, &[",", "=", "head_length", "0", "-", ">"]),
                connection: g.text(&CONNECTIONS, &[",", "=", "rad", "armA", "1e308"]),
            },
            2 => Self::Legend {
                loc: g.pick(&LOCATIONS).to_owned(),
                ncols: count(g, 3),
                fontsize: g.number_in(1.0, 30.0),
                title: g.chance(0.3),
                numpoints: count(g, 4),
                markerscale: g.number_in(0.0, 5.0),
                expand: g.chance(0.2),
            },
            3 => Self::Colorbar {
                horizontal: g.chance(0.4),
                shrink: g.number_in(0.0, 1.5),
                aspect: g.number_in(0.0, 50.0),
                pad: g.number_in(0.0, 0.5),
                fraction: g.number_in(0.0, 0.5),
                drawedges: g.chance(0.3),
            },
            4 => Self::Table {
                rows: g.below(6),
                columns: g.below(6),
                fontsize: g.number_in(1.0, 20.0),
            },
            5 => Self::Titles {
                title: g.pick(&TEXTS).to_owned(),
                xlabel: g.pick(&TEXTS).to_owned(),
                ylabel: g.pick(&TEXTS).to_owned(),
            },
            _ => Self::BarLabels {
                template: g.text(&TEMPLATES, &["%", "{", "}", ".", "9", "f", "x"]),
                padding: g.number_in(-5.0, 10.0),
                center: g.chance(0.5),
            },
        }
    }

    /// Adds the decoration to `ax` of `fig`; errors are fine.
    pub(crate) fn run(&self, fig: &Figure, ax: &Axes, mappables: &Mappables) {
        match self {
            Self::Text {
                at,
                text,
                size,
                rotation,
                bbox,
                wrap,
            } => {
                let handle = ax.text(at.0, at.1, text.as_str());
                handle
                    .set_fontsize(*size)
                    .set_rotation(*rotation)
                    .set_wrap(*wrap);
                if let Some(style) = bbox
                    .as_ref()
                    .and_then(|style| style.parse::<BoxStyle>().ok())
                {
                    handle.set_bbox(Some(TextBoxProps {
                        boxstyle: Some(style),
                        ..TextBoxProps::default()
                    }));
                }
            }
            Self::Annotate {
                text,
                xy,
                xytext,
                arrow,
                connection,
            } => {
                annotate(ax, text, *xy, *xytext, (arrow, connection));
            }
            Self::Legend { .. } => self.legend(ax),
            Self::Colorbar { .. } => self.colorbar(fig, mappables),
            Self::Table {
                rows,
                columns,
                fontsize,
            } => {
                let options = TableOptions {
                    cell_text: (0..*rows)
                        .map(|r| (0..*columns).map(|c| format!("{r}{c}")).collect())
                        .collect(),
                    fontsize: Some(*fontsize),
                    ..TableOptions::default()
                };
                drop(ax.table(options));
            }
            Self::Titles {
                title,
                xlabel,
                ylabel,
            } => {
                ax.set_title(title.as_str());
                ax.set_xlabel(xlabel.as_str()).set_ylabel(ylabel.as_str());
            }
            Self::BarLabels {
                template,
                padding,
                center,
            } => {
                let Some(bars) = mappables.bars.last() else {
                    return;
                };
                let options = BarLabelOptions {
                    fmt: LabelFormat::Template(template.clone()),
                    padding: vec![*padding],
                    label_type: if *center {
                        BarLabelType::Center
                    } else {
                        BarLabelType::Edge
                    },
                    ..BarLabelOptions::default()
                };
                drop(ax.bar_label(bars, options));
            }
        }
    }

    /// Adds a [`Decor::Legend`].
    fn legend(&self, ax: &Axes) {
        let Self::Legend {
            loc,
            ncols,
            fontsize,
            title,
            numpoints,
            markerscale,
            expand,
        } = self
        else {
            return;
        };
        let options = LegendOptions {
            loc: loc.parse::<LegendLoc>().ok(),
            ncols: *ncols,
            fontsize: Some(FontSize::Points(*fontsize)),
            title: title.then(|| "Legend".to_owned()),
            numpoints: Some(*numpoints),
            markerscale: Some(*markerscale),
            mode: if *expand {
                LegendMode::Expand
            } else {
                LegendMode::default()
            },
            ..LegendOptions::default()
        };
        drop(ax.legend(options));
    }

    /// Adds a [`Decor::Colorbar`] for the most recent mappable, if any.
    fn colorbar(&self, fig: &Figure, mappables: &Mappables) {
        let Self::Colorbar {
            horizontal,
            shrink,
            aspect,
            pad,
            fraction,
            drawedges,
        } = self
        else {
            return;
        };
        let options = ColorbarOptions {
            orientation: Some(if *horizontal {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            }),
            shrink: *shrink,
            aspect: *aspect,
            pad: Some(*pad),
            fraction: *fraction,
            drawedges: *drawedges,
            ..ColorbarOptions::default()
        };
        let result = if let Some(image) = mappables.images.last() {
            fig.colorbar(image, options)
        } else if let Some(set) = mappables.contours.last() {
            fig.colorbar(set, options)
        } else if let Some(collection) = mappables.collections.last() {
            fig.colorbar(collection, options)
        } else {
            return;
        };
        drop(result);
    }
}

/// An annotation with a fancy arrow of the `(arrow, connection)` styles,
/// if they parse.
fn annotate(
    ax: &Axes,
    text: &str,
    xy: (f64, f64),
    xytext: (f64, f64),
    (arrow, connection): (&str, &str),
) {
    let arrowprops = match (
        arrow.parse::<ArrowStyle>(),
        connection.parse::<ConnectionStyle>(),
    ) {
        (Ok(arrowstyle), Ok(connectionstyle)) => Some(ArrowProps::Fancy(FancyArrowProps {
            connectionstyle,
            ..FancyArrowProps::new(arrowstyle)
        })),
        _ => None,
    };
    let options = AnnotateOptions {
        xytext: Some(Point::new(xytext.0, xytext.1).into()),
        arrowprops,
        ..AnnotateOptions::default()
    };
    drop(ax.annotate(text, xy, options));
}
