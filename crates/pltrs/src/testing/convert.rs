//! Rasterising the files image comparisons meet in other formats than PNG
//! (matplotlib's `testing.compare.converter`, `convert` and
//! `get_file_hash`): PDF and PostScript files with Ghostscript and SVG
//! files with Inkscape, when they are installed, as matplotlib's
//! converters run them, and GIF files by pltrs itself (where matplotlib
//! runs `ImageMagick`).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use pltrs_codec::png::{self, ImageRef};
use pltrs_codec::sha256::sha256_hex;

use super::compare::ComparisonError;
use crate::figure::{Executable, ProgramError, SaveFormat, Workspace};

/// How a file of one format becomes a PNG image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Converter {
    /// Ghostscript's `png16m` device, for PDF and (Encapsulated)
    /// PostScript files (matplotlib's `_GSConverter`).
    Ghostscript,
    /// Inkscape's PNG export (matplotlib's `_SVGConverter`).
    Inkscape,
    /// pltrs' own GIF decoder.
    Gif,
}

impl Converter {
    /// The converter of the files with the extension of `path`, if any.
    fn of(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "pdf" | "eps" => Some(Self::Ghostscript),
            "svg" => Some(Self::Inkscape),
            "gif" => Some(Self::Gif),
            _ => None,
        }
    }

    /// Whether the converter can run here: whether its program is
    /// installed.
    fn available(self) -> bool {
        match self {
            Self::Ghostscript => Executable::Ghostscript.check().is_ok(),
            Self::Inkscape => Executable::Inkscape.check().is_ok(),
            Self::Gif => true,
        }
    }

    /// Writes the PNG image of `orig` to `dest`.
    fn run(self, orig: &Path, dest: &Path) -> Result<(), ComparisonError> {
        let failed = |err: ProgramError| ComparisonError::Convert {
            path: orig.to_owned(),
            reason: err.to_string(),
        };
        match self {
            Self::Ghostscript => ghostscript(orig, dest).map_err(failed),
            Self::Inkscape => inkscape(orig, dest).map_err(failed),
            Self::Gif => gif(orig, dest),
        }
    }
}

/// The formats [`compare_images`](super::compare_images) compares here
/// (matplotlib's `comparable_formats`): PNG, GIF, PDF and PostScript when
/// Ghostscript is installed, and SVG when Inkscape is.
///
/// ```
/// use pltrs::figure::SaveFormat;
/// use pltrs::testing::comparable_formats;
/// let formats = comparable_formats();
/// assert_eq!(formats[..2], [SaveFormat::Png, SaveFormat::Gif]);
/// ```
pub fn comparable_formats() -> Vec<SaveFormat> {
    let mut formats = vec![SaveFormat::Png, SaveFormat::Gif];
    if Converter::Ghostscript.available() {
        formats.extend([SaveFormat::Pdf, SaveFormat::Eps]);
    }
    if Converter::Inkscape.available() {
        formats.push(SaveFormat::Svg);
    }
    formats
}

/// Whether [`compare_images`](super::compare_images) can compare the file
/// `path`: a PNG file, or one a converter installed here rasterises.
pub(super) fn is_comparable(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "png")
        || Converter::of(path).is_some_and(Converter::available)
}

