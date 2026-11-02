//! DVI files as `latex` writes them (matplotlib's `dviread.Dvi`): pages of
//! glyphs and rules, with the fonts they are set in, and their extents.

mod machine;

use std::sync::Arc;

use pltrs_core::{Path, Point};

pub(crate) use machine::{Machine, Registers, State, mul1220};

use super::error::{TexError, TexFile};
use super::files::FontSource;
use super::native::{NativeEffects, NativeFace, NativeFont};
use super::tfm::{TexMetrics, Tfm};
use super::vf::Vf;
use machine::Executed;

/// TeX's scaled points per inch at one dot per inch: DVI coordinates are
/// multiplied by `dpi / SCALED_POINTS_PER_INCH` (`72.27 * 2**16`).
const SCALED_POINTS_PER_INCH: f64 = 72.27 * 65536.0;

/// Where a DVI font's glyph metrics come from: TeX font metrics, or the
/// OpenType or TrueType font of a native font.
#[derive(Clone, Debug)]
pub enum DviMetrics {
    /// A TFM file.
    Tfm(Arc<Tfm>),
    /// The font file of a native font.
    Native(Arc<NativeFace>),
}

/// A font a DVI file sets glyphs in (matplotlib's `DviFont`): its TeX name,
/// scale and metrics, the virtual font it is, if it is one, and its font
/// file, if it is a native font of XeTeX or LuaTeX.
#[derive(Clone, Debug)]
pub struct DviFont {
    texname: Vec<u8>,
    scale: i128,
    metrics: DviMetrics,
    vf: Option<Arc<Vf>>,
    native: Option<NativeFont>,
}

impl PartialEq for DviFont {
    /// Fonts compare by name and size, as matplotlib's do.
    fn eq(&self, other: &Self) -> bool {
        self.texname == other.texname && self.size().to_bits() == other.size().to_bits()
    }
}

impl DviFont {
    /// A font of `metrics` scaled by `scale` (12.20 fixed point) that TeX
    /// knows as `texname`, the virtual font `vf` if it is one
    /// (matplotlib's `DviFont(scale, metrics, texname, vf)`).
    pub fn new(scale: i128, metrics: DviMetrics, texname: Vec<u8>, vf: Option<Arc<Vf>>) -> Self {
        Self {
            texname,
            scale,
            metrics,
            vf,
            native: None,
        }
    }

    /// A native font of LuaTeX (matplotlib's `DviFont.from_luatex`): its
    /// `texname` is the font file in brackets, optionally followed by `:`
    /// and its options (see the DVI files LuaTeX writes); the metrics are
    /// read from the file through `files`.
    ///
    /// # Errors
    ///
    /// When the name is not of that form or the font file cannot be read.
    pub fn from_luatex(
        scale: i128,
        texname: Vec<u8>,
        files: &dyn FontSource,
    ) -> Result<Self, TexError> {
        let native = NativeFont::luatex(&texname)?;
        let metrics = files.native_face(&native.path, native.subfont)?;
        Ok(Self::native(scale, texname, native, metrics))
    }

    /// A native font of XeTeX (matplotlib's `DviFont.from_xetex`): face
    /// `subfont` of the font file `name`, shown with `effects`, its
    /// metrics read from the file through `files`; TeX knows it as the
    /// name in brackets.
    ///
    /// # Errors
    ///
    /// When the font file cannot be read or has no such face.
    pub fn from_xetex(
        scale: i128,
        name: &[u8],
        subfont: u32,
        effects: NativeEffects,
        files: &dyn FontSource,
    ) -> Result<Self, TexError> {
        let native = NativeFont::from_file_name(name, subfont, effects);
        let metrics = files.native_face(&native.path, subfont)?;
        let mut texname = Vec::with_capacity(name.len() + 2);
        texname.push(b'[');
        texname.extend_from_slice(name);
        texname.push(b']');
        Ok(Self::native(scale, texname, native, metrics))
    }

    /// A native font, its metrics read from its font file.
    pub(crate) fn native(
        scale: i128,
        texname: Vec<u8>,
        font: NativeFont,
        metrics: Arc<NativeFace>,
    ) -> Self {
        Self {
            texname,
            scale,
            metrics: DviMetrics::Native(metrics),
            vf: None,
            native: Some(font),
        }
    }

    /// The same font at another `scale` (a font of a virtual font's
    /// packets as the virtual font scales it).
    pub(crate) fn rescaled(&self, scale: i128) -> Self {
        Self {
            scale,
            ..self.clone()
        }
    }

