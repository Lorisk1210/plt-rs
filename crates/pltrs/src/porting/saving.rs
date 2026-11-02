//! Saving figures and the output formats.
//!
//! [`Figure::savefig`] picks the format from the file extension, as
//! matplotlib does: PNG, JPEG, TIFF, GIF, WebP and raw RGBA pixels; SVG
//! (and compressed SVGZ), PDF, PostScript and EPS documents; and PGF
//! pictures for LaTeX. The keywords of `savefig` are the fields of a
//! [`SaveOptions`], unset ones following the `savefig.*` rcParams.
//!
//! There are no file-like objects. Instead of saving into a `BytesIO`,
//! render to memory with [`Figure::to_png`], [`Figure::to_svg`],
//! [`Figure::to_pdf`], ... or any format with [`Figure::to_format`].
//! Raster and binary formats return bytes, text formats (SVG, PS, EPS,
//! PGF) a `String`.
//!
//! Format-specific settings stay rcParams, as in matplotlib
//! (`pdf.fonttype`, `svg.fonttype`, `ps.papersize`, `pdf.use14corefonts`,
//! ...). Documents carry pltrs as creator and the current date, or the
//! date in `SOURCE_DATE_EPOCH` for reproducible builds.
//!
//! Like matplotlib, PostScript output runs external programs where asked
//! to: the distiller of `ps.usedistiller` (Ghostscript's `ps2write`, or
//! `ps2pdf` and `pdftops` for `xpdf`) at `ps.distiller.res`, and with
//! `text.usetex` LaTeX with `psfrag`, `dvips` and Ghostscript. They run in
//! a temporary directory removed afterwards, dated as the document is (as
//! `SOURCE_DATE_EPOCH`); a missing program fails the save with
//! matplotlib's message ([`ProgramError`], [`check_distiller`]). Where
//! matplotlib finds no Ghostscript or `pdftops`, it logs that message and
//! leaves the document undistilled instead; pltrs, which logs nothing,
//! fails.
//!
//! # Equivalences
//!
//! | matplotlib                                         | pltrs                                          |
//! |----------------------------------------------------|------------------------------------------------|
//! | `fig.savefig("plot.png")`, `plt.savefig(...)`      | [`Figure::savefig`], [`crate::pyplot::savefig`] |
//! | `fig.savefig(path, dpi=200)`                       | [`Figure::savefig_with`] and [`SaveOptions::dpi`] |
//! | `bbox_inches="tight"`, `pad_inches=0.05`           | [`SaveOptions::bbox_inches`]`: Some(BboxInches::Tight)`, [`SaveOptions::pad_inches`]`: Some(0.05.into())` |
//! | `pad_inches="layout"`                              | [`PadInches::Layout`]                          |
//! | `bbox_inches=Bbox([[x0, y0], [x1, y1]])`           | [`BboxInches::Inches`]                         |
//! | `transparent=True`, `facecolor=`, `edgecolor=`     | [`SaveOptions::transparent`], [`SaveOptions::facecolor`], [`SaveOptions::edgecolor`] |
//! | `format="pdf"`                                     | [`SaveOptions::format`]`: Some(SaveFormat::Pdf)`, or `"pdf".parse::<SaveFormat>()?` |
//! | `metadata={"Title": ..., "Author": ...}`           | [`SaveOptions::metadata`]                      |
//! | `metadata={"Title": ..., "Software": None}` (PNG)  | [`Metadata::png`]`: PngMetadata::default().with("Title", ..).without("Software")` ([`PngMetadata`]) |
//! | `pil_kwargs={"quality": 90}`                       | [`SaveOptions::jpeg_quality`]                  |
//! | `fig.savefig(io.BytesIO(), format="png")`          | [`Figure::to_png`], [`Figure::to_png_with`]    |
//! | the same for SVG, PDF, PS, EPS, PGF                | [`Figure::to_svg`], [`Figure::to_pdf`], [`Figure::to_ps`], [`Figure::to_eps`], [`Figure::to_pgf`] |
//! | JPEG, TIFF, GIF, WebP, raw, SVGZ into a buffer     | [`Figure::to_format`] with a [`SaveFormat`]    |
//! | `fig.canvas.get_supported_filetypes()`             | the variants of [`SaveFormat`]                 |
//! | `PdfPages("doc.pdf")`, `pdf.savefig(fig)`          | [`PdfPages::new`], [`PdfPages::savefig`], then [`PdfPages::save`] or [`PdfPages::finish`] |
//! | `pdf.attach_note(text)`, `pdf.infodict()`          | [`PdfPages::attach_note`], [`PdfPages::metadata_mut`] |
//! | `fig.canvas.draw()`, `fig.canvas.buffer_rgba()`    | [`FigureCanvas::draw`], [`FigureCanvas::buffer_rgba`] via [`Figure::canvas`] |
//! | `fig.canvas.device_pixel_ratio`, `_set_device_pixel_ratio(2)` | [`FigureCanvas::device_pixel_ratio`], [`FigureCanvas::set_device_pixel_ratio`]; saving keeps the resolution the figure was made at |
//! | `fig.get_tightbbox()`                              | [`Figure::tight_bbox`]                         |
//! | `mpl.use("pgf")` and `savefig("fig.pgf")`          | [`Figure::to_pgf`], or a `.pgf` file name      |
//! | `savefig("fig.pdf", backend="pgf")`, `"fig.png"`   | [`Figure::to_pgf_pdf`], [`Figure::to_pgf_png`]: LaTeX (`pgf.texsystem`) typesets the picture |
//! | `backend_pgf.PdfPages("doc.pdf")`                  | [`PgfPages::new`], [`PgfPages::savefig`], [`PgfPages::finish`] |
//! | `backend_pgf.make_pdf_to_png_converter()`          | [`PdfToPng::find`]                             |
//! | `FigureCanvasAgg.print_to_buffer()`, `print_raw`   | [`Figure::render_raster`], [`Figure::to_format`]`(SaveFormat::Raw, ..)` |
//! | `RendererAgg`, `RendererPdf`, `RendererSVG`, `RendererPS`, `RendererPgf` | `pltrs_render`'s [`RasterRenderer`], [`PdfRenderer`], [`SvgRenderer`], [`PsRenderer`], [`PgfRenderer`], all [`Renderer`]s |
//! | `backend_pdf.PdfFile`: `newPage`, `writeObject`, `reserveObject`, ... | [`PdfRenderer`]: [`PdfRenderer::new_page`], [`PdfRenderer::write_object`], [`PdfRenderer::reserve_object`], ... (snake case) |
//! | `backend_pdf.Name`, `Reference`, `Verbatim`, `pdfRepr` | [`PdfObject`]`::Name`, `::Ref`, `::Raw`, [`PdfObject::repr`] |
//! | `backend_svg.XMLWriter`                            | `pltrs_render::svg::xml::XmlWriter`            |
//! | `savefig("fig.avif")`                              | not written by pltrs: matplotlib (and pltrs (Python)) save AVIF through Pillow |
//! | `rcParams["savefig.dpi"]`, `["pdf.fonttype"]`, ... | [`crate::rc::set`] with the same keys          |
//! | animations                                         | see [`super::interaction`]                     |
//!
//! # Files and bytes
//!
//! ```python
//! fig.savefig(tmp / "plot.png")
//! fig.savefig(tmp / "plot.svg")
//! buffer = io.BytesIO()
//! fig.savefig(buffer, format="png", dpi=200)
//! fig.savefig(io.BytesIO(), format="pdf", bbox_inches="tight", transparent=True)
//! fig.savefig(io.BytesIO(), format="jpeg", pil_kwargs={"quality": 90})
//! ```
//!
//! ```
//! use pltrs::figure::{BboxInches, Figure, SaveFormat, SaveOptions};
//! use pltrs::image::imread_bytes;
//!
//! let fig = Figure::with_size(6.4, 4.8, 100.0);
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//!
//! let dir = std::env::temp_dir().join(format!("pltrs-porting-{}", std::process::id()));
//! std::fs::create_dir_all(&dir)?;
//! fig.savefig(dir.join("plot.png"))?;
//! fig.savefig(dir.join("plot.svg"))?;
//! assert!(std::fs::read(dir.join("plot.png"))?.starts_with(b"\x89PNG"));
//! assert!(std::fs::read_to_string(dir.join("plot.svg"))?.contains("<svg"));
//! std::fs::remove_dir_all(&dir)?;
//!
//! let png = fig.to_png_with(SaveOptions { dpi: Some(200.0), ..SaveOptions::default() })?;
//! assert_eq!(imread_bytes(&png)?.shape(), (960, 1280));
//!
//! let tight = SaveOptions {
//!     bbox_inches: Some(BboxInches::Tight),
//!     transparent: Some(true),
//!     ..SaveOptions::default()
//! };
//! let pdf = fig.to_pdf_with(tight)?;
//! assert!(pdf.starts_with(b"%PDF"));
//!
//! let jpeg = SaveOptions { jpeg_quality: Some(90), ..SaveOptions::default() };
//! let bytes = fig.to_format("jpg".parse::<SaveFormat>()?, jpeg)?;
//! assert!(bytes.starts_with(&[0xFF, 0xD8]));
//! assert!("docx".parse::<SaveFormat>().is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Several figures in one PDF
//!
//! ```python
//! from matplotlib.backends.backend_pdf import PdfPages
//! with PdfPages(tmp / "report.pdf") as pdf:
//!     for n in range(1, 4):
//!         fig = plt.figure(figsize=(2, 1.5))
//!         fig.add_subplot().plot([0, 1], [0, n])
//!         pdf.savefig(fig)
//!     pdf.infodict()["Title"] = "Report"
//! ```
//!
//! ```
//! use pltrs::figure::{Figure, PdfPages};
//!
//! let mut pages = PdfPages::new()?;
//! for n in 1..=3 {
//!     let fig = Figure::with_size(2.0, 1.5, 72.0);
//!     fig.subplot().plot([0.0, 1.0], [0.0, f64::from(n)])?;
//!     pages.savefig(&fig)?;
//! }
//! pages.metadata_mut().title = Some("Report".into());
//! assert_eq!(pages.page_count(), 3);
//! let pdf = pages.finish()?; // or `pages.save(path)?`
//! assert!(pdf.starts_with(b"%PDF"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Pixels
//!
//! ```python
//! fig.canvas.draw()
//! rgba = np.asarray(fig.canvas.buffer_rgba())
//! assert rgba.shape == (240, 320, 4)
//! ```
//!
//! ```
//! use pltrs::figure::Figure;
//!
//! let fig = Figure::with_size(3.2, 2.4, 100.0);
//! fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
//! let canvas = fig.canvas();
//! canvas.draw()?;
//! let rgba = canvas.buffer_rgba()?;
//! assert_eq!(rgba.data().len(), 320 * 240 * 4);
//! assert_eq!(rgba.pixel(0, 0).map(|p| p[3]), Some(255), "an opaque background");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure::savefig`]: crate::figure::Figure::savefig
//! [`Figure::savefig_with`]: crate::figure::Figure::savefig_with
//! [`Figure::to_png`]: crate::figure::Figure::to_png
//! [`Figure::to_png_with`]: crate::figure::Figure::to_png_with
//! [`Figure::to_svg`]: crate::figure::Figure::to_svg
//! [`Figure::to_pdf`]: crate::figure::Figure::to_pdf
//! [`Figure::to_ps`]: crate::figure::Figure::to_ps
//! [`Figure::to_eps`]: crate::figure::Figure::to_eps
//! [`ProgramError`]: crate::figure::ProgramError
//! [`check_distiller`]: crate::figure::check_distiller
//! [`Figure::to_pgf`]: crate::figure::Figure::to_pgf
//! [`Figure::to_format`]: crate::figure::Figure::to_format
//! [`Figure::canvas`]: crate::figure::Figure::canvas
//! [`Figure::tight_bbox`]: crate::figure::Figure::tight_bbox
//! [`SaveOptions`]: crate::figure::SaveOptions
//! [`SaveOptions::dpi`]: crate::figure::SaveOptions::dpi
//! [`SaveOptions::bbox_inches`]: crate::figure::SaveOptions::bbox_inches
//! [`SaveOptions::pad_inches`]: crate::figure::SaveOptions::pad_inches
//! [`SaveOptions::transparent`]: crate::figure::SaveOptions::transparent
//! [`SaveOptions::facecolor`]: crate::figure::SaveOptions::facecolor
//! [`SaveOptions::edgecolor`]: crate::figure::SaveOptions::edgecolor
//! [`SaveOptions::format`]: crate::figure::SaveOptions::format
//! [`SaveOptions::metadata`]: crate::figure::SaveOptions::metadata
//! [`Metadata::png`]: crate::figure::Metadata::png
//! [`PngMetadata`]: crate::figure::PngMetadata
//! [`SaveOptions::jpeg_quality`]: crate::figure::SaveOptions::jpeg_quality
//! [`BboxInches::Inches`]: crate::figure::BboxInches::Inches
//! [`PadInches::Layout`]: crate::figure::PadInches::Layout
//! [`SaveFormat`]: crate::figure::SaveFormat
//! [`PdfPages::new`]: crate::figure::PdfPages::new
//! [`PdfPages::savefig`]: crate::figure::PdfPages::savefig
//! [`PdfPages::save`]: crate::figure::PdfPages::save
//! [`PdfPages::finish`]: crate::figure::PdfPages::finish
//! [`PdfPages::attach_note`]: crate::figure::PdfPages::attach_note
//! [`PdfPages::metadata_mut`]: crate::figure::PdfPages::metadata_mut
//! [`FigureCanvas::draw`]: crate::figure::FigureCanvas::draw
//! [`FigureCanvas::buffer_rgba`]: crate::figure::FigureCanvas::buffer_rgba
//! [`FigureCanvas::device_pixel_ratio`]: crate::figure::FigureCanvas::device_pixel_ratio
//! [`FigureCanvas::set_device_pixel_ratio`]: crate::figure::FigureCanvas::set_device_pixel_ratio
//! [`Figure::to_pgf_pdf`]: crate::figure::Figure::to_pgf_pdf
//! [`Figure::to_pgf_png`]: crate::figure::Figure::to_pgf_png
//! [`PgfPages::new`]: crate::figure::PgfPages::new
//! [`PgfPages::savefig`]: crate::figure::PgfPages::savefig
//! [`PgfPages::finish`]: crate::figure::PgfPages::finish
//! [`PdfToPng::find`]: crate::figure::PdfToPng::find
//! [`Figure::render_raster`]: crate::figure::Figure::render_raster
//! [`RasterRenderer`]: pltrs_render::RasterRenderer
//! [`PdfRenderer`]: pltrs_render::PdfRenderer
//! [`SvgRenderer`]: pltrs_render::SvgRenderer
//! [`PsRenderer`]: pltrs_render::PsRenderer
//! [`PgfRenderer`]: pltrs_render::PgfRenderer
//! [`Renderer`]: pltrs_render::Renderer
//! [`PdfRenderer::new_page`]: pltrs_render::PdfRenderer::new_page
//! [`PdfRenderer::write_object`]: pltrs_render::PdfRenderer::write_object
//! [`PdfRenderer::reserve_object`]: pltrs_render::PdfRenderer::reserve_object
//! [`PdfObject`]: pltrs_render::PdfObject
//! [`PdfObject::repr`]: pltrs_render::PdfObject::repr
