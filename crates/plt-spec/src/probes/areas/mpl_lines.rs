//! Probe operations for the `mpl_lines` area: matplotlib's own non-image
//! tests of `Line2D` that the `line_behaviour` and `markers` areas leave
//! out (see `spec/reference/probes/test_mpl_lines.py`).

use plt::figure::Figure;
use plt::lines::{AxLineThrough, Line2D, LineProps, MarkEvery};
use plt::markers::{FillStyle, Marker};
use plt::transforms::Point;

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("AxLine.set", axline_set),
    Op::new("Line2D.get_markerfacecolor", markerfacecolor),
    Op::new("lines._mark_every_path", mark_every_path),
];

/// A JSON `[x, y]` as a point.
fn point(value: &Json) -> Result<Point, ProbeError> {
    match value.as_f64_vec().as_deref() {
        Some(&[x, y]) => Ok(Point::new(x, y)),
        _ => Err(ProbeError::BadArgs(format!("not a point: {value}"))),
    }
}

/// `{xy1, xy2, slope}` as an infinite line's getters return them.
fn axline_state(line: &Line2D) -> Result<Json, ProbeError> {
    let xy = |p: Point| numbers([p.x, p.y]);
    let xy1 = line
        .axline_xy1()
        .ok_or_else(|| ProbeError::rejected("not an infinite line"))?;
    Ok(Json::Object(vec![
        ("xy1".into(), xy(xy1)),
        ("xy2".into(), line.axline_xy2().map_or(Json::Null, xy)),
        (
            "slope".into(),
            line.axline_slope().map_or(Json::Null, Json::from),
        ),
    ]))
}

/// The state of an infinite line after each setter call of `steps`
/// (`test_axline_setters`).
fn axline_set(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        let through = match args.str("through")? {
            "slope" => AxLineThrough::Slope(0.6),
            _ => AxLineThrough::Point(Point::new(0.8, 0.4)),
        };
        let line = ax
            .axline(Point::new(0.1, 0.1), through, LineProps::default())
            .map_err(ProbeError::rejected)?;
        let mut out = vec![axline_state(&line)?];
        for step in args.json("steps")?.as_array().unwrap_or(&[]) {
            let (name, value) = match step.as_array() {
                Some([Json::String(name), value]) => (name.as_str(), value),
                _ => return Err(ProbeError::BadArgs(format!("bad step {step}"))),
            };
            match name {
                "xy1" => line.set_axline_xy1(point(value)?),
                "xy2" => line.set_axline_xy2(point(value)?),
                "slope" => {
                    let slope = value
                        .as_f64()
                        .ok_or_else(|| ProbeError::BadArgs(format!("bad slope {value}")))?;
                    line.set_axline_slope(slope)
                }
                other => return Err(ProbeError::BadArgs(format!("unknown setter {other}"))),
            }
            .map_err(ProbeError::rejected)?;
            out.push(axline_state(&line)?);
        }
        Ok(Json::Array(out))
    })
}

/// `[fillstyle, facecolor]` of a line whose hollow markers are given a
/// face colour (`test_markerfacecolor_fillstyle`).
fn markerfacecolor(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let line = Figure::new().subplot().plot_y([1.0, 3.0, 2.0]);
        line.set_marker("o".parse::<Marker>().map_err(ProbeError::rejected)?)
            .set_fillstyle(FillStyle::None)
            .set_markerfacecolor("red");
        let face = line
            .effective_markerfacecolor()
            .resolve()
            .map_err(ProbeError::rejected)?;
        Ok(Json::Array(vec![
            Json::from(line.fillstyle().name()),
            numbers([face.r, face.g, face.b, face.a]),
        ]))
    })
}

/// The x of the vertices `(x, 0)`, scaled by the axes diagonal, that a
/// float `markevery` picks.
fn mark_every_path(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let (start, step) = match args.json("markevery")? {
            Json::Array(_) => match args.f64_vec("markevery")?.as_slice() {
                &[start, step] => (start, step),
                _ => return Err(ProbeError::BadArgs("markevery is not a pair".into())),
            },
            _ => (0.0, args.f64("markevery")?),
        };
        let fig = Figure::new();
        let ax = fig.subplot();
        let axes = fig
            .resolve_transform(&ax.trans_axes())
            .map_err(ProbeError::rejected)?;
        let corner = |at: f64| axes.transform_point(Point::new(at, at));
        let (low, high) = (corner(0.0), corner(1.0));
        let diagonal = (high.x - low.x).hypot(high.y - low.y);
        let x = args.f64_vec("x")?;
        let scaled: Vec<Point> = x.iter().map(|&x| Point::new(x * diagonal, 0.0)).collect();
        let picked = MarkEvery::Distance { start, step }
            .select(&scaled, Some(diagonal))
            .map_err(ProbeError::rejected)?;
        Ok(numbers(picked.into_iter().map(|i| x[i])))
    })
}
