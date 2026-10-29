//! The distillers of `ps.usedistiller` (matplotlib's `gs_distill` and
//! `xpdf_distill`): Ghostscript's `ps2write` device, or `ps2pdf` then
//! `pdftops`, rewrite a PostScript file in place; EPS output then goes
//! through `pstoeps`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use plt_core::Bbox;
use plt_core::pyfmt;
use plt_render::{Paper, pstoeps};

use super::programs::{Executable, ProgramError, Workspace, path_argument, read, replace, write};

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
    /// The figure's bounding box in points.
    pub(super) bbox: Bbox,
    /// Whether `dvips` turned the page on its side, which `pstoeps` undoes.
    pub(super) rotated: bool,
}

impl Distillation {
    /// The options choosing the page: EPS cropped to its bounding box, a
    /// page the size of the figure (from the origin of the bounding box)
    /// or a named paper size, written `-name<separator>value`.
    fn paper_options(&self, separator: char) -> Vec<OsString> {
        let options = if self.eps {
            vec!["-dEPSCrop".to_owned()]
        } else {
            match self.paper {
                Paper::Figure => vec![
                    format!(
                        "-dDEVICEWIDTHPOINTS{separator}{}",
                        pyfmt::repr(self.bbox.x1())
                    ),
                    format!(
                        "-dDEVICEHEIGHTPOINTS{separator}{}",
                        pyfmt::repr(self.bbox.y1())
                    ),
                ],
                Paper::Named { name, .. } => vec![format!("-sPAPERSIZE{separator}{name}")],
            }
        };
        options.into_iter().map(OsString::from).collect()
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
    Executable::Ghostscript.check()?;
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
    args.extend(distillation.paper_options('='));
    args.push(path_argument("-sOutputFile=", &psfile));
    args.push(file.path().into());
    file.workspace().run("gs", &args)?;
    file.replace_with(&psfile)?;
    if distillation.eps {
        file.to_eps(Some(&distillation.bbox), distillation.rotated)?;
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
    args.extend(distillation.paper_options('#'));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn distillation(eps: bool, paper: &str) -> Distillation {
        Distillation {
            eps,
            paper: paper.parse().unwrap(),
            bbox: Bbox::from_extents(0.0, 0.0, 432.0, 576.0),
            rotated: false,
        }
    }

    #[test]
    fn paper_options_follow_matplotlib() {
        let options = |d: Distillation, separator| -> Vec<String> {
            d.paper_options(separator)
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
    }

    /// Distilling a document with Ghostscript where it is installed, and
    /// matplotlib's error where it is not.
    #[test]
    fn ghostscript_rewrites_the_file() {
        let document = b"%!PS-Adobe-3.0\n%%BoundingBox: 0 0 100 100\n%%EndComments\n\
                         newpath 10 10 moveto 90 90 lineto stroke showpage\n%%EOF\n";
        let file = WorkFile::new(document, 0).unwrap();
        let eps = Distillation {
            bbox: Bbox::from_extents(0.0, 0.0, 100.0, 100.0),
            ..distillation(true, "letter")
        };
        match gs_distill(&file, &eps, 6000) {
            Ok(()) => {
                let distilled = String::from_utf8(file.contents().unwrap()).unwrap();
                assert!(distilled.starts_with(
                    "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\n\
                     %%HiResBoundingBox: 0.000000 0.000000 100.000000 100.000000\n\
                     %%Invocation: path/gs -dBATCH -dNOPAUSE -dSAFER -r6000 -sDEVICE=ps2write \
                     -dEPSCrop -sOutputFile=? ?\n"
                ));
                assert!(distilled.contains("%%CreationDate: D:19700101000000Z00'00'\n"));
                assert!(distilled.ends_with("restore\nshowpage\n%%EOF\n"));
            }
            Err(err) => assert_eq!(err.to_string(), "Failed to find a Ghostscript installation"),
        }
    }
}
