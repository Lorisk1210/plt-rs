//! The OpenType and TrueType fonts XeTeX and LuaTeX set glyphs in, their
//! "native fonts" (matplotlib's `DviFont.from_xetex`,
//! `DviFont.from_luatex` and `TtfMetrics`): the font file, the face in it,
//! the effects the document asked for, and glyph metrics in TeX's units
//! read from the font itself. Their DVI character codes are glyph
//! indices.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use pltrs_font::{FontId, GlyphId};

use super::error::{TexError, TexFile, bytes_repr};
use super::tfm::TexMetrics;
use crate::engine::TextEngine;

/// XeTeX's `define_native_font` flags (`flags & ...`) for the effects
/// that follow the font's name and index.
const XETEX_COLORED: u16 = 0x0200;
const XETEX_EXTEND: u16 = 0x1000;
const XETEX_SLANT: u16 = 0x2000;
const XETEX_EMBOLDEN: u16 = 0x4000;

/// The checksum LuaTeX gives its native fonts (`"LuaF"`).
pub(crate) const LUATEX_CHECKSUM: u32 = 0x4c75_6146;

/// What a document asked a native font to look like (matplotlib's
/// `DviFont.effects` of a native font): its colour, and how much it is
/// extended, slanted and emboldened.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NativeEffects {
    /// The colour, as red, green, blue and alpha bytes (XeTeX only).
    pub rgba: Option<[u8; 4]>,
    /// The horizontal stretch factor.
    pub extend: Option<f64>,
    /// The slant, as the tangent of the angle.
    pub slant: Option<f64>,
    /// How much the strokes are emboldened.
    pub embolden: Option<f64>,
}

/// A native font: its file, the face in the file, and its effects.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeFont {
    /// The font file, as the DVI file names it (relative paths are
    /// relative to the directory TeX ran in).
    pub path: PathBuf,
    /// The index of the face in the file (`subfont`).
    pub subfont: u32,
    /// The effects.
    pub effects: NativeEffects,
}

