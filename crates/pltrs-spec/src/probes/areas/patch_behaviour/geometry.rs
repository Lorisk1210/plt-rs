//! The `patch.geometry` op: standalone patch shapes, mutated by setters
//! and queried for their corners, axis ends, outline and hit tests.

use pltrs::patches::{
    Arrow, Circle, Ellipse, FancyArrow, PatchShape, Polygon, Rectangle, RotationPoint, Wedge,
};
use pltrs_core::path::PolygonOptions;
use pltrs_core::{Path, Point};

use crate::json::Json;
use crate::probes::encode::{bbox, parse_points, path, point, points};
use crate::probes::{Args, Outcome, ProbeError};

/// One of the shapes the op builds.
enum Shape {
    Rectangle(Rectangle),
    Ellipse(Ellipse),
    Circle(Circle),
    Polygon(Polygon),
    Wedge(Wedge),
    Arrow(Arrow),
    FancyArrow(FancyArrow),
}

/// A required number of an argument object.
fn number(object: &Json, key: &str) -> Result<f64, ProbeError> {
    object
        .get(key)
        .and_then(Json::as_f64)
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a number in {object}")))
}

fn optional(object: &Json, key: &str) -> Result<Option<f64>, ProbeError> {
    match object.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a number in {object}"))),
    }
}

fn xy(object: &Json, key: &str) -> Result<Point, ProbeError> {
    match object.get(key).and_then(Json::as_f64_vec).as_deref() {
        Some(&[x, y]) => Ok(Point::new(x, y)),
        _ => Err(ProbeError::BadArgs(format!(
            "{key} is not a point in {object}"
        ))),
    }
}

/// `Polygon(np.asarray(xy).reshape(-1, 2), closed)`.
fn polygon(value: &Json, closed: bool) -> Result<Polygon, ProbeError> {
    let vertices = if value.as_array().is_some_and(<[Json]>::is_empty) {
        Vec::new()
    } else {
        parse_points(value)?
    };
    Ok(Polygon::new(vertices, closed))
}

fn rotation_point(value: Option<&Json>) -> Result<RotationPoint, ProbeError> {
    match value {
        None => Ok(RotationPoint::Xy),
        Some(Json::String(name)) if name == "xy" => Ok(RotationPoint::Xy),
        Some(Json::String(name)) if name == "center" => Ok(RotationPoint::Center),
        Some(other) => match other.as_f64_vec().as_deref() {
            Some(&[x, y]) => Ok(RotationPoint::At(Point::new(x, y))),
            _ => Err(ProbeError::BadArgs(format!("bad rotation point {other}"))),
        },
    }
}

impl Shape {
    fn parse(spec: &Json) -> Result<Self, ProbeError> {
        let kind = spec.get("kind").and_then(Json::as_str).unwrap_or_default();
        Ok(match kind {
            "rectangle" => {
                let rect = Rectangle::new(
                    xy(spec, "xy")?,
                    number(spec, "width")?,
                    number(spec, "height")?,
                );
                Self::Rectangle(rect.rotated(
                    optional(spec, "angle")?.unwrap_or(0.0),
                    rotation_point(spec.get("rotation_point"))?,
                ))
            }
            "ellipse" => Self::Ellipse(Ellipse {
                angle: optional(spec, "angle")?.unwrap_or(0.0),
                ..Ellipse::new(
                    xy(spec, "xy")?,
                    number(spec, "width")?,
                    number(spec, "height")?,
                )
            }),
            "circle" => Self::Circle(Circle::new(xy(spec, "xy")?, number(spec, "radius")?)),
            "polygon" => {
                let closed = spec.get("closed").and_then(Json::as_bool).unwrap_or(true);
                Self::Polygon(polygon(spec.get("xy").unwrap_or(&Json::Null), closed)?)
            }
            "wedge" => {
                let center = xy(spec, "center")?;
                Self::Wedge(Wedge {
                    width: optional(spec, "width")?,
                    ..Wedge::new(
                        center,
                        number(spec, "r")?,
                        number(spec, "theta1")?,
                        number(spec, "theta2")?,
                    )
                })
            }
            "arrow" => Self::Arrow(
                Arrow::new(
                    number(spec, "x")?,
                    number(spec, "y")?,
                    number(spec, "dx")?,
                    number(spec, "dy")?,
                )
                .with_width(optional(spec, "width")?.unwrap_or(1.0)),
            ),
            "fancy_arrow" => Self::FancyArrow(fancy_arrow(spec)?),
            other => return Err(ProbeError::BadArgs(format!("unknown shape {other:?}"))),
        })
    }

    /// The outline mapped by the patch transform (the patch's own
    /// coordinates, as for a patch outside any axes).
    fn placed_path(&self) -> Path {
        match self {
            Self::Rectangle(s) => s.placed_path(),
            Self::Ellipse(s) => s.placed_path(),
            Self::Circle(s) => s.placed_path(),
            Self::Polygon(s) => s.placed_path(),
            Self::Wedge(s) => s.placed_path(),
            Self::Arrow(s) => s.placed_path(),
            Self::FancyArrow(s) => s.placed_path(),
        }
    }