/// The PNG image of `path` (matplotlib's `convert`): `<stem>_<ext>.png`
/// beside it, made unless it is newer than `path` already. With `cache`,
/// conversions are kept in the `test_cache` directory of the
/// [cache directory](crate::config::cache_dir), named by
/// [`get_file_hash`], and copied from there when the same file is
/// converted again (pltrs does not prune the cache, which matplotlib
/// keeps to twice the size of its baseline images).
///
/// ```no_run
/// use pltrs::prelude::*;
/// use pltrs::testing::convert;
/// let fig = Figure::new();
/// fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
/// fig.savefig("plot.pdf")?;
/// // Needs Ghostscript.
/// assert_eq!(convert("plot.pdf", false)?, std::path::Path::new("plot_pdf.png"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When `path` does not exist, no converter installed here takes its
/// format, the converter fails, or the image or the cache cannot be
/// written.
pub fn convert(path: impl AsRef<Path>, cache: bool) -> Result<PathBuf, ComparisonError> {
    let path = path.as_ref();
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|_| ComparisonError::MissingOutput(path.to_owned()))?;
    let converter = Converter::of(path)
        .filter(|converter| converter.available())
        .ok_or_else(|| ComparisonError::NotComparable(path.to_owned()))?;
    let newpath = converted_path(path);
    let fresh = fs::metadata(&newpath)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|converted| converted >= modified);
    if fresh {
        return Ok(newpath);
    }
    let cached = if cache {
        let dir = cache_dir()?;
        Some(dir.join(format!("{}.png", get_file_hash(path)?)))
    } else {
        None
    };
    if let Some(cached) = cached.as_ref().filter(|cached| cached.exists()) {
        copy(cached, &newpath)?;
        return Ok(newpath);
    }
    converter.run(path, &newpath)?;
    if let Some(cached) = &cached {
        copy(&newpath, cached)?;
    }
    Ok(newpath)
}

/// The SHA-256 of the file `path` in hexadecimal, with the version of
/// Ghostscript for PDF files and of Inkscape for SVG files (matplotlib's
/// `get_file_hash`), as conversions are cached by.
///
/// ```
/// use pltrs::testing::get_file_hash;
/// let path = std::env::temp_dir().join(format!("pltrs-hash-doc-{}.txt", std::process::id()));
/// std::fs::write(&path, b"abc")?;
/// assert_eq!(
///     get_file_hash(&path)?,
///     "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
/// );
/// std::fs::remove_file(path)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the file cannot be read, or the program whose version is hashed
/// is not installed.
pub fn get_file_hash(path: impl AsRef<Path>) -> Result<String, ComparisonError> {
    let path = path.as_ref();
    let mut data = fs::read(path).map_err(|err| ComparisonError::Unreadable {
        path: path.to_owned(),
        reason: err.to_string(),
    })?;
    let program = match path.extension().and_then(|extension| extension.to_str()) {
        Some("pdf") => Some(Executable::Ghostscript),
        Some("svg") => Some(Executable::Inkscape),
        _ => None,
    };
    if let Some(program) = program {
        let version = program.version().map_err(|err| ComparisonError::Convert {
            path: path.to_owned(),
            reason: err.to_string(),
        })?;
        data.extend_from_slice(version.as_bytes());
    }
    Ok(sha256_hex(&data))
}

/// Where [`convert`] writes the image of `path`: `<stem>_<ext>.png`.
fn converted_path(path: &Path) -> PathBuf {
    let mut name = path.file_stem().unwrap_or_default().to_owned();
    name.push("_");
    name.push(path.extension().unwrap_or_default());
    name.push(".png");
    path.with_file_name(name)
}

/// The directory conversions are cached in, made if missing.
fn cache_dir() -> Result<PathBuf, ComparisonError> {
    let dir = crate::config::cache_dir()
        .map_err(|err| ComparisonError::Unwritable {
            path: PathBuf::from("test_cache"),
            reason: err.to_string(),
        })?
        .join("test_cache");
    fs::create_dir_all(&dir).map_err(|err| ComparisonError::Unwritable {
        path: dir.clone(),
        reason: err.to_string(),
    })?;
    Ok(dir)
}

/// Copies `from` to `to`.
fn copy(from: &Path, to: &Path) -> Result<(), ComparisonError> {
    fs::copy(from, to)
        .map(drop)
        .map_err(|err| ComparisonError::Unwritable {
            path: to.to_owned(),
            reason: err.to_string(),
        })
}

/// Rasterises the PDF or PostScript file `orig` into `dest` with
/// matplotlib's Ghostscript settings: 72 dpi, 24-bit colour, the page of
/// an EPS file cropped to its bounding box.
fn ghostscript(orig: &Path, dest: &Path) -> Result<(), ProgramError> {
    // Ghostscript runs in a directory of its own.
    let absolute =
        |path: &Path| std::path::absolute(path).map_err(|err| ProgramError::io(path, &err));
    let (orig, dest) = (absolute(orig)?, absolute(dest)?);
    let mut output = OsString::from("-sOutputFile=");
    output.push(&dest);
    let args: Vec<OsString> = [
        "-q",
        "-dNOSAFER",
        "-dNOPAUSE",
        "-dBATCH",
        "-dEPSCrop",
        "-sDEVICE=png16m",
    ]
    .into_iter()
    .map(OsString::from)
    .chain([output, orig.as_os_str().to_owned()])
    .collect();
    let gs = Executable::Ghostscript.check()?;
    Workspace::new(0)?.run(gs, &args)?;
    if dest.exists() {
        Ok(())
    } else {
        Err(ProgramError::Io {
            path: dest.display().to_string(),
            message: "Ghostscript wrote no image".to_owned(),
        })
    }
}

