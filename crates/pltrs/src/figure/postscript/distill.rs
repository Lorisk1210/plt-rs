//! The distillers of `ps.usedistiller` (matplotlib's `gs_distill` and
//! `xpdf_distill`): Ghostscript's `ps2write` device, or `ps2pdf` then
//! `pdftops`, rewrite a PostScript file in place; EPS output then goes
//! through `pstoeps`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use pltrs_core::Bbox;
use pltrs_core::pyfmt;
use pltrs_render::{Paper, pstoeps};

use super::programs::{Executable, ProgramError, Workspace, path_argument, read, replace, write};
use crate::rc::Distiller;

/// A PostScript file being distilled in a workspace (matplotlib's
/// `tmpfile`, `tmp.ps` in a temporary directory).
pub(super) struct WorkFile {
    workspace: Workspace,
    path: PathBuf,
}

impl WorkFile {
    /// Writes `document` to `tmp.ps` in a fresh workspace whose programs
    /// date what they write `source_date`.
    pub(super) fn new(document: &[u8], source_date: i64) -> Result<Self, ProgramError> {
        let workspace = Workspace::new(source_date)?;
        let path = workspace.path("tmp.ps");
        write(&path, document)?;
        Ok(Self { workspace, path })
    }

    /// The file.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// The workspace its programs run in.
    pub(super) fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// What the file holds now.
    pub(super) fn contents(&self) -> Result<Vec<u8>, ProgramError> {
        read(&self.path)
    }

    /// Replaces the file by `from` (`shutil.move`).
    pub(super) fn replace_with(&self, from: &Path) -> Result<(), ProgramError> {
        replace(from, &self.path)
    }

    /// `pstoeps`: the file as Encapsulated PostScript.
    fn to_eps(&self, bbox: Option<&Bbox>, rotated: bool) -> Result<(), ProgramError> {
        write(&self.path, &pstoeps(&self.contents()?, bbox, rotated))
    }
}

/// What a distiller is told about the document (the arguments of
/// `gs_distill` and `xpdf_distill`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Distillation {
    /// Encapsulated PostScript, cropped to its bounding box.
    pub(super) eps: bool,
    /// The paper (`ptype`).
    pub(super) paper: Paper,
    /// The figure's bounding box in points: what a page the size of the
    /// figure spans, and what EPS output declares (`None` keeps what the
    /// distiller writes).
    pub(super) bbox: Option<Bbox>,
    /// Whether `dvips` turned the page on its side, which `pstoeps` undoes.
    pub(super) rotated: bool,
}

impl Distillation {
    /// The options choosing the page: EPS cropped to its bounding box, a
    /// page the size of the figure (from the origin of the bounding box)
    /// or a named paper size, written `-name<separator>value`.
    fn paper_options(&self, separator: char) -> Result<Vec<OsString>, ProgramError> {
        let options = if self.eps {
            vec!["-dEPSCrop".to_owned()]
        } else {
            match self.paper {
                Paper::Figure => {
                    let bbox = self.bbox.ok_or(ProgramError::MissingBbox)?;
                    vec![
                        format!("-dDEVICEWIDTHPOINTS{separator}{}", pyfmt::repr(bbox.x1())),
                        format!("-dDEVICEHEIGHTPOINTS{separator}{}", pyfmt::repr(bbox.y1())),
                    ]
                }
                Paper::Named { name, .. } => vec![format!("-sPAPERSIZE{separator}{name}")],
            }
        };
        Ok(options.into_iter().map(OsString::from).collect())
    }
}

/// `gs_distill`: Ghostscript's `ps2write` device at `resolution` dpi
/// (`ps.distiller.res`) rewrites the file, which becomes EPS again with
/// the original bounding box for EPS output.
///
/// # Errors
///
/// When Ghostscript is missing ("Failed to find a Ghostscript
/// installation") or fails, or the files cannot be moved.
pub(super) fn gs_distill(
    file: &WorkFile,
    distillation: &Distillation,
    resolution: i64,
) -> Result<(), ProgramError> {
    let gs = Executable::Ghostscript.check()?;
    let mut psfile = file.path().as_os_str().to_owned();
    psfile.push(".ps");
    let psfile = PathBuf::from(psfile);
    let mut args: Vec<OsString> = [
        "-dBATCH",
        "-dNOPAUSE",
        "-dSAFER",
        &format!("-r{resolution}"),
        "-sDEVICE=ps2write",
    ]
    .map(OsString::from)
    .into();
    args.extend(distillation.paper_options('=')?);
    args.push(path_argument("-sOutputFile=", &psfile));
    args.push(file.path().into());
    file.workspace().run(gs, &args)?;
    file.replace_with(&psfile)?;
    if distillation.eps {
        file.to_eps(distillation.bbox.as_ref(), distillation.rotated)?;
    }
    Ok(())
}

