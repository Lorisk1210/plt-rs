//! Named page resources registered while drawing and written when the
//! file is finished: alpha graphics states, soft masks, hatch patterns,
//! Gouraud shadings, images, markers and path-collection templates
//! (matplotlib's `PdfFile.alphaState`, `hatchPattern`,
//! `addGouraudTriangles`, `imageObject`, `markerObject`,
//! `pathCollectionObject` and their `write*` counterparts).

use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};

use super::object::Obj;
use super::writer::PdfWriter;
use super::{image, path_ops};
use crate::gc::{CapStyle, JoinStyle};
use crate::image::RgbaImage;
use crate::vector::VectorError;

/// matplotlib's PDF codes for cap styles.
pub(super) const fn cap_code(cap: CapStyle) -> i64 {
    match cap {
        CapStyle::Butt => 0,
        CapStyle::Round => 1,
        CapStyle::Projecting => 2,
    }
}

/// matplotlib's PDF codes for join styles.
pub(super) const fn join_code(join: JoinStyle) -> i64 {
    match join {
        JoinStyle::Miter => 0,
        JoinStyle::Round => 1,
        JoinStyle::Bevel => 2,
    }
}

/// `Op.paint_path`: the operator painting a path.
pub(super) const fn paint_op(fill: bool, stroke: bool) -> &'static str {
    match (fill, stroke) {
        (true, true) => "B",
        (false, true) => "S",
        (true, false) => "f",
        (false, false) => "n",
    }
}

/// A hatch pattern's colours, pattern and line width.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HatchStyle {
    pub(super) edge: Option<Rgba>,
    pub(super) face: Option<Rgba>,
    pub(super) pattern: String,
    pub(super) linewidth: f64,
}

/// Vertex colours of a Gouraud shading (`addGouraudTriangles`): RGB, or
/// one grey level (alpha values for a soft mask).
#[derive(Clone, Debug, PartialEq)]
pub enum ShadingColors {
    /// The colours of the three vertices of every triangle (their alpha
    /// is not written).
    Rgb(Vec<[Rgba; 3]>),
    /// The grey levels of the three vertices of every triangle.
    Gray(Vec<[f64; 3]>),
}

/// One registered shading.
#[derive(Clone, Debug)]
struct Shading {
    name: String,
    id: u32,
    points: Vec<[Point; 3]>,
    colors: ShadingColors,
}

/// How a marker form paints its path, and the line width it is drawn
/// with (the arguments of `markerObject`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerPaint {
    /// Whether the path is filled.
    pub fill: bool,
    /// Whether the path is stroked.
    pub stroke: bool,
    /// The line join.
    pub join: JoinStyle,
    /// The line cap.
    pub cap: CapStyle,
    /// The widest line the marker is drawn with, which pads its box.
    pub linewidth: f64,
}

/// One marker form.
#[derive(Clone, Debug)]
struct Marker {
    ops: Vec<u8>,
    fill: bool,
    stroke: bool,
    join: JoinStyle,
    cap: CapStyle,
    name: String,
    id: u32,
    bbox: Bbox,
    linewidth: f64,
}

/// How the paths of a collection are painted (the arguments of
/// `pathCollectionObject`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemplatePaint {
    /// The line join.
    pub join: JoinStyle,
    /// The line cap.
    pub cap: CapStyle,
    /// Extra room around the path for the widest edge.
    pub padding: f64,
    /// Whether the paths are filled.
    pub filled: bool,
    /// Whether the paths are stroked.
    pub stroked: bool,
}

/// One path-collection template.
#[derive(Clone, Debug)]
struct PathTemplate {
    name: String,
    path: Path,
    transform: Affine2D,
    id: u32,
    paint: TemplatePaint,
}

/// The object numbers of the shared resource dictionaries.
#[derive(Clone, Copy, Debug)]
pub(super) struct ResourceIds {
    pub(super) fonts: u32,
    pub(super) ext_gstates: u32,
    pub(super) patterns: u32,
    pub(super) shadings: u32,
    pub(super) xobjects: u32,
}

