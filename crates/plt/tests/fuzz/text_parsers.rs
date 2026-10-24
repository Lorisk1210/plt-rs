//! Parsers of user strings: colours, format strings, markers, dashes,
//! legend locations, scale names, mosaics, dates, Python format templates,
//! hatches, fontconfig patterns and every other `FromStr` type of the
//! public API. Each parsed value is also put to use (resolved, measured,
//! laid out), since a parser that accepts a value promises it works.

use std::fmt::Debug;
use std::str::FromStr;

use plt::colors::{Color, is_color_like, to_rgba};
use plt::dates::iso::{Datetime64, fromisoformat};
use plt::dates::{Epoch, TimeZone, datestr2num, num2date};
use plt::figure::{Figure, Mosaic, MosaicOptions};
use plt::fonts::{FontProperties, FontconfigPattern};
use plt::legend::LegendLoc;
use plt::lines::{FormatString, LineStyle};
use plt::markers::{Marker, MarkerStyle};
use plt::rc::RcParams;
use plt::scale::ScaleName;
use plt_core::hatch::pattern_path;
use plt_core::pyfmt::{FormatSpec, PrintfFormat, StrTemplate, TemplateValue, auto_format};
use plt_text::TextEngine;

use crate::harness::fuzz;

/// Colour specifications.
const COLORS: [&str; 16] = [
    "red",
    "C3",
    "C10",
    "#ff000080",
    "#abc",
    "#AbCd",
    "0.5",
    "xkcd:sky blue",
    "tab:orange",
    "none",
    "k",
    "(1, 0, 0)",
    "(0.1, 0.2, 0.3, 0.4)",
    "red, 0.5",
    "('green', 0.3)",
    "grey",
];

/// Pieces of colour specifications.
const COLOR_PIECES: [&str; 16] = [
    "#", "C", "xkcd:", "tab:", "(", ")", ",", "0.", "1", "f", "a", " ", "red", "-", "e", "9",
];

/// Line format strings.
const FORMATS: [&str; 10] = [
    "r-", "bo--", "C1:", "k^-.", "-", "o", "g", "1", "tab:blue", "#ff0000",
];

/// Pieces of format strings.
const FORMAT_PIECES: [&str; 18] = [
    "-", "--", "-.", ":", "o", "^", "s", "1", "|", "_", ".", ",", "r", "C", "k", "w", "$", " ",
];

/// Marker specifications.
const MARKERS: [&str; 12] = [
    "o",
    "s",
    "^",
    "8",
    "P",
    "$x$",
    "$\\alpha$",
    "None",
    "",
    "4",
    "|",
    "$\\frac{1}{2}$",
];

/// Line styles and dash specifications.
const LINESTYLES: [&str; 10] = [
    "-",
    "--",
    "dashdot",
    "dotted",
    "none",
    "(0, (1, 1))",
    "(5, (10, 3))",
    "(0, (3, 5, 1, 5))",
    "(0, ())",
    "(0.0, (0.0, 0.0))",
];

/// Pieces of dash specifications.
const DASH_PIECES: [&str; 10] = ["(", ")", ",", " ", "0", "1e-300", "-1", "1e300", "nan", "."];

/// Legend locations.
const LEGEND_LOCS: [&str; 8] = [
    "best",
    "upper right",
    "center",
    "outside upper left",
    "outside right lower",
    "lower center",
    "right",
    "outside top",
];

/// Scale names.
const SCALES: [&str; 6] = ["linear", "log", "symlog", "logit", "asinh", "function"];

/// Mosaic layouts.
const MOSAICS: [&str; 8] = [
    "AB;CC",
    "A.B\nCCC",
    "AAB\nAAB\nCCD",
    ".",
    "A",
    "AB\nBA",
    "ABC;DEF;GHI",
    "AAA;B.C",
];

/// Pieces of mosaics.
const MOSAIC_PIECES: [&str; 8] = ["A", "B", "C", ".", ";", "\n", " ", "AA"];

/// Date strings.
const DATES: [&str; 10] = [
    "2020-06-15",
    "2020-06-15T12:00:00",
    "2020-06-15 12:00:00.123456+02:00",
    "1970-01-01T00:00:01.5",
    "-0001-01-01",
    "9999-12-31T23:59:59.999999",
    "2020-W01-1",
    "NaT",
    "2020-02-30",
    "0000-12-31T00:00:00",
];

/// Pieces of date strings.
const DATE_PIECES: [&str; 12] = [
    "-",
    ":",
    "T",
    " ",
    "Z",
    "+",
    ".",
    "W",
    "2020",
    "12",
    "99",
    "000000000000000000",
];

/// `strftime` formats.
const STRFTIME: [&str; 6] = [
    "%Y-%m-%d",
    "%H:%M:%S.%f",
    "%a %b %j",
    "%%",
    "%",
    "%-d %Z %z",
];

