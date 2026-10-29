//! pyplot's figures: numbers, labels, sizes and clearing (`pyplot.figure`
//! steps) and `figaspect`.

use pltrs::figure::{FigSize, Figure, SizeUnit};
use pltrs::pyplot;
use pltrs::text::Text;

use super::super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Outcome, ProbeError};

/// A figure number or label, as matplotlib's `num` argument.
enum Num<'a> {
    Number(u32),
    Label(&'a str),
}

impl<'a> Num<'a> {
    fn parse(value: &'a Json) -> Result<Self, ProbeError> {
        match value {
            Json::String(label) => Ok(Self::Label(label)),
            _ => value
                .as_i64()
                .and_then(|n| u32::try_from(n).ok())
                .map(Self::Number)
                .ok_or_else(|| ProbeError::BadArgs(format!("not a figure number: {value}"))),
        }
    }
}

/// The steps of a pyplot session (see the module docstring of
/// `test_mpl_figure.py`), one result per step.
pub(super) fn figure(args: Args<'_>) -> Outcome {
    let steps = args
        .json("steps")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("steps is not a list".into()))?;
    in_test_style(args, || {
        let mut made: Vec<Option<Figure>> = Vec::new();
        let mut results = Vec::new();
        for step in steps {
            let (made_now, result) = run_step(step, &made)?;
            made.push(made_now);
            results.push(result.unwrap_or(Json::Null));
        }
        Ok(Json::Array(results))
    })
}

type StepResult = (Option<Figure>, Option<Json>);

fn run_step(step: &Json, made: &[Option<Figure>]) -> Result<StepResult, ProbeError> {
    let parts = step.as_array().unwrap_or(&[]);
    let (name, rest) = match parts.split_first() {
        Some((Json::String(name), rest)) => (name.as_str(), rest),
        _ => return Err(ProbeError::BadArgs(format!("bad step {step}"))),
    };
    let (args, kwargs) = match rest.split_last() {
        Some((kwargs @ Json::Object(_), args)) => (args, Some(kwargs)),
        _ => (rest, None),
    };
    let kwarg = |key: &str| kwargs.and_then(|kw| kw.get(key));
    let figure = |made_figure: Figure| Ok((Some(made_figure), None));
    let value = |json: Json| Ok((None, Some(json)));
    match (name, args) {
        ("figure", []) if kwarg("num").is_some() => {
            figure(numbered_figure(kwarg("num"), kwarg("clear"))?)
        }
        ("figure", []) if kwarg("figsize").is_some() => figure(sized_figure(
            kwarg("figsize").unwrap_or(&Json::Null),
            kwarg("dpi"),
        )?),
        ("figure", []) => figure(pyplot::figure()),
        ("figure", [num]) => figure(numbered_figure(Some(num), None)?),
        ("figure_of", [index]) => {
            let fig = made_figure(made, index)?;
            pyplot::figure_activate(&fig);
            figure(fig)
        }
        ("close", []) => {
            pyplot::close();
            Ok((None, None))
        }
        ("close", [num]) => {
            match Num::parse(num)? {
                Num::Number(n) => pyplot::close_num(n),
                Num::Label(label) => pyplot::close_label(label),
            }
            Ok((None, None))
        }
        ("subplots", [nrows, ncols]) => {
            let fig = numbered_figure(kwarg("num"), kwarg("clear"))?;
            let count = |v: &Json| {
                v.as_i64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad grid {v}")))
            };
            fig.subplots(count(nrows)?, count(ncols)?)
                .map_err(ProbeError::rejected)?;
            figure(fig)
        }
        ("suptitle", [Json::String(text)]) => {
            pyplot::gcf().suptitle(text.as_str());
            Ok((None, None))
        }
        ("text", [x, y, Json::String(text)]) => {
            let coordinate = |v: &Json| {
                v.as_f64()
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad coordinate {v}")))
            };
            pyplot::gcf().text(coordinate(x)?, coordinate(y)?, text.as_str());
            Ok((None, None))
        }
        ("get_fignums", []) => value(numbers(pyplot::get_fignums().into_iter().map(f64::from))),
        ("get_figlabels", []) => value(Json::from(pyplot::get_figlabels())),
        ("fignum_exists", [num]) => value(Json::Bool(match Num::parse(num)? {
            Num::Number(n) => pyplot::fignum_exists(n),
            Num::Label(label) => pyplot::figlabel_exists(label),
        })),
        ("gcf_is", [index]) => value(Json::Bool(pyplot::gcf() == made_figure(made, index)?)),
        ("texts", []) => value(Json::from(
            pyplot::gcf()
                .texts()
                .iter()
                .map(Text::text)
                .collect::<Vec<_>>(),
        )),
        ("get_size_inches", []) => {
            let (width, height) = pyplot::gcf().size_inches();
            value(numbers([width, height]))
        }
        _ => Err(ProbeError::BadArgs(format!("unknown step {step}"))),
    }
}

/// The figure step `index` made.
fn made_figure(made: &[Option<Figure>], index: &Json) -> Result<Figure, ProbeError> {
    index
        .as_i64()
        .and_then(|i| usize::try_from(i).ok())
        .and_then(|i| made.get(i).cloned().flatten())
        .ok_or_else(|| ProbeError::BadArgs(format!("step {index} made no figure")))
}

/// `plt.figure(num, clear=clear)` (a new figure without `num`).
fn numbered_figure(num: Option<&Json>, clear: Option<&Json>) -> Result<Figure, ProbeError> {
    let fig = match num.map(Num::parse).transpose()? {
        None => pyplot::figure(),
        Some(Num::Number(n)) => pyplot::figure_num(n),
        Some(Num::Label(label)) => pyplot::figure_label(label),
    };
    if clear.and_then(Json::as_bool) == Some(true) {
        fig.clear();
    }
    Ok(fig)
}

/// `plt.figure(figsize=figsize, dpi=dpi)`: `[width, height]` in inches or
/// `[width, height, unit]`, either side `null`.
fn sized_figure(figsize: &Json, dpi: Option<&Json>) -> Result<Figure, ProbeError> {
    let parts = figsize.as_array().unwrap_or(&[]);
    let (sides, unit) = match parts {
        [width, height] => ([width, height], Ok(SizeUnit::Inches)),
        [width, height, Json::String(unit)] => ([width, height], unit.parse()),
        _ => return Err(ProbeError::BadArgs(format!("bad figsize {figsize}"))),
    };
    let unit = unit.map_err(ProbeError::rejected)?;
    let side = |v: &Json| (!v.is_null()).then(|| v.as_f64()).flatten();
    let size = FigSize::new(side(sides[0]), side(sides[1]), unit);
    let dpi = dpi
        .and_then(Json::as_f64)
        .unwrap_or_else(|| pltrs::rc::params().figure_dpi);
    pyplot::figure_with_figsize(&size, dpi).map_err(ProbeError::rejected)
}

/// `plt.figaspect(arg)` of a ratio or of an array of `shape`.
pub(super) fn figaspect(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let (width, height) = match args.opt_f64("arg")? {
            Some(ratio) => pyplot::figaspect(ratio),
            None => match args.f64_vec("shape")?.as_slice() {
                [rows, cols] => pyplot::figaspect_of_shape(*rows as usize, *cols as usize),
                _ => return Err(ProbeError::BadArgs("shape needs two sides".into())),
            },
        };
        Ok(numbers([width, height]))
    })
}