/// Every resource registered so far, in registration order.
#[derive(Debug, Default)]
pub(super) struct Resources {
    alpha_states: Vec<((f64, f64), String)>,
    soft_masks: Vec<(u32, String, u32)>,
    hatches: Vec<(HatchStyle, String)>,
    shadings: Vec<Shading>,
    images: Vec<(RgbaImage, String, u32)>,
    markers: Vec<Marker>,
    paths: Vec<PathTemplate>,
}

/// The bounding box of `path` under `transform` (`Path.get_extents`).
pub(super) fn extents(path: &Path, transform: Affine2D) -> Bbox {
    path.transformed(&transform).extents()
}

impl Resources {
    /// `alphaState`: the graphics state setting stroke and fill alpha.
    pub(super) fn alpha_state(&mut self, alphas: (f64, f64)) -> String {
        if let Some((_, name)) = self.alpha_states.iter().find(|(a, _)| *a == alphas) {
            return name.clone();
        }
        let name = format!("A{}", self.alpha_states.len() + 1);
        self.alpha_states.push((alphas, name.clone()));
        name
    }

    /// `_soft_mask_state`: a graphics state masking with the luminosity of
    /// shading `smask`.
    pub(super) fn soft_mask_state(&mut self, writer: &mut PdfWriter, smask: u32) -> String {
        if let Some((_, name, _)) = self.soft_masks.iter().find(|(s, _, _)| *s == smask) {
            return name.clone();
        }
        let name = format!("SM{}", self.soft_masks.len() + 1);
        let group = writer.reserve();
        self.soft_masks.push((smask, name.clone(), group));
        name
    }

    /// `hatchPattern`.
    pub(super) fn hatch_pattern(&mut self, style: HatchStyle) -> String {
        if let Some((_, name)) = self.hatches.iter().find(|(s, _)| *s == style) {
            return name.clone();
        }
        let name = format!("H{}", self.hatches.len() + 1);
        self.hatches.push((style, name.clone()));
        name
    }

    /// The hatch patterns made so far with their names, in order.
    pub(super) fn hatch_styles(&self) -> impl Iterator<Item = (&HatchStyle, &str)> {
        self.hatches
            .iter()
            .map(|(style, name)| (style, name.as_str()))
    }

    /// `addGouraudTriangles`: a new shading of `points` (display
    /// coordinates); returns its name and object.
    pub(super) fn gouraud(
        &mut self,
        writer: &mut PdfWriter,
        points: Vec<[Point; 3]>,
        colors: ShadingColors,
    ) -> (String, u32) {
        let name = format!("GT{}", self.shadings.len());
        let id = writer.reserve();
        self.shadings.push(Shading {
            name: name.clone(),
            id,
            points,
            colors,
        });
        (name, id)
    }

    /// The names of the image `XObject`s registered so far, in order.
    pub(super) fn image_names(&self) -> impl Iterator<Item = &str> {
        self.images.iter().map(|(_, name, _)| name.as_str())
    }

    /// `imageObject`: every drawn image is a new `XObject`.
    pub(super) fn image(&mut self, writer: &mut PdfWriter, image: &RgbaImage) -> String {
        let name = format!("I{}", self.images.len() + 1);
        let id = writer.reserve();
        self.images.push((image.clone(), name.clone(), id));
        name
    }

    /// `markerObject`: a new form `XObject` per call, its box padded for
    /// the line drawing it. matplotlib keys its markers by their path
    /// operations, which compare by identity (`Verbatim`), so markers drawn
    /// alike by different calls do not share a form.
    pub(super) fn marker(
        &mut self,
        writer: &mut PdfWriter,
        path: &Path,
        transform: Affine2D,
        paint: MarkerPaint,
    ) -> String {
        let ops = path_ops(path, transform, None, None, None);
        let MarkerPaint {
            fill,
            stroke,
            join,
            cap,
            linewidth,
        } = paint;
        let name = format!("M{}", self.markers.len());
        let id = writer.reserve();
        self.markers.push(Marker {
            ops,
            fill,
            stroke,
            join,
            cap,
            name: name.clone(),
            id,
            bbox: extents(path, transform),
            linewidth,
        });
        name
    }

    /// `pathCollectionObject`.
    pub(super) fn path_template(
        &mut self,
        writer: &mut PdfWriter,
        path: &Path,
        transform: Affine2D,
        paint: TemplatePaint,
    ) -> String {
        let name = format!("P{}", self.paths.len());
        self.paths.push(PathTemplate {
            name: name.clone(),
            path: path.clone(),
            transform,
            id: writer.reserve(),
            paint,
        });
        name
    }