    /// The name TeX knows the font by (`texname`), usually ASCII; the
    /// font map gives its PostScript font. A native font's is its file in
    /// brackets (with LuaTeX's options after them).
    pub fn texname(&self) -> &[u8] {
        &self.texname
    }

    /// The name as text (`fname`, read as Latin-1).
    pub fn name(&self) -> String {
        self.texname.iter().map(|&b| char::from(b)).collect()
    }

    /// The factor the font is scaled by from its design size, in 12.20
    /// fixed point.
    pub fn scale(&self) -> i128 {
        self.scale
    }

    /// The size in PostScript points, converted from TeX points
    /// (matplotlib's `DviFont.size`).
    pub fn size(&self) -> f64 {
        self.scale as f64 * (72.0 / SCALED_POINTS_PER_INCH)
    }

    /// Where the font's metrics come from.
    pub fn metrics(&self) -> &DviMetrics {
        &self.metrics
    }

    /// The widths of the characters from 0 to the last the TFM file
    /// defines, in thousandths of the design size, 0 for those it does
    /// not (matplotlib's deprecated `DviFont.widths`); none for a native
    /// font, whose characters are glyph indices.
    pub fn widths(&self) -> Vec<i64> {
        let DviMetrics::Tfm(tfm) = &self.metrics else {
            return Vec::new();
        };
        let last = tfm.codes().last().copied().unwrap_or(-1);
        (0..=last)
            .map(|code| {
                tfm.metrics(code)
                    .map_or(0, |metrics| (1000 * i64::from(metrics.width)) >> 20)
            })
            .collect()
    }

    /// The virtual font this font is, if it is one.
    pub fn vf(&self) -> Option<&Arc<Vf>> {
        self.vf.as_ref()
    }

    /// The font file, face and effects of a native font of XeTeX or
    /// LuaTeX, whose character codes are glyph indices.
    pub fn native_font(&self) -> Option<&NativeFont> {
        self.native.as_ref()
    }

    /// The metrics of character `code` relative to the design size
    /// (`_metrics.get_metrics`), `None` for a character the TFM file has
    /// no metrics for.
    ///
    /// # Errors
    ///
    /// When a native font has no such glyph.
    pub fn char_metrics(&self, code: i128) -> Result<Option<TexMetrics>, TexError> {
        match &self.metrics {
            DviMetrics::Tfm(tfm) => Ok(i64::try_from(code).ok().and_then(|c| tfm.metrics(c))),
            DviMetrics::Native(native) => native.metrics(code).map(Some),
        }
    }

    /// `_width_of`: the width of character `code` in DVI units (0 for a
    /// character the font has no metrics for).
    ///
    /// # Errors
    ///
    /// When the width overflows or a native font has no such glyph.
    pub fn width_of(&self, code: i128) -> Result<i128, TexError> {
        match self.char_metrics(code)? {
            Some(metrics) => mul1220(i128::from(metrics.width), self.scale),
            None => Ok(0),
        }
    }

    /// `_height_depth_of`: the height and depth of character `code` in DVI
    /// units. The minus sign of the `cmsy` fonts (character 0) gets no
    /// depth, as matplotlib aligns `dvipng` images by the ink.
    ///
    /// # Errors
    ///
    /// When the extent overflows or a native font has no such glyph.
    pub fn height_depth_of(&self, code: i128) -> Result<(i128, i128), TexError> {
        let Some(metrics) = self.char_metrics(code)? else {
            return Ok((0, 0));
        };
        let height = mul1220(i128::from(metrics.height), self.scale)?;
        let depth = if code == 0 && is_cmsy(&self.texname) {
            0
        } else {
            mul1220(i128::from(metrics.depth), self.scale)?
        };
        Ok((height, depth))
    }
}

/// `re.match(br'^cmsy\d+$', name)` (`$` also matches before a final
/// newline).
fn is_cmsy(name: &[u8]) -> bool {
    let name = name.strip_suffix(b"\n").unwrap_or(name);
    name.strip_prefix(b"cmsy")
        .is_some_and(|digits| !digits.is_empty() && digits.iter().all(u8::is_ascii_digit))
}

/// A glyph in DVI units (scaled points): the position of its origin (y
/// down), its font, character code and width.
#[derive(Clone, Debug)]
pub struct RawGlyph {
    /// Horizontal position.
    pub x: i128,
    /// Vertical position, down.
    pub y: i128,
    /// The font.
    pub font: Arc<DviFont>,
    /// The character code.
    pub glyph: i128,
    /// The advance width.
    pub width: i128,
}