/// printf templates.
const PRINTF: [&str; 8] = [
    "%d",
    "%.3f",
    "%10.2e",
    "%-+#08.3g",
    "%s",
    "%%",
    "%x",
    "%5.1f%%",
];

/// `str.format` templates.
const TEMPLATES: [&str; 8] = [
    "{x}", "{x:.2f}", "{x:>10}", "{:,.3e}", "{0:{1}}", "{x!r}", "{pos}", "{{}}",
];

/// Pieces of format templates and specs.
const FORMAT_SPEC_PIECES: [&str; 22] = [
    "{",
    "}",
    "%",
    ":",
    ".",
    ",",
    "_",
    "x",
    "pos",
    "0",
    "9",
    "<",
    ">",
    "^",
    "=",
    "+",
    "#",
    "e",
    "f",
    "g",
    "%",
    "999999999",
];

/// Hatch patterns.
const HATCHES: [&str; 8] = ["/", "\\\\", "|", "-", "+", "x", "o", "O.*"];

/// Fontconfig patterns.
const FONTCONFIG: [&str; 6] = [
    "DejaVu Sans",
    "serif:bold",
    "Sans-12:italic",
    "monospace:size=20:weight=light",
    "Times\\-New:slant=italic",
    ":family=cursive",
];

/// Pieces of fontconfig patterns.
const FONTCONFIG_PIECES: [&str; 14] = [
    ":", "-", ",", "=", "\\", "size", "weight", "bold", "12", "e30", " ", "family", "slant", "x",
];

/// General punctuation for the other `FromStr` types.
const WORD_PIECES: [&str; 18] = [
    "upper", "lower", "left", "right", "center", " ", ",", "=", "(", ")", "-", "pad", "0.3",
    "round", "->", "<-", "arc3", "rad",
];

#[test]
fn colors() {
    fuzz(
        "colors",
        20_000,
        |g| g.text(&COLORS, &COLOR_PIECES),
        |text| {
            let _ = (to_rgba(text, None), is_color_like(text));
            if let Ok(color) = Color::parse(text) {
                let _ = color.resolve_alpha(Some(0.5));
                let _ = color.split_alpha();
            }
        },
    );
}

#[test]
fn line_formats_markers_and_dashes() {
    let rc = RcParams::matplotlib();
    fuzz(
        "line formats",
        10_000,
        |g| {
            let kind = g.below(3);
            let text = match kind {
                0 => g.text(&FORMATS, &FORMAT_PIECES),
                1 => g.text(&MARKERS, &FORMAT_PIECES),
                _ => g.text(&LINESTYLES, &DASH_PIECES),
            };
            (kind, text, g.number())
        },
        |(kind, text, width)| match kind {
            0 => {
                let _ = FormatString::parse_with(text, &rc);
            }
            1 => {
                if let Ok(style) = MarkerStyle::from_str(text) {
                    let _ = (style.path().extents(), style.rotated_deg(*width));
                }
                let _ = Marker::from_str(text);
            }
            _ => {
                if let Ok(style) = LineStyle::from_str(text) {
                    let _ = style.scaled_dashes(*width, &rc);
                    let _ = style.dash_pattern(&rc);
                }
            }
        },
    );
}

#[test]
fn legend_locations_and_scale_names() {
    fuzz(
        "names",
        10_000,
        |g| {
            let examples: Vec<&str> = LEGEND_LOCS.iter().chain(&SCALES).copied().collect();
            g.text(&examples, &WORD_PIECES)
        },
        |text| {
            let _ = LegendLoc::from_str(text);
            let _ = ScaleName::from_str(text);
            let fig = Figure::with_size(1.0, 1.0, 20.0);
            let ax = fig.subplot();
            let _ = ax.set_xscale(text.as_str());
        },
    );
}

#[test]
fn mosaics() {
    fuzz(
        "mosaics",
        5_000,
        |g| g.text(&MOSAICS, &MOSAIC_PIECES),
        |text| {
            if let Ok(mosaic) = Mosaic::from_str(text) {
                let fig = Figure::with_size(2.0, 2.0, 20.0);
                let _ = fig.subplot_mosaic(&mosaic, MosaicOptions::default());
            }
        },
    );
}

#[test]
fn dates() {
    fuzz(
        "dates",
        20_000,
        |g| {
            (
                g.text(&DATES, &DATE_PIECES),
                g.text(&STRFTIME, &["%", "Y", "f", "-", "z", "Z", "j", "U"]),
                g.number(),
            )
        },
        |(text, format, number)| {
            let _ = (
                datestr2num(text),
                fromisoformat(text),
                Datetime64::parse(text),
            );
            if let Ok(epoch) = Epoch::parse(text) {
                let _ = epoch.num2date(*number, &TimeZone::Utc);
            }
            if let Ok(date) = num2date(*number, &TimeZone::Utc) {
                let _ = date.strftime(format);
            }
            let _ = TimeZone::named(text);
        },
    );
}