    /// Writes every resource and the resource dictionaries (after the
    /// fonts, which the caller writes).
    pub(super) fn write(
        &self,
        writer: &mut PdfWriter,
        ids: ResourceIds,
        page_height: f64,
        compressed: bool,
    ) -> Result<(), VectorError> {
        self.write_ext_gstates(writer, ids.ext_gstates)?;
        self.write_soft_mask_groups(writer)?;
        self.write_hatches(writer, ids.patterns, page_height)?;
        self.write_shadings(writer, ids.shadings)?;
        self.write_xobjects(writer, ids.xobjects)?;
        self.write_images(writer, compressed)?;
        self.write_markers(writer)?;
        self.write_path_templates(writer)
    }

    /// `writeExtGSTates`: the dictionary of the alpha and soft mask
    /// graphics states, as object `dictionary`.
    pub(super) fn write_ext_gstates(
        &self,
        writer: &mut PdfWriter,
        dictionary: u32,
    ) -> Result<(), VectorError> {
        let mut states: Vec<(String, Obj)> = self
            .alpha_states
            .iter()
            .map(|((stroke, fill), name)| {
                let state = Obj::dict([
                    ("Type", Obj::name("ExtGState")),
                    ("CA", Obj::Real(*stroke)),
                    ("ca", Obj::Real(*fill)),
                ]);
                (name.clone(), state)
            })
            .collect();
        for (_, name, group) in &self.soft_masks {
            let mask = Obj::dict([
                ("Type", Obj::name("Mask")),
                ("S", Obj::name("Luminosity")),
                ("BC", Obj::Array(vec![Obj::Int(1)])),
                ("G", Obj::Ref(*group)),
            ]);
            let state = Obj::dict([
                ("Type", Obj::name("ExtGState")),
                ("AIS", Obj::Bool(false)),
                ("SMask", mask),
            ]);
            states.push((name.clone(), state));
        }
        writer.write_object(dictionary, &Obj::Dict(states))
    }