/// `xpdf_distill`: `ps2pdf` makes a PDF file of the document (images
/// compressed without loss, pages not rotated) and `pdftops` PostScript
/// level 3 of that, on paper matching its pages; EPS output becomes EPS
/// again with the bounding box `pdftops` gives.
///
/// # Errors
///
/// When Ghostscript or `pdftops` is missing or too old, `ps2pdf` or
/// `pdftops` fails, or the files cannot be moved.
pub(super) fn xpdf_distill(
    file: &WorkFile,
    distillation: &Distillation,
) -> Result<(), ProgramError> {
    Executable::Ghostscript.check()?;
    Executable::Pdftops.check()?;
    let directory = file.workspace().subdirectory()?;
    let pdf = directory.path().join("tmp.pdf");
    let ps = directory.path().join("tmp.ps");
    // Options are written `-name#value`, which Ghostscript on Windows
    // needs.
    let mut args: Vec<OsString> = [
        "-dSAFER",
        "-dAutoFilterColorImages#false",
        "-dAutoFilterGrayImages#false",
        "-sAutoRotatePages#None",
        "-sGrayImageFilter#FlateEncode",
        "-sColorImageFilter#FlateEncode",
    ]
    .map(OsString::from)
    .into();
    args.extend(distillation.paper_options('#')?);
    args.extend([file.path().into(), pdf.clone().into()]);
    file.workspace().run("ps2pdf", &args)?;
    let args = ["-paper", "match", "-level3"]
        .map(OsString::from)
        .into_iter()
        .chain([pdf.into(), ps.clone().into()])
        .collect::<Vec<_>>();
    file.workspace().run("pdftops", &args)?;
    file.replace_with(&ps)?;
    if distillation.eps {
        file.to_eps(None, false)?;
    }
    Ok(())
}

/// Runs `distill` as a save runs a distiller (matplotlib's
/// `_try_distill`): a program it needs that is missing logs matplotlib's
/// warning, and the file is left as written.
///
/// # Errors
///
/// When the distiller fails otherwise.
pub(super) fn try_distill(
    distill: impl FnOnce() -> Result<(), ProgramError>,
) -> Result<(), ProgramError> {
    match distill() {
        Err(ProgramError::NotFound(missing)) => {
            crate::warnings::log(
                "backends.backend_ps",
                &format!("{missing}.  Distillation step skipped."),
            );
            Ok(())
        }
        done => done,
    }
}

/// How a PostScript file is distilled (the arguments of matplotlib's
/// `gs_distill` and `xpdf_distill` but the file).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistillOptions {
    /// Encapsulated PostScript, cropped to its bounding box.
    pub eps: bool,
    /// The paper (`ptype`).
    pub paper: Paper,
    /// The figure's bounding box in points: what a page the size of the
    /// figure spans, and what Ghostscript's EPS output declares (`None`
    /// keeps what the distiller writes).
    pub bbox: Option<Bbox>,
    /// Whether `dvips` turned the page on its side, which Ghostscript's
    /// EPS output undoes.
    pub rotated: bool,
}

/// Distills the PostScript file at `path` in place with `distiller`
/// (matplotlib's `gs_distill` and `xpdf_distill`); the programs date what
/// they write as `SOURCE_DATE_EPOCH` says, else now. `Distiller::Off`
/// leaves the file as it is.
///
/// # Errors
///
/// When a program is missing or fails, a page the size of the figure is
/// asked for without its bounding box, or the file cannot be read or
/// written.
pub fn distill_file(
    path: &Path,
    distiller: Distiller,
    options: &DistillOptions,
) -> Result<(), ProgramError> {
    if distiller == Distiller::Off {
        return Ok(());
    }
    let source_date = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|epoch| epoch.trim().parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| {
                    i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
                })
        });
    let file = WorkFile::new(&read(path)?, source_date)?;
    let distillation = Distillation {
        eps: options.eps,
        paper: options.paper,
        bbox: options.bbox,
        rotated: options.rotated,
    };
    if distiller == Distiller::Xpdf {
        xpdf_distill(&file, &distillation)?;
    } else {
        gs_distill(&file, &distillation, crate::rc::params().ps_distiller_res)?;
    }
    write(path, &file.contents()?)
}