/// Rasterises the SVG file `orig` into `dest` with Inkscape, run as
/// matplotlib runs it: without a display, without the user's
/// preferences, on fixed file names in a directory of its own, and with
/// the fonts pltrs bundles when the drawing names fonts (SVG files saved
/// with `svg.fonttype: none`).
fn inkscape(orig: &Path, dest: &Path) -> Result<(), ProgramError> {
    let workspace = Workspace::new(0)?;
    let profile = workspace.path("");
    let svg = workspace.path("f.svg");
    let png = workspace.path("f.png");
    let source = fs::read(orig).map_err(|err| ProgramError::io(orig, &err))?;
    if styles_fonts(&String::from_utf8_lossy(&source)) {
        let fonts = workspace.path("fonts");
        fs::create_dir_all(&fonts).map_err(|err| ProgramError::io(&fonts, &err))?;
        for (name, data) in pltrs_text::bundled_font_files() {
            let file = fonts.join(name);
            fs::write(&file, data).map_err(|err| ProgramError::io(&file, &err))?;
        }
    }
    fs::write(&svg, source).map_err(|err| ProgramError::io(&svg, &err))?;
    let old = Executable::Inkscape
        .version()?
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .is_some_and(|major| major < 1);
    let args: &[&str] = if old {
        &["--without-gui", "f.svg", "--export-png=f.png"]
    } else {
        &["f.svg", "--export-filename=f.png"]
    };
    let output = crate::process::output(
        Command::new("inkscape")
            .args(args)
            .current_dir(&profile)
            .env("DISPLAY", "")
            .env("INKSCAPE_PROFILE_DIR", &profile)
            .stdin(Stdio::null()),
    )
    .map_err(|err| ProgramError::NotFound(format!("Failed to start Inkscape: {err}")))?;
    if !output.status.success() || !png.exists() {
        let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
        return Err(ProgramError::Failed {
            command: pltrs_text::tex::format_command("inkscape", &args),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    fs::rename(&png, dest)
        .or_else(|_| fs::copy(&png, dest).map(drop))
        .map_err(|err| ProgramError::io(dest, &err))
}

/// Whether an SVG document styles text with fonts (a `style` attribute
/// with `font`, `font-size`, `font-weight`, `font-family`, `font-variant`
/// or `font-style`), as text kept as text does (matplotlib's check before
/// handing Inkscape its fonts).
fn styles_fonts(svg: &str) -> bool {
    const PROPERTIES: [&str; 6] = [
        "font:",
        "font-size:",
        "font-weight:",
        "font-family:",
        "font-variant:",
        "font-style:",
    ];
    svg.split("style=\"").skip(1).any(|rest| {
        let value = rest.split('"').next().unwrap_or("");
        PROPERTIES.iter().any(|property| value.contains(property))
    })
}

/// Decodes the GIF file `orig` and writes its first frame to `dest` as an
/// RGBA PNG.
fn gif(orig: &Path, dest: &Path) -> Result<(), ComparisonError> {
    let unreadable = |reason: String| ComparisonError::Unreadable {
        path: orig.to_owned(),
        reason,
    };
    let data = fs::read(orig).map_err(|err| unreadable(err.to_string()))?;
    let image = pltrs_codec::gif::decode(&data).map_err(|err| unreadable(err.to_string()))?;
    let pixels = image.to_rgba8();
    let unwritable = |reason: String| ComparisonError::Unwritable {
        path: dest.to_owned(),
        reason,
    };
    let encoded = ImageRef::rgba8(image.width(), image.height(), pixels.as_flattened())
        .and_then(png::encode)
        .map_err(|err| unwritable(err.to_string()))?;
    fs::write(dest, encoded).map_err(|err| unwritable(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    #[test]
    fn converted_images_sit_beside_their_files() {
        assert_eq!(
            converted_path(Path::new("out/plot.pdf")),
            Path::new("out/plot_pdf.png")
        );
        assert_eq!(
            converted_path(Path::new("a.b.svg")),
            Path::new("a.b_svg.png")
        );
    }

    #[test]
    fn fonts_are_handed_over_for_text_kept_as_text() {
        assert!(styles_fonts(
            r#"<text style="font: 10px 'DejaVu Sans'">x</text>"#
        ));
        assert!(styles_fonts(
            r#"<g style="fill: red"/><text style="opacity: 0.5; font-family: serif">x</text>"#
        ));
        assert!(!styles_fonts(
            r#"<path style="fill: #ff0000; stroke-width: 1"/>"#
        ));
        assert!(!styles_fonts("font-size: 3"), "outside a style attribute");
    }

    #[test]
    fn converters_follow_the_extension() {
        assert_eq!(
            Converter::of(Path::new("a.eps")),
            Some(Converter::Ghostscript)
        );
        assert_eq!(Converter::of(Path::new("a.svg")), Some(Converter::Inkscape));
        assert_eq!(Converter::of(Path::new("a.gif")), Some(Converter::Gif));
        assert_eq!(Converter::of(Path::new("a.png")), None);
        assert_eq!(Converter::of(Path::new("a")), None);
    }

    #[test]
    fn gif_files_convert_without_external_programs() {
        let scratch = Scratch::new("convert-gif");
        let dir = scratch.path("");
        let gif = dir.join("dot.gif");
        let pixels = [[0_u8, 0, 0, 255], [255, 255, 255, 255]];
        let frame = pltrs_codec::gif::GifFrame {
            pixels: pixels.as_flattened(),
            delay: 0,
        };
        let encoded = pltrs_codec::gif::GifEncoder::new()
            .encode(2, 1, &[frame])
            .unwrap();
        fs::write(&gif, encoded).unwrap();
        let png_path = convert(&gif, false).unwrap();
        assert_eq!(png_path, dir.join("dot_gif.png"));
        let image = png::decode(&fs::read(&png_path).unwrap()).unwrap();
        assert_eq!(image.to_rgba8(), pixels.as_flattened());
        // Up to date: not converted again.
        let before = fs::metadata(&png_path).unwrap().modified().unwrap();
        assert_eq!(convert(&gif, false).unwrap(), png_path);
        assert_eq!(fs::metadata(&png_path).unwrap().modified().unwrap(), before);
        assert!(matches!(
            convert(dir.join("missing.gif"), false),
            Err(ComparisonError::MissingOutput(_))
        ));
        fs::write(dir.join("data.txt"), b"x").unwrap();
        assert!(matches!(
            convert(dir.join("data.txt"), false),
            Err(ComparisonError::NotComparable(_))
        ));
    }

    #[test]
    fn pdf_files_convert_with_ghostscript() {
        if Executable::Ghostscript.check().is_err() {
            eprintln!("skipped: Ghostscript is not installed");
            return;
        }
        crate::testing::setup();
        let scratch = Scratch::new("convert-pdf");
        let dir = scratch.path("");
        let fig = crate::figure::Figure::with_size(2.0, 1.0, 100.0);
        fig.subplot();
        let pdf = dir.join("axes.pdf");
        fig.savefig(&pdf).unwrap();
        let png_path = convert(&pdf, false).unwrap();
        let image = png::decode(&fs::read(&png_path).unwrap()).unwrap();
        // 72 dpi, whatever the figure's resolution.
        assert_eq!((image.width(), image.height()), (144, 72));
        assert!(comparable_formats().contains(&SaveFormat::Pdf));
        let hash = get_file_hash(&pdf).unwrap();
        let mut data = fs::read(&pdf).unwrap();
        data.extend_from_slice(Executable::Ghostscript.version().unwrap().as_bytes());
        assert_eq!(hash, sha256_hex(&data));
    }
}
