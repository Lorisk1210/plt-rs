use std::process::{Command, Stdio};

use crate::scratch::Scratch;

use super::*;

/// The metadata of matplotlib's documents these tests compare with.
fn metadata() -> Metadata {
    Metadata {
        creator: Some("c".into()),
        producer: Some("p".into()),
        date: Some(Timestamp(946_782_245)),
        title: Some("T".into()),
        ..Metadata::none()
    }
}

/// Whether `pgf.texsystem` can typeset PGF pictures here (matplotlib's
/// `_check_for_pgf`): LaTeX with the `pgf` package.
fn pgf_available() -> bool {
    let texsystem = crate::rc::params().pgf_texsystem.name();
    let found = |program: &str, arg: &str| {
        Command::new(program)
            .arg(arg)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };
    found(texsystem, "--version") && found("kpsewhich", "pgf.sty")
}

#[test]
fn figure_documents_input_the_picture_as_matplotlib_writes_them() {
    let document = figure_document(&metadata(), (3.0, 2.0));
    let expected = [
        r"\documentclass{article}",
        r"\usepackage[pdfinfo={Creator={c},Producer={p},CreationDate={D:20000102030405Z},Title={T}}]{hyperref}",
        r"\usepackage[papersize={3.000000in,2.000000in}, margin=0in]{geometry}",
        r"\usepackage{pgf}",
        &pgf_preamble(&crate::rc::params()),
        r"\begin{document}",
        r"\centering",
        r"\input{figure.pgf}",
        r"\end{document}",
    ]
    .join("\n");
    assert_eq!(document, expected);
}

#[test]
fn absent_entries_stay_out_of_the_pdf_information() {
    assert_eq!(pdfinfo(&Metadata::none()), "");
    let trapped = Metadata {
        trapped: Some(pltrs_render::Trapped::True),
        author: Some("A".into()),
        ..Metadata::none()
    };
    assert_eq!(pdfinfo(&trapped), "Author={A},Trapped={True}");
}

#[test]
fn pages_follow_each_other_at_their_figures_sizes() {
    crate::testing::setup();
    let mut pages = PgfPages::new("pages.pdf", Some(metadata()));
    assert_eq!(pages.page_count(), 0);
    let (first, second) = (
        Figure::with_size(3.0, 2.0, 100.0),
        Figure::with_size(4.0, 5.0, 100.0),
    );
    pages.savefig(&first).unwrap();
    pages.savefig(&second).unwrap();
    assert_eq!(pages.page_count(), 2);
    let source = pages.source();
    let header = [
        document_header(&metadata(), (3.0, 2.0)),
        r"\setlength{\parindent}{0pt}".to_owned(),
        r"\begin{document}%".to_owned(),
    ]
    .join("\n");
    let newpage = r"\newpage\ifdefined\pdfpagewidth\pdfpagewidth\else\pagewidth\fi=4.000000in\ifdefined\pdfpageheight\pdfpageheight\else\pageheight\fi=5.000000in%";
    let expected = format!(
        "{header}{}{newpage}\n{}",
        first.to_pgf().unwrap(),
        second.to_pgf().unwrap()
    );
    assert_eq!(source, expected);
}

#[test]
fn a_document_without_pages_writes_nothing() {
    let scratch = Scratch::new("pgf-pages-empty");
    let dir = scratch.path("");
    let path = dir.join("empty.pdf");
    PgfPages::new(&path, None).finish().unwrap();
    assert!(!path.exists());
}

#[test]
fn converters_take_matplotlibs_arguments() {
    let (pdf, png) = (Path::new("/t/figure.pdf"), Path::new("/t/figure.png"));
    let args = PdfToPng::Pdftocairo.arguments(pdf, png, 100.7);
    assert_eq!(PdfToPng::Pdftocairo.program().unwrap(), "pdftocairo");
    assert_eq!(
        args,
        [
            "-singlefile",
            "-transp",
            "-png",
            "-r",
            "100",
            "/t/figure.pdf",
            "/t/figure"
        ]
    );
    let args = PdfToPng::Ghostscript.arguments(pdf, png, 72.0);
    assert_eq!(args[9], "-sDEVICE=pngalpha");
    assert_eq!(args[10], "-sOutputFile=/t/figure.png");
    assert_eq!(args[11], "-r72");
    assert_eq!(args[12], "/t/figure.pdf");
}

#[test]
fn latex_typesets_figures_and_pages() {
    crate::testing::setup();
    if !pgf_available() {
        eprintln!("skipped: LaTeX cannot typeset PGF pictures here");
        return;
    }
    let fig = Figure::with_size(2.0, 1.5, 100.0);
    fig.subplot().set_title("typeset");
    let pdf = fig.to_pgf_pdf().unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let scratch = Scratch::new("pgf-pages");
    let dir = scratch.path("");
    let mut pages = PgfPages::new(dir.join("pages.pdf"), None);
    pages.savefig(&fig).unwrap();
    pages.savefig(&fig).unwrap();
    pages.finish().unwrap();
    assert!(
        std::fs::read(dir.join("pages.pdf"))
            .unwrap()
            .starts_with(b"%PDF")
    );
    if PdfToPng::find().is_ok() {
        let png = fig.to_pgf_png().unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }
}