/// `document` with the program of its `%%Invocation` line written `gs`:
/// Ghostscript writes `path/gs` when it was started by an absolute path (a
/// wrapper script), `gs` when by name, and on Windows `gswin64c` followed
/// by the arguments its build adds for the display
/// (`-dDisplayFormat=... -dDisplayResolution=...`), before matplotlib's.
#[cfg(test)]
pub(super) fn invoked_as_gs(document: &str) -> String {
    const INVOCATION: &str = "%%Invocation: ";
    let program = document.find(INVOCATION).and_then(|at| {
        let start = at + INVOCATION.len();
        document[start..]
            .find(" -dBATCH")
            .map(|end| (start, start + end))
    });
    match program {
        Some((start, end)) => format!("{}gs{}", &document[..start], &document[end..]),
        None => document.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distillation(eps: bool, paper: &str) -> Distillation {
        Distillation {
            eps,
            paper: paper.parse().unwrap(),
            bbox: Some(Bbox::from_extents(0.0, 0.0, 432.0, 576.0)),
            rotated: false,
        }
    }

    #[test]
    fn paper_options_follow_matplotlib() {
        let options = |d: Distillation, separator| -> Vec<String> {
            d.paper_options(separator)
                .unwrap()
                .into_iter()
                .map(|o| o.into_string().unwrap())
                .collect()
        };
        assert_eq!(options(distillation(true, "letter"), '='), ["-dEPSCrop"]);
        assert_eq!(
            options(distillation(false, "figure"), '='),
            ["-dDEVICEWIDTHPOINTS=432.0", "-dDEVICEHEIGHTPOINTS=576.0"]
        );
        assert_eq!(options(distillation(false, "a4"), '#'), ["-sPAPERSIZE#a4"]);
        let no_bbox = Distillation {
            bbox: None,
            ..distillation(false, "figure")
        };
        assert!(matches!(
            no_bbox.paper_options('='),
            Err(ProgramError::MissingBbox)
        ));
    }

    /// Files are distilled in place, or left alone without a distiller.
    #[test]
    fn files_are_distilled_in_place() {
        let document = b"%!PS-Adobe-3.0\n%%BoundingBox: 0 0 100 100\n%%EndComments\n\
                         newpath 10 10 moveto 90 90 lineto stroke showpage\n%%EOF\n";
        let workspace = Workspace::new(0).unwrap();
        let path = workspace.path("figure.ps");
        write(&path, document).unwrap();
        let options = DistillOptions {
            eps: true,
            paper: "letter".parse().unwrap(),
            bbox: None,
            rotated: false,
        };
        distill_file(&path, Distiller::Off, &options).unwrap();
        assert_eq!(read(&path).unwrap(), document);
        match distill_file(&path, Distiller::Ghostscript, &options) {
            Ok(()) => assert!(
                read(&path)
                    .unwrap()
                    .starts_with(b"%!PS-Adobe-3.0 EPSF-3.0\n")
            ),
            Err(err) => assert_eq!(err.to_string(), "Failed to find a Ghostscript installation"),
        }
    }

    /// Distilling a document with Ghostscript where it is installed, and
    /// matplotlib's error where it is not.
    #[test]
    fn ghostscript_rewrites_the_file() {
        let document = b"%!PS-Adobe-3.0\n%%BoundingBox: 0 0 100 100\n%%EndComments\n\
                         newpath 10 10 moveto 90 90 lineto stroke showpage\n%%EOF\n";
        let file = WorkFile::new(document, 0).unwrap();
        let eps = Distillation {
            bbox: Some(Bbox::from_extents(0.0, 0.0, 100.0, 100.0)),
            ..distillation(true, "letter")
        };
        match gs_distill(&file, &eps, 6000) {
            Ok(()) => {
                // Its date follows `SOURCE_DATE_EPOCH` only in builds that
                // read it (Debian's), else the clock.
                let distilled =
                    invoked_as_gs(&String::from_utf8(file.contents().unwrap()).unwrap());
                assert!(distilled.starts_with(
                    "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\n\
                     %%HiResBoundingBox: 0.000000 0.000000 100.000000 100.000000\n\
                     %%Invocation: gs -dBATCH -dNOPAUSE -dSAFER -r6000 -sDEVICE=ps2write \
                     -dEPSCrop -sOutputFile=? ?\n"
                ));
                assert!(distilled.contains("\n%%CreationDate: D:"));
                assert!(distilled.ends_with("restore\nshowpage\n%%EOF\n"));
            }
            Err(err) => assert_eq!(err.to_string(), "Failed to find a Ghostscript installation"),
        }
    }
}