/// A rule in DVI units (scaled points): its bottom-left corner (y down)
/// and size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawRule {
    /// Horizontal position of the left edge.
    pub x: i128,
    /// Vertical position of the bottom edge, down.
    pub y: i128,
    /// Height.
    pub height: i128,
    /// Width.
    pub width: i128,
}

/// A page as read, in DVI units, with the baseline matplotlib detects
/// from the page's structure.
#[derive(Clone, Debug)]
pub struct RawPage {
    pub(crate) text: Vec<RawGlyph>,
    pub(crate) boxes: Vec<RawRule>,
    pub(crate) baseline: Option<i128>,
}

/// A glyph on a page (matplotlib's `dviread.Text`): the position of its
/// origin relative to the page's bottom-left corner, y up, its font,
/// character code (as the DVI file stores it) and width.
#[derive(Clone, Debug, PartialEq)]
pub struct DviText {
    /// Distance from the left edge.
    pub x: f64,
    /// Height of the baseline above the bottom edge.
    pub y: f64,
    /// The font.
    pub font: Arc<DviFont>,
    /// The character code.
    pub glyph: i64,
    /// The advance width.
    pub width: f64,
}

/// A rule on a page (matplotlib's `dviread.Box`): its bottom-left corner
/// relative to the page's bottom-left corner, y up, and its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DviBox {
    /// Distance from the left edge.
    pub x: f64,
    /// Distance from the bottom edge.
    pub y: f64,
    /// Height.
    pub height: f64,
    /// Width.
    pub width: f64,
}

impl DviBox {
    /// The rule as a closed rectangle from its bottom-left corner, going
    /// right first (as `TextToPath` outlines rules).
    pub fn outline(&self) -> Path {
        let (x, y, w, h) = (self.x, self.y, self.width, self.height);
        Path::compound_from_polygons(&[[
            Point::new(x, y),
            Point::new(x + w, y),
            Point::new(x + w, y + h),
            Point::new(x, y + h),
            Point::new(x, y),
        ]])
    }
}

/// A page at a resolution (matplotlib's `dviread.Page`): its glyphs and
/// rules, and the extent of its ink, in pixels at that resolution.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DviPage {
    /// The glyphs.
    pub text: Vec<DviText>,
    /// The rules.
    pub boxes: Vec<DviBox>,
    /// Width of the ink.
    pub width: f64,
    /// Height of the ink above the baseline.
    pub height: f64,
    /// Depth of the ink below the baseline.
    pub descent: f64,
}

/// The error of DVI arithmetic that overflows.
fn too_large() -> TexError {
    TexError::malformed(TexFile::Dvi, "number too large")
}

/// `a - b`, an error when it overflows.
fn difference(a: i128, b: i128) -> Result<i128, TexError> {
    a.checked_sub(b).ok_or_else(too_large)
}

/// `a + b`, an error when it overflows.
fn sum(a: i128, b: i128) -> Result<i128, TexError> {
    a.checked_add(b).ok_or_else(too_large)
}

/// The extremes of a page's ink and its baseline, in DVI units.
struct Ink {
    minx: i128,
    miny: i128,
    maxx: i128,
    maxy: i128,
    baseline: i128,
}

impl RawPage {
    /// The glyphs, in the order they are set.
    pub fn glyphs(&self) -> &[RawGlyph] {
        &self.text
    }

    /// The rules.
    pub fn rules(&self) -> &[RawRule] {
        &self.boxes
    }

    /// The extremes of the ink (`None` for an empty page), the baseline
    /// being the detected one if any, else the lowest glyph origin.
    fn ink(&self) -> Result<Option<Ink>, TexError> {
        if self.text.is_empty() && self.boxes.is_empty() {
            return Ok(None);
        }
        let (mut minx, mut miny) = (i128::MAX, i128::MAX);
        let (mut maxx, mut maxy, mut maxy_pure) = (i128::MIN, i128::MIN, i128::MIN);
        let glyphs = self.text.iter().map(|text| {
            let (h, e) = text.font.height_depth_of(text.glyph)?;
            Ok((text.x, text.y, h, text.width, e))
        });
        let rules = self
            .boxes
            .iter()
            .map(|b| Ok((b.x, b.y, b.height, b.width, 0)));
        for extent in glyphs.chain(rules) {
            let (x, y, h, w, e): (i128, i128, i128, i128, i128) = extent?;
            minx = minx.min(x);
            miny = miny.min(difference(y, h)?);
            maxx = maxx.max(sum(x, w)?);
            maxy = maxy.max(sum(y, e)?);
            maxy_pure = maxy_pure.max(y);
        }
        Ok(Some(Ink {
            minx,
            miny,
            maxx,
            maxy,
            baseline: self.baseline.unwrap_or(maxy_pure),
        }))
    }