    fn apply(&mut self, name: &str, value: &Json) -> Result<(), ProbeError> {
        let wrong = || ProbeError::BadArgs(format!("{name} does not apply"));
        let amount = || {
            value
                .as_f64()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad {name} {value}")))
        };
        match (self, name) {
            (Self::Rectangle(rect), "set_angle") => rect.angle = amount()?,
            (Self::Ellipse(ellipse), "set_angle") => ellipse.angle = amount()?,
            (Self::Polygon(poly), "set_xy") => *poly = polygon(value, poly.is_closed())?,
            (Self::Wedge(wedge), "set_center") => {
                wedge.center = parse_points(&Json::Array(vec![value.clone()]))?[0];
            }
            (Self::Wedge(wedge), "set_radius") => wedge.r = amount()?,
            (Self::Wedge(wedge), "set_width") => wedge.width = Some(amount()?),
            (Self::Wedge(wedge), "set_theta1") => wedge.theta1 = amount()?,
            (Self::Wedge(wedge), "set_theta2") => wedge.theta2 = amount()?,
            (Self::Arrow(arrow), "set_data") => {
                let data = ArrowData::parse(value)?;
                data.update(&mut arrow.tail, &mut arrow.delta);
                arrow.width = data.width.unwrap_or(arrow.width);
            }
            (Self::FancyArrow(arrow), "set_data") => {
                let data = ArrowData::parse(value)?;
                data.update(&mut arrow.tail, &mut arrow.delta);
                arrow.width = data.width.unwrap_or(arrow.width);
                arrow.head_width = data.head_width.or(arrow.head_width);
                arrow.head_length = data.head_length.or(arrow.head_length);
            }
            _ => return Err(wrong()),
        }
        Ok(())
    }

    fn query(&self, query: &Json) -> Result<(String, Json), ProbeError> {
        if let Some([name, probe_points]) = query.as_array().map(<[Json]>::as_ref)
            && name.as_str() == Some("contains")
        {
            let outline = self.placed_path();
            let hits = parse_points(probe_points)?
                .into_iter()
                .map(|p| Json::Bool(outline.contains_point(p, None, 0.0)))
                .collect();
            return Ok(("contains".to_owned(), Json::Array(hits)));
        }
        let name = query
            .as_str()
            .ok_or_else(|| ProbeError::BadArgs(format!("bad query {query}")))?;
        let value = match (self, name) {
            (Self::Rectangle(rect), "corners") => points(&rect.corners()),
            (Self::Ellipse(ellipse), "corners") => points(&ellipse.corners()),
            (Self::Rectangle(rect), "center") => point(rect.center()),
            (Self::Ellipse(ellipse), "center") => point(ellipse.center),
            (Self::Ellipse(ellipse), "vertices") => points(&ellipse.vertices()),
            (Self::Ellipse(ellipse), "co_vertices") => points(&ellipse.co_vertices()),
            (Self::Polygon(poly), "xy") => points(poly.vertices()),
            (_, "verts") => {
                let polygons = self.placed_path().to_polygons(&PolygonOptions::default());
                points(polygons.first().map_or(&[], Vec::as_slice))
            }
            (_, "extents") => bbox(&self.placed_path().extents()),
            (_, "path") => path(&self.placed_path()),
            _ => {
                return Err(ProbeError::BadArgs(format!(
                    "query {name:?} does not apply"
                )));
            }
        };
        Ok((name.to_owned(), value))
    }
}

fn fancy_arrow(spec: &Json) -> Result<FancyArrow, ProbeError> {
    let shape = match spec.get("shape").and_then(Json::as_str).unwrap_or("full") {
        "full" => pltrs::patches::ArrowShape::Full,
        "left" => pltrs::patches::ArrowShape::Left,
        "right" => pltrs::patches::ArrowShape::Right,
        other => {
            return Err(ProbeError::BadArgs(format!(
                "unknown arrow shape {other:?}"
            )));
        }
    };
    Ok(FancyArrow {
        width: number(spec, "width")?,
        head_width: optional(spec, "head_width")?,
        head_length: optional(spec, "head_length")?,
        shape,
        overhang: optional(spec, "overhang")?.unwrap_or(0.0),
        ..FancyArrow::new(
            number(spec, "x")?,
            number(spec, "y")?,
            number(spec, "dx")?,
            number(spec, "dy")?,
        )
    })
}

/// The keyword arguments of `Arrow.set_data` / `FancyArrow.set_data`;
/// absent ones keep their values.
struct ArrowData {
    x: Option<f64>,
    y: Option<f64>,
    dx: Option<f64>,
    dy: Option<f64>,
    width: Option<f64>,
    head_width: Option<f64>,
    head_length: Option<f64>,
}

impl ArrowData {
    fn parse(value: &Json) -> Result<Self, ProbeError> {
        Ok(Self {
            x: optional(value, "x")?,
            y: optional(value, "y")?,
            dx: optional(value, "dx")?,
            dy: optional(value, "dy")?,
            width: optional(value, "width")?,
            head_width: optional(value, "head_width")?,
            head_length: optional(value, "head_length")?,
        })
    }

    fn update(&self, tail: &mut Point, delta: &mut Point) {
        *tail = Point::new(self.x.unwrap_or(tail.x), self.y.unwrap_or(tail.y));
        *delta = Point::new(self.dx.unwrap_or(delta.x), self.dy.unwrap_or(delta.y));
    }
}

pub(super) fn geometry(args: Args<'_>) -> Outcome {
    let mut shape = Shape::parse(args.json("shape")?)?;
    for step in args.json("steps")?.as_array().unwrap_or(&[]) {
        match step.as_array().map(<[Json]>::as_ref) {
            Some([Json::String(name), value]) => shape.apply(name, value)?,
            _ => return Err(ProbeError::BadArgs(format!("bad step {step}"))),
        }
    }
    let answers = args
        .json("queries")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|query| shape.query(query))
        .collect::<Result<_, _>>()?;
    Ok(Json::Object(answers))
}