/// A path from the bytes of a file name (`os.fsdecode`).
fn path_of(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// A 16.16 fixed-point number.
fn fixed(value: i128) -> f64 {
    value as f64 / 65536.0
}

impl NativeFont {
    /// Face `subfont` of the font file `name` (the bytes of its path),
    /// shown with `effects`.
    pub(crate) fn from_file_name(name: &[u8], subfont: u32, effects: NativeEffects) -> Self {
        Self {
            path: path_of(name),
            subfont,
            effects,
        }
    }

    /// A font of XeTeX's `define_native_font`: the file `name`, face
    /// `index`, and the effects `flags` announces, whose values `read`
    /// gives (`read(bytes, signed)`: four colour bytes, then each factor
    /// as a signed 16.16 number).
    pub(crate) fn xetex(
        name: &[u8],
        index: u32,
        flags: u16,
        mut read: impl FnMut(usize, bool) -> i128,
    ) -> Self {
        let mut effects = NativeEffects::default();
        if flags & XETEX_COLORED != 0 {
            effects.rgba = Some([(); 4].map(|()| read(1, false) as u8));
        }
        let mut factor = |flag| (flags & flag != 0).then(|| fixed(read(4, true)));
        effects.extend = factor(XETEX_EXTEND);
        effects.slant = factor(XETEX_SLANT);
        effects.embolden = factor(XETEX_EMBOLDEN);
        Self {
            path: path_of(name),
            subfont: index,
            effects,
        }
    }

    /// A font of LuaTeX's font definitions, whose name is the font file in
    /// brackets, optionally followed by `:` and `key=value` options joined
    /// by `;` (`index`, the face, and the `embolden`, `slant` and
    /// `extend` effects as 16.16 numbers; other options are ignored).
    ///
    /// # Errors
    ///
    /// When the name is not of that form.
    pub(crate) fn luatex(texname: &[u8]) -> Result<Self, TexError> {
        let invalid = || {
            TexError::malformed(
                TexFile::Dvi,
                format!("Invalid modern font name: {}", bytes_repr(texname)),
            )
        };
        let inner = texname.strip_prefix(b"[").ok_or_else(invalid)?;
        let close = inner.iter().rposition(|&b| b == b']').ok_or_else(invalid)?;
        let (path, rest) = (&inner[..close], &inner[close + 1..]);
        let options = match rest {
            [] => &[][..],
            [b':', options @ ..] => options,
            _ => return Err(invalid()),
        };
        let mut font = Self {
            path: path_of(path),
            subfont: 0,
            effects: NativeEffects::default(),
        };
        if options.is_empty() {
            return Ok(font);
        }
        let options = std::str::from_utf8(options)
            .ok()
            .filter(|options| options.is_ascii())
            .ok_or_else(invalid)?;
        for option in options.split(';') {
            let (key, value) = option.split_once('=').ok_or_else(invalid)?;
            let number = || value.parse::<i128>().map_err(|_| invalid());
            match key {
                "index" => font.subfont = value.parse().map_err(|_| invalid())?,
                "embolden" => font.effects.embolden = Some(fixed(number()?)),
                "slant" => font.effects.slant = Some(fixed(number()?)),
                "extend" => font.effects.extend = Some(fixed(number()?)),
                _ => {}
            }
        }
        Ok(font)
    }
}

/// The face of a native font (matplotlib's `TtfMetrics` and the
/// `FT2Font` `dviread` opens for it): each glyph's advance, height and
/// depth in TeX's units, as fractions of the em in 12.20 fixed point read
/// unscaled from the font and rounded, and each glyph's name.
#[derive(Debug)]
pub struct NativeFace {
    font: FontId,
    units_per_em: f64,
    glyphs: Mutex<HashMap<u16, TexMetrics>>,
}

impl NativeFace {
    /// Face `subfont` of the font file at `path` (relative to the current
    /// directory, as for TeX, which wrote it).
    ///
    /// # Errors
    ///
    /// When the path names no regular file, or the file cannot be read or
    /// has no such face.
    pub fn open(path: &Path, subfont: u32) -> Result<Self, TexError> {
        // DVI files are untrusted: a device or a pipe is no font file.
        let path = &std::path::absolute(path).map_err(|err| TexError::io(path, &err))?;
        let metadata = std::fs::metadata(path).map_err(|err| TexError::io(path, &err))?;
        if !metadata.is_file() {
            return Err(TexError::malformed(
                TexFile::NativeFont,
                format!("{}: not a font file", path.display()),
            ));
        }
        let database = TextEngine::global().database();
        let font = database
            .file_faces(path)
            .map_err(|err| font_error(path, &err))?
            .get(subfont as usize)
            .copied()
            .ok_or_else(|| {
                TexError::malformed(
                    TexFile::NativeFont,
                    format!("{}: no face {subfont}", path.display()),
                )
            })?;
        let face = database.face(font).map_err(|err| font_error(path, &err))?;
        Ok(Self {
            font,
            units_per_em: f64::from(face.units_per_em()),
            glyphs: Mutex::default(),
        })
    }

    /// The metrics of glyph `index` (`get_metrics`).
    ///
    /// # Errors
    ///
    /// When the face has no such glyph, its outline is malformed or its
    /// metrics do not fit TeX's numbers.
    pub fn metrics(&self, index: i128) -> Result<TexMetrics, TexError> {
        let glyph = glyph_id(index)?;
        let mut glyphs = self.glyphs.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(metrics) = glyphs.get(&glyph.0) {
            return Ok(*metrics);
        }
        let database = TextEngine::global().database();
        let face = database.face(self.font).map_err(|_| no_glyph(index))?;
        let (advance, bbox) = face.unscaled_metrics(glyph).map_err(|_| no_glyph(index))?;
        let (top, bottom) = bbox.map_or((0, 0), |bbox| (bbox.y_max, bbox.y_min));
        let units = |value: i64| {
            // `round(value / upem * 2**20)`, rounding halves to even.
            let tex = (value as f64 / self.units_per_em * f64::from(1 << 20)).round_ties_even();
            i32::try_from(tex as i64).map_err(|_| {
                TexError::malformed(
                    TexFile::NativeFont,
                    format!("glyph {index}: metrics too large"),
                )
            })
        };
        let metrics = TexMetrics {
            width: units(advance)?,
            height: units(i64::from(top))?,
            // `height - horiBearingY`: how far the box reaches below the
            // baseline.
            depth: units(-i64::from(bottom))?,
        };
        glyphs.insert(glyph.0, metrics);
        Ok(metrics)
    }

    /// The name of glyph `index` (`FT2Font.get_glyph_name`): its name in
    /// the font, `uni` and eight hexadecimal digits of the index for a
    /// face without glyph names, empty for a glyph without one.
    ///
    /// # Errors
    ///
    /// When the face has no such glyph.
    pub fn glyph_name(&self, index: i128) -> Result<String, TexError> {
        let glyph = glyph_id(index)?;
        let face = TextEngine::global()
            .database()
            .face(self.font)
            .map_err(|_| no_glyph(index))?;
        if glyph.0 >= face.num_glyphs() {
            return Err(no_glyph(index));
        }
        if !face.has_glyph_names() {
            return Ok(format!("uni{:08x}", glyph.0));
        }
        Ok(face.glyph_name(glyph).unwrap_or_default().to_owned())
    }
}

/// The glyph a native font's character code is.
fn glyph_id(index: i128) -> Result<GlyphId, TexError> {
    u16::try_from(index)
        .map(GlyphId)
        .map_err(|_| no_glyph(index))
}

/// The error of a glyph index a native font does not have.
fn no_glyph(index: i128) -> TexError {
    TexError::malformed(TexFile::NativeFont, format!("no glyph {index}"))
}

/// The error of a font file that cannot be read or parsed.
fn font_error(path: &Path, err: &pltrs_font::FontError) -> TexError {
    match err {
        pltrs_font::FontError::Io { kind, .. } => TexError::Io {
            path: path.display().to_string(),
            message: std::io::Error::from(*kind).to_string(),
        },
        other => TexError::malformed(TexFile::NativeFont, format!("{}: {other}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tex::test_support::DEJAVU_SANS;

    #[test]
    fn luatex_names_give_the_file_face_and_effects() {
        let font =
            NativeFont::luatex(b"[DejaVuSans.ttf]:extend=81920;slant=16384;embolden=16384;color=0")
                .expect("valid");
        assert_eq!(font.path, PathBuf::from("DejaVuSans.ttf"));
        assert_eq!(font.subfont, 0);
        let effects = NativeEffects {
            rgba: None,
            extend: Some(1.25),
            slant: Some(0.25),
            embolden: Some(0.25),
        };
        assert_eq!(font.effects, effects);
        let plain = NativeFont::luatex(b"[/fonts/a]b.otf]").expect("valid");
        assert_eq!(plain.path, PathBuf::from("/fonts/a]b.otf"));
        assert_eq!(plain.effects, NativeEffects::default());
        let face = NativeFont::luatex(b"[f.ttc]:index=2").expect("valid");
        assert_eq!(face.subfont, 2);
        for invalid in [
            &b"f.ttf"[..],
            b"[f.ttf",
            b"[f.ttf]x",
            b"[f.ttf]:slant",
            b"[f.ttf]:slant=x",
        ] {
            let err = NativeFont::luatex(invalid).expect_err("invalid");
            assert!(
                err.to_string().contains("Invalid modern font name"),
                "{err}"
            );
        }
    }

    #[test]
    fn xetex_fonts_read_the_effects_their_flags_announce() {
        let mut values = vec![0, 0, 255, 255, 81920, 16384].into_iter();
        let font = NativeFont::xetex(b"./DejaVuSans.ttf", 0, 0x3200, |_, _| {
            values.next().expect("announced")
        });
        assert_eq!(font.path, PathBuf::from("./DejaVuSans.ttf"));
        let effects = NativeEffects {
            rgba: Some([0, 0, 255, 255]),
            extend: Some(1.25),
            slant: Some(0.25),
            embolden: None,
        };
        assert_eq!(font.effects, effects);
        assert!(values.next().is_none(), "nothing more read");
    }

    #[test]
    fn faces_measure_glyphs_in_tex_units() {
        let face = NativeFace::open(Path::new(DEJAVU_SANS), 0).expect("on disk");
        // `A` advances 1401 of 2048 units and rises 1493: exact in 12.20.
        let a = face.metrics(36).expect("glyph");
        assert_eq!((a.width, a.height, a.depth), (1401 << 9, 1493 << 9, 0));
        // `g` reaches 426 units below the baseline.
        let g = face.metrics(74).expect("glyph");
        assert_eq!(g.depth, 426 << 9);
        assert_eq!(face.glyph_name(36).expect("glyph"), "A");
        assert!(face.metrics(-1).is_err());
        assert!(face.metrics(1 << 20).is_err());
        assert!(face.glyph_name(70_000).is_err());
        assert!(NativeFace::open(Path::new(DEJAVU_SANS), 1).is_err());
        let directory = Path::new(DEJAVU_SANS).parent().expect("a directory");
        assert!(NativeFace::open(directory, 0).is_err(), "not a font file");
        assert!(matches!(
            NativeFace::open(Path::new("/no/such/font.ttf"), 0),
            Err(TexError::Io { .. })
        ));
    }
}
