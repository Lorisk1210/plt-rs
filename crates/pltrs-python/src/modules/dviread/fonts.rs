//! matplotlib's `dviread.Tfm`, `TtfMetrics` and `Vf`, and the `DviFont`s a
//! program makes itself (`DviFont(scale, metrics, texname, vf)`,
//! `DviFont.from_luatex`, `DviFont.from_xetex`), over pltrs-text's readers.

use std::path::PathBuf;
use std::sync::Arc;

use pltrs_text::tex::{DviFont, DviMetrics, NativeEffects, NativeFace, TexMetrics, Tfm, Vf};

use super::tex_files;
use crate::binding::prelude::*;

native_value!(Arc<Tfm>, Arc<NativeFace>, Arc<Vf>);

/// A character's metrics as `(tex_width, tex_height, tex_depth)`, the
/// fields of matplotlib's `TexMetrics`.
type MetricsFields = (i32, i32, i32);

/// The fields of `metrics`.
const fn fields(metrics: TexMetrics) -> MetricsFields {
    (metrics.width, metrics.height, metrics.depth)
}

/// `Tfm(filename)`: the TeX font metrics in `data`.
fn tfm(data: Bytes) -> PyResult<Arc<Tfm>> {
    Ok(Arc::new(Tfm::parse(&data.0)?))
}

/// `Tfm.checksum` and `Tfm.design_size`.
fn tfm_header(tfm: Arc<Tfm>) -> PyResult<(u32, u32)> {
    Ok((tfm.checksum(), tfm.design_size()))
}

/// `Tfm.get_metrics(idx)`: the metrics of a character, `None` when the
/// font has none for it.
fn tfm_metrics(tfm: Arc<Tfm>, code: i64) -> PyResult<Option<MetricsFields>> {
    Ok(tfm.metrics(code).map(fields))
}

/// The characters a TFM file has metrics for, ascending.
fn tfm_codes(tfm: Arc<Tfm>) -> PyResult<Vec<i64>> {
    Ok(tfm.codes())
}

/// `TtfMetrics(filename)`: the glyph metrics of the first face of an
/// OpenType or TrueType font file.
fn ttf_metrics(path: PathBuf) -> PyResult<Arc<NativeFace>> {
    Ok(Arc::new(NativeFace::open(&path, 0)?))
}

/// `TtfMetrics.get_metrics(idx)`: the metrics of a glyph.
fn ttf_metrics_get(face: Arc<NativeFace>, index: i64) -> PyResult<MetricsFields> {
    Ok(fields(face.metrics(i128::from(index))?))
}

/// `Vf(filename)`: the virtual font in `data`, the fonts it refers to
/// found through kpathsea.
fn vf(data: Bytes) -> PyResult<Arc<Vf>> {
    Ok(Arc::new(Vf::parse(&data.0, tex_files()?)?))
}

/// A glyph of a packet: `(x, y, font, glyph, width)` in DVI units, y down.
type PacketGlyph = (i64, i64, Arc<DviFont>, i64, i64);

/// A number of a packet as a Python `int`.
fn int(value: i128) -> PyResult<i64> {
    i64::try_from(value)
        .map_err(|_| PyErr::new(ExceptionKind::OverflowError, "DVI number too large"))
}

/// `Vf[code]`: the glyphs, rules (`(x, y, height, width)`) and width of
/// the packet of a character, `None` when the font does not define it.
#[expect(
    clippy::type_complexity,
    reason = "the fields of matplotlib's Page of a packet"
)]
fn vf_packet(
    vf: Arc<Vf>,
    code: i64,
) -> PyResult<Option<(Vec<PacketGlyph>, Vec<(i64, i64, i64, i64)>, i64)>> {
    let Some(packet) = vf.packet(code) else {
        return Ok(None);
    };
    let glyphs = packet
        .glyphs()
        .iter()
        .map(|glyph| {
            Ok((
                int(glyph.x)?,
                int(glyph.y)?,
                Arc::clone(&glyph.font),
                int(glyph.glyph)?,
                int(glyph.width)?,
            ))
        })
        .collect::<PyResult<_>>()?;
    let rules = packet
        .rules()
        .iter()
        .map(|rule| {
            Ok((
                int(rule.x)?,
                int(rule.y)?,
                int(rule.height)?,
                int(rule.width)?,
            ))
        })
        .collect::<PyResult<_>>()?;
    Ok(Some((glyphs, rules, int(packet.width())?)))
}

/// The metrics a `DviFont` is made with: a `Tfm` or a `TtfMetrics`.
enum Metrics {
    Tfm(Arc<Tfm>),
    Native(Arc<NativeFace>),
}

impl FromPy<'_> for Metrics {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract::<Arc<Tfm>>() {
            Ok(tfm) => Ok(Self::Tfm(tfm)),
            Err(_) => Ok(Self::Native(object.extract()?)),
        }
    }
}

/// `DviFont(scale, metrics, texname, vf)`.
fn font_new(
    scale: i64,
    metrics: Metrics,
    texname: Bytes,
    vf: Option<Arc<Vf>>,
) -> PyResult<Arc<DviFont>> {
    let metrics = match metrics {
        Metrics::Tfm(tfm) => DviMetrics::Tfm(tfm),
        Metrics::Native(face) => DviMetrics::Native(face),
    };
    Ok(Arc::new(DviFont::new(
        i128::from(scale),
        metrics,
        texname.0,
        vf,
    )))
}

/// `DviFont.from_luatex(scale, texname)`.
fn font_from_luatex(scale: i64, texname: Bytes) -> PyResult<Arc<DviFont>> {
    Ok(Arc::new(DviFont::from_luatex(
        i128::from(scale),
        texname.0,
        tex_files()?,
    )?))
}

/// The effects of a native font as matplotlib's dict gives them: `rgba`
/// (four bytes), `extend`, `slant` and `embolden`.
type EffectsFields = (Option<Vec<u8>>, Option<f64>, Option<f64>, Option<f64>);

/// `DviFont.from_xetex(scale, texname, subfont, effects)`.
fn font_from_xetex(
    scale: i64,
    name: Bytes,
    subfont: u32,
    (rgba, extend, slant, embolden): EffectsFields,
) -> PyResult<Arc<DviFont>> {
    let rgba = rgba
        .map(|rgba| {
            <[u8; 4]>::try_from(rgba.as_slice())
                .map_err(|_| PyErr::value_error("rgba must hold four bytes"))
        })
        .transpose()?;
    let effects = NativeEffects {
        rgba,
        extend,
        slant,
        embolden,
    };
    Ok(Arc::new(DviFont::from_xetex(
        i128::from(scale),
        &name.0,
        subfont,
        effects,
        tex_files()?,
    )?))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("tfm", tfm)?;
    module.add("tfm_header", tfm_header)?;
    module.add("tfm_metrics", tfm_metrics)?;
    module.add("tfm_codes", tfm_codes)?;
    module.add("ttf_metrics", ttf_metrics)?;
    module.add("ttf_metrics_get", ttf_metrics_get)?;
    module.add("vf", vf)?;
    module.add("vf_packet", vf_packet)?;
    module.add("font_new", font_new)?;
    module.add("font_from_luatex", font_from_luatex)?;
    module.add("font_from_xetex", font_from_xetex)
}