    /// `_write_soft_mask_groups`: the transparency groups of the soft
    /// masks.
    pub(super) fn write_soft_mask_groups(&self, writer: &mut PdfWriter) -> Result<(), VectorError> {
        for (smask, _, group) in &self.soft_masks {
            let form = vec![
                ("Type".to_owned(), Obj::name("XObject")),
                ("Subtype".to_owned(), Obj::name("Form")),
                ("FormType".to_owned(), Obj::Int(1)),
                (
                    "Group".to_owned(),
                    Obj::dict([
                        ("S", Obj::name("Transparency")),
                        ("CS", Obj::name("DeviceGray")),
                    ]),
                ),
                (
                    "Matrix".to_owned(),
                    Obj::reals([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
                ),
                (
                    "Resources".to_owned(),
                    Obj::dict([("Shading", Obj::dict([("S", Obj::Ref(*smask))]))]),
                ),
                ("BBox".to_owned(), Obj::reals([0.0, 0.0, 1.0, 1.0])),
            ];
            writer.begin_stream(*group, None, form, None);
            writer.output(&[Obj::name("S"), Obj::op("sh")])?;
            writer.end_stream()?;
        }
        Ok(())
    }

    /// The dictionary of the images, marker forms and path templates, as
    /// object `dictionary`.
    pub(super) fn write_xobjects(
        &self,
        writer: &mut PdfWriter,
        dictionary: u32,
    ) -> Result<(), VectorError> {
        let mut xobjects: Vec<(String, Obj)> = Vec::new();
        xobjects.extend(
            self.images
                .iter()
                .map(|(_, n, id)| (n.clone(), Obj::Ref(*id))),
        );
        xobjects.extend(
            self.markers
                .iter()
                .map(|m| (m.name.clone(), Obj::Ref(m.id))),
        );
        xobjects.extend(self.paths.iter().map(|p| (p.name.clone(), Obj::Ref(p.id))));
        writer.write_object(dictionary, &Obj::Dict(xobjects))
    }

    /// `writeImages`: every image `XObject`, its streams compressed with
    /// `compressed`.
    pub(super) fn write_images(
        &self,
        writer: &mut PdfWriter,
        compressed: bool,
    ) -> Result<(), VectorError> {
        for (image, _, id) in &self.images {
            image::write_image(writer, image, *id, compressed)?;
        }
        Ok(())
    }

    /// `writeHatches`: 72-point tiles in the page's coordinate origin.
    pub(super) fn write_hatches(
        &self,
        writer: &mut PdfWriter,
        dictionary: u32,
        page_height: f64,
    ) -> Result<(), VectorError> {
        const SIDE: f64 = 72.0;
        let mut patterns = Vec::new();
        for (style, name) in &self.hatches {
            let id = writer.reserve();
            patterns.push((name.clone(), Obj::Ref(id)));
            let procsets = ["PDF", "Text", "ImageB", "ImageC", "ImageI"]
                .map(Obj::name)
                .to_vec();
            let dict = vec![
                ("Type".to_owned(), Obj::name("Pattern")),
                ("PatternType".to_owned(), Obj::Int(1)),
                ("PaintType".to_owned(), Obj::Int(1)),
                ("TilingType".to_owned(), Obj::Int(1)),
                ("BBox".to_owned(), Obj::reals([0.0, 0.0, SIDE, SIDE])),
                ("XStep".to_owned(), Obj::Real(SIDE)),
                ("YStep".to_owned(), Obj::Real(SIDE)),
                (
                    "Resources".to_owned(),
                    Obj::dict([("Procsets", Obj::Array(procsets))]),
                ),
                (
                    "Matrix".to_owned(),
                    Obj::reals([1.0, 0.0, 0.0, 1.0, 0.0, page_height * 72.0]),
                ),
            ];
            writer.begin_stream(id, None, dict, None);
            let edge = style.edge.unwrap_or(Rgba::BLACK);
            let rgb = |c: Rgba| [Obj::Real(c.r), Obj::Real(c.g), Obj::Real(c.b)];
            writer.output(&[rgb(edge).as_slice(), &[Obj::op("RG")]].concat())?;
            if let Some(face) = style.face {
                let square = [
                    Obj::op("rg"),
                    Obj::Int(0),
                    Obj::Int(0),
                    Obj::Real(SIDE),
                    Obj::Real(SIDE),
                    Obj::op("re"),
                    Obj::op("f"),
                ];
                writer.output(&[rgb(face).as_slice(), &square].concat())?;
            }
            writer.output(&[rgb(edge).as_slice(), &[Obj::op("rg")]].concat())?;
            writer.output(&[Obj::Real(style.linewidth), Obj::op("w")])?;
            let tile =
                pltrs_core::hatch::pattern_path(&style.pattern, pltrs_core::hatch::DEFAULT_DENSITY)
                    .unwrap_or_default();
            let ops = path_ops(&tile, Affine2D::scaling(SIDE, SIDE), None, None, None);
            writer.output(&[Obj::Raw(ops)])?;
            writer.output(&[Obj::op("B")])?;
            writer.end_stream()?;
        }
        writer.write_object(dictionary, &Obj::Dict(patterns))
    }

    /// `writeGouraudTriangles`: free-form triangle meshes (shading type 4)
    /// with 32-bit coordinates and 8-bit colours.
    pub(super) fn write_shadings(
        &self,
        writer: &mut PdfWriter,
        dictionary: u32,
    ) -> Result<(), VectorError> {
        let mut entries = Vec::new();
        for shading in &self.shadings {
            entries.push((shading.name.clone(), Obj::Ref(shading.id)));
            let points: Vec<Point> = shading.points.iter().flatten().copied().collect();
            let min = points
                .iter()
                .fold(Point::new(f64::INFINITY, f64::INFINITY), |a, &p| a.min(p));
            let max = points
                .iter()
                .fold(Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY), |a, &p| {
                    a.max(p)
                });
            let (min, max) = (
                Point::new(min.x - 256.0, min.y - 256.0),
                Point::new(max.x + 256.0, max.y + 256.0),
            );
            let factor = Point::new(
                f64::from(u32::MAX) / (max.x - min.x),
                f64::from(u32::MAX) / (max.y - min.y),
            );
            let colors: Vec<Vec<f64>> = match &shading.colors {
                ShadingColors::Rgb(colors) => colors
                    .iter()
                    .flatten()
                    .map(|c| vec![c.r, c.g, c.b])
                    .collect(),
                ShadingColors::Gray(levels) => levels.iter().flatten().map(|&g| vec![g]).collect(),
            };
            let channels = colors.first().map_or(3, Vec::len);
            let mut decode = vec![min.x, max.x, min.y, max.y];
            for _ in 0..channels {
                decode.extend([0.0, 1.0]);
            }
            let space = if channels == 3 {
                "DeviceRGB"
            } else {
                "DeviceGray"
            };
            let dict = vec![
                ("ShadingType".to_owned(), Obj::Int(4)),
                ("BitsPerCoordinate".to_owned(), Obj::Int(32)),
                ("BitsPerComponent".to_owned(), Obj::Int(8)),
                ("BitsPerFlag".to_owned(), Obj::Int(8)),
                ("ColorSpace".to_owned(), Obj::name(space)),
                ("AntiAlias".to_owned(), Obj::Bool(false)),
                ("Decode".to_owned(), Obj::reals(decode)),
            ];
            let mut data = Vec::with_capacity(points.len() * (9 + channels));
            for (point, color) in points.iter().zip(&colors) {
                data.push(0);
                data.extend_from_slice(&(((point.x - min.x) * factor.x) as u32).to_be_bytes());
                data.extend_from_slice(&(((point.y - min.y) * factor.y) as u32).to_be_bytes());
                data.extend(color.iter().map(|&c| (c * 255.0) as u8));
            }
            writer.begin_stream(shading.id, None, dict, None);
            writer.write(&data);
            writer.end_stream()?;
        }
        writer.write_object(dictionary, &Obj::Dict(entries))
    }

    /// `writeMarkers`: the form `XObject` of every marker, setting join and
    /// cap, then painting its path.
    pub(super) fn write_markers(&self, writer: &mut PdfWriter) -> Result<(), VectorError> {
        for marker in &self.markers {
            // Half a line, plus a miter of up to ten widths, may stick out.
            let bbox = marker
                .bbox
                .padded(marker.linewidth * 5.0, marker.linewidth * 5.0);
            let (x0, y0, x1, y1) = bbox.extents();
            write_form(
                writer,
                marker.id,
                [x0, y0, x1, y1],
                (marker.join, marker.cap),
                &marker.ops,
                paint_op(marker.fill, marker.stroke),
            )?;
        }
        Ok(())
    }

    /// `writePathCollectionTemplates`: the form `XObject` of every path
    /// of a collection drawn as a template.
    pub(super) fn write_path_templates(&self, writer: &mut PdfWriter) -> Result<(), VectorError> {
        for template in &self.paths {
            let ops = path_ops(&template.path, template.transform, None, None, None);
            let bbox = extents(&template.path, template.transform);
            let paint = template.paint;
            let extents = if bbox.is_finite() {
                let (x0, y0, x1, y1) = bbox.padded(paint.padding, paint.padding).extents();
                [x0, y0, x1, y1]
            } else {
                [0.0; 4]
            };
            write_form(
                writer,
                template.id,
                extents,
                (paint.join, paint.cap),
                &ops,
                paint_op(paint.filled, paint.stroked),
            )?;
        }
        Ok(())
    }
}

/// One form `XObject` painting `ops`.
fn write_form(
    writer: &mut PdfWriter,
    id: u32,
    bbox: [f64; 4],
    (join, cap): (JoinStyle, CapStyle),
    ops: &[u8],
    paint: &str,
) -> Result<(), VectorError> {
    let dict = vec![
        ("Type".to_owned(), Obj::name("XObject")),
        ("Subtype".to_owned(), Obj::name("Form")),
        ("BBox".to_owned(), Obj::reals(bbox)),
    ];
    writer.begin_stream(id, None, dict, None);
    writer.output(&[Obj::Int(join_code(join)), Obj::op("j")])?;
    writer.output(&[Obj::Int(cap_code(cap)), Obj::op("J")])?;
    writer.output(&[Obj::Raw(ops.to_vec())])?;
    writer.output(&[Obj::op(paint)])?;
    writer.end_stream()
}