#[test]
fn python_format_templates() {
    fuzz(
        "pyfmt",
        30_000,
        |g| {
            let examples: Vec<&str> = PRINTF.iter().chain(&TEMPLATES).copied().collect();
            (g.text(&examples, &FORMAT_SPEC_PIECES), g.number())
        },
        |(template, x)| {
            let _ = auto_format(template, *x);
            if let Ok(printf) = PrintfFormat::parse(template) {
                let _ = printf.format(*x);
            }
            if let Ok(spec) = FormatSpec::parse(template) {
                let _ = spec.format_float(*x);
                let _ = spec.format_int(*x as i64);
                let _ = spec.format_str(template);
            }
            if let Ok(parsed) = StrTemplate::parse(template) {
                let _ = parsed.render(*x, Some(3), |s| s);
                let _ = parsed.render(*x, None, |s| s);
                let values = [TemplateValue::Float(*x), TemplateValue::Int(-7)];
                let _ = parsed.render_values(&values, |s| s);
            }
        },
    );
}

#[test]
fn hatches() {
    fuzz(
        "hatches",
        5_000,
        |g| {
            let density = g.pick(&[0, 1, 2, 6, 10, -1, 1000, i32::MAX, i32::MIN]);
            (
                g.text(
                    &HATCHES,
                    &["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*"],
                ),
                density,
            )
        },
        |(pattern, density)| {
            if let Ok(path) = pattern_path(pattern, *density) {
                let _ = path.extents();
            }
        },
    );
}

#[test]
fn font_patterns() {
    let rc = RcParams::matplotlib();
    fuzz(
        "font patterns",
        5_000,
        |g| g.text(&FONTCONFIG, &FONTCONFIG_PIECES),
        |pattern| {
            if let Ok(parsed) = FontconfigPattern::parse(pattern) {
                let _ = parsed.properties();
            }
            if let Ok(props) = FontProperties::from_pattern(pattern, &rc) {
                let _ = props.fontconfig_pattern();
                let request = props.request(&rc);
                let _ = TextEngine::global().layout_line("Ag", &request, 72.0);
            }
        },
    );
}

/// Parses `text` as `T` and formats the result.
fn parse_as<T: FromStr<Err: Debug> + Debug>(text: &str) {
    let _ = format!("{:?}", text.parse::<T>());
}

#[test]
fn every_from_str() {
    fuzz(
        "from_str",
        20_000,
        |g| {
            let examples = [
                "round,pad=0.3",
                "-|>,head_length=0.4,head_width=0.2",
                "arc3,rad=0.2",
                "angle3,angleA=90,angleB=0",
                "bar,fraction=-0.5,angle=1e300",
                "sawtooth,tooth_size=0",
                "roundtooth,pad=1e-300",
                "data",
                "axes fraction",
                "offset points",
                "x-large",
                "1e308",
                "bold",
                "kern=0",
                "+liga[3:5]",
                "nearest",
                "upper left",
                "cycler(color=['r', 'g'])",
                "cycler('lw', [1, 2]) * cycler('ls', ['-', '--'])",
                "fancy,head_length=1e9",
                "BRTL",
                "auto",
                "fd",
            ];
            g.text(&examples, &WORD_PIECES)
        },
        |text| {
            parse_as::<plt::patches::BoxStyle>(text);
            parse_as::<plt::patches::ArrowStyle>(text);
            parse_as::<plt::patches::ConnectionStyle>(text);
            parse_as::<plt::text::FancyArrowProps>(text);
            parse_as::<plt::coords::Coords>(text);
            parse_as::<plt::fonts::FontSize>(text);
            parse_as::<plt::fonts::FontWeight>(text);
            parse_as::<plt::fonts::FontStretch>(text);
            parse_as::<plt::text::Feature>(text);
            parse_as::<plt::rc::Cycler>(text);
            parse_as::<plt::image::Interpolation>(text);
            parse_as::<plt::offsetbox::AnchorLoc>(text);
            parse_as::<plt::table::CellEdges>(text);
            parse_as::<plt::table::TableLoc>(text);
            parse_as::<plt::stats::BinEstimator>(text);
            parse_as::<plt::streamplot::StreamArrowStyle>(text);
            parse_as::<plt::colors::NormName>(text);
            parse_as::<plt::figure::SaveFormat>(text);
            parse_as::<plt::figure::SizeUnit>(text);
            parse_as::<plt::lines::DrawStyle>(text);
            parse_as::<plt::markers::FillStyle>(text);
            parse_as::<plt::axes::Shading>(text);
            parse_as::<plt::axes::ErrorbarFormat>(text);
            parse_as::<plt_render::Paper>(text);
            parse_as::<plt_text::HAlign>(text);
            parse_as::<plt_text::VAlign>(text);
            parse_as::<plt_text::mathtext::MathFont>(text);
        },
    );
}