    /// The width, height and descent of the ink in DVI units (matplotlib's
    /// `Dvi._output` with `dpi=None`), all zero for an empty page.
    ///
    /// # Errors
    ///
    /// When a glyph's metrics or the extent overflow.
    pub fn extent(&self) -> Result<(i128, i128, i128), TexError> {
        let Some(ink) = self.ink()? else {
            return Ok((0, 0, 0));
        };
        Ok((
            difference(ink.maxx, ink.minx)?,
            difference(ink.baseline, ink.miny)?,
            difference(ink.maxy, ink.baseline)?,
        ))
    }

    /// The page in pixels at `dpi` (matplotlib's `Dvi._output`).
    ///
    /// # Errors
    ///
    /// When a glyph's metrics or the extent overflow.
    pub fn at(&self, dpi: f64) -> Result<DviPage, TexError> {
        let Some(ink) = self.ink()? else {
            return Ok(DviPage::default());
        };
        let Ink {
            minx,
            miny,
            maxx,
            maxy,
            baseline,
        } = ink;
        let d = dpi / SCALED_POINTS_PER_INCH;
        let pixels = |a: i128, b: i128| Ok::<_, TexError>(difference(a, b)? as f64 * d);
        let descent = pixels(maxy, baseline)?;
        let text = self
            .text
            .iter()
            .map(|text| {
                Ok(DviText {
                    x: pixels(text.x, minx)?,
                    y: pixels(maxy, text.y)? - descent,
                    font: Arc::clone(&text.font),
                    glyph: text.glyph as i64,
                    width: text.width as f64 * d,
                })
            })
            .collect::<Result<_, TexError>>()?;
        let boxes = self
            .boxes
            .iter()
            .map(|b| {
                Ok(DviBox {
                    x: pixels(b.x, minx)?,
                    y: pixels(maxy, b.y)? - descent,
                    height: b.height as f64 * d,
                    width: b.width as f64 * d,
                })
            })
            .collect::<Result<_, TexError>>()?;
        Ok(DviPage {
            text,
            boxes,
            width: pixels(maxx, minx)?,
            height: pixels(baseline, miny)?,
            descent,
        })
    }
}

/// Reads one page (`Dvi._read`), `None` after the postamble. The baseline
/// is the vertical position when the stack first reaches depth 3 after at
/// least four `down` moves that were not popped.
fn read_page(machine: &mut Machine<'_>) -> Result<Option<RawPage>, TexError> {
    let mut down_stack: Vec<u64> = vec![0];
    let mut baseline = None;
    loop {
        let executed = machine.next_opcode()?;
        if let Some(missing) = machine.missing_font.take() {
            return Err(missing);
        }
        let top = |stack: &Vec<u64>| stack.last().copied();
        match executed {
            Executed::Push => down_stack.push(top(&down_stack).unwrap_or(0)),
            Executed::Pop => {
                down_stack.pop();
            }
            Executed::Down => {
                if let Some(last) = down_stack.last_mut() {
                    *last += 1;
                }
            }
            Executed::EndOfPage | Executed::Other => {}
        }
        if baseline.is_none()
            && machine.stack.len() == 3
            && top(&down_stack).ok_or_else(|| machine.error("list index out of range"))? >= 4
        {
            baseline = Some(machine.regs.v);
        }
        if executed == Executed::EndOfPage {
            return Ok(Some(RawPage {
                text: std::mem::take(&mut machine.text),
                boxes: std::mem::take(&mut machine.boxes),
                baseline,
            }));
        }
        if machine.state == State::PostPost {
            return Ok(None);
        }
    }
}

/// Reads every page of a DVI file (iterating over matplotlib's `Dvi`),
/// loading the fonts it defines from `fonts`.
///
/// # Errors
///
/// When the file is malformed, uses a font that cannot be found, or ends
/// before its postamble.
pub fn read_dvi(data: &[u8], fonts: &dyn FontSource) -> Result<Vec<RawPage>, TexError> {
    let mut machine = Machine::new(data, fonts, TexFile::Dvi);
    let mut pages = Vec::new();
    while let Some(page) = read_page(&mut machine)? {
        pages.push(page);
    }
    Ok(pages)
}

#[cfg(test)]
mod tests;
