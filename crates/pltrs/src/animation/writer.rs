//! Writing a figure's frames as the program draws them (matplotlib's
//! `MovieWriter` classes, their `setup`, `grab_frame` and `finish`, and
//! the `saving` context): no animation object drives the figure, the
//! program changes it and grabs a frame whenever it wants one.

use std::fmt;
use std::path::{Path, PathBuf};

use pltrs_core::Bbox;
use pltrs_core::numeric::floor_div;
use pltrs_render::RgbaImage;

use super::html::{FrameDirectory, PlayMode, player_page};
use super::movie::FfmpegPipe;
use super::save::{encode_apng, encode_gif, encode_webp};
use super::{AnimationError, MovieOptions};
use crate::figure::{BboxInches, Figure, SaveError, SaveOptions};
use crate::ticker::pyfmt;

/// The frames per second of a writer given none (matplotlib's
/// `MovieWriter` default).
const DEFAULT_FPS: f64 = 5.0;

/// Where grabbed frames go until the file is finished.
enum Sink {
    /// Kept to be encoded as an animated PNG, GIF or WebP.
    Images(ImageFormat, Vec<RgbaImage>),
    /// Written as files beside an HTML page (matplotlib's `HTMLWriter`).
    Html(FrameDirectory),
    /// Piped to ffmpeg (matplotlib's `FFMpegWriter`).
    Ffmpeg(FfmpegPipe),
}

/// The animated image formats pltrs encodes itself (matplotlib's
/// `PillowWriter`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImageFormat {
    Apng,
    Gif,
    Webp,
}

/// Records frames of a figure into an animation file (matplotlib's
/// `MovieWriter`: `setup(fig, outfile, dpi)`, `grab_frame()` and
/// `finish()`). The format follows the file's extension as
/// [`FuncAnimation::save`](super::FuncAnimation::save) chooses it: `.png`
/// or `.apng`, `.gif` and `.webp` are encoded by pltrs (matplotlib's
/// `PillowWriter`), `.html` or `.htm` is a page of matplotlib's player
/// with its frames beside it (`HTMLWriter`), and any other extension is a
/// video ffmpeg encodes as frames come (`FFMpegWriter`).
///
/// ```python
/// writer = PillowWriter(fps=10)
/// with writer.saving(fig, "wave.gif", dpi=50):
///     for phase in np.linspace(0, 1, 5):
///         line.set_ydata(np.sin(x + phase))
///         writer.grab_frame()
/// ```
///
/// ```
/// use pltrs::animation::{MovieOptions, MovieWriter};
/// use pltrs::prelude::*;
///
/// let fig = Figure::with_size(2.0, 2.0, 50.0);
/// let x = linspace(0.0, 6.0, 20);
/// let line = fig.subplot().plot(&x, x.iter().map(|v| v.sin()).collect::<Vec<_>>())?;
/// let path = std::env::temp_dir().join(format!("pltrs-writer-doc-{}.gif", std::process::id()));
/// let options = MovieOptions { fps: Some(10.0), ..MovieOptions::default() };
/// let mut writer = MovieWriter::new(&fig, &path, Some(50.0), options)?;
/// assert_eq!(writer.frame_size(), (100, 100));
/// for phase in linspace(0.0, 1.0, 5) {
///     line.set_ydata(x.iter().map(|v| (v + phase).sin()).collect::<Vec<_>>());
///     writer.grab_frame()?;
/// }
/// assert_eq!(writer.frame_count(), 5);
/// writer.finish()?;
/// assert!(std::fs::read(&path)?.starts_with(b"GIF89a"));
/// std::fs::remove_file(path)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct MovieWriter {
    figure: Figure,
    path: PathBuf,
    dpi: f64,
    fps: f64,
    size: (usize, usize),
    frames: usize,
    sink: Sink,
}

impl fmt::Debug for MovieWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MovieWriter")
            .field("figure", &self.figure)
            .field("path", &self.path)
            .field("dpi", &self.dpi)
            .field("fps", &self.fps)
            .field("frames", &self.frames)
            .finish_non_exhaustive()
    }
}

impl MovieWriter {
    /// A writer of frames of `figure` into `path` (matplotlib's `setup`),
    /// rendered at `dpi` (the figure's own when `None`) and shown at
    /// [`MovieOptions::fps`] frames per second (5 when unset); the other
    /// options are ffmpeg's. A video's ffmpeg starts at once.
    ///
    /// # Errors
    ///
    /// When `path` has no extension, the frame directory of an HTML page
    /// cannot be made, or ffmpeg cannot be started.
    pub fn new(
        figure: &Figure,
        path: impl AsRef<Path>,
        dpi: Option<f64>,
        options: MovieOptions,
    ) -> Result<Self, AnimationError> {
        let path = path.as_ref().to_owned();
        let dpi = dpi.unwrap_or_else(|| figure.dpi());
        let fps = options.fps.unwrap_or(DEFAULT_FPS);
        let (width, height) = figure.size_inches();
        let canvas =
            Figure::raster_canvas_sized(width, height, dpi).map_err(AnimationError::Save)?;
        let image = canvas.to_image();
        let size = (image.width(), image.height());
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let sink = match extension.as_str() {
            "png" | "apng" => Sink::Images(ImageFormat::Apng, Vec::new()),
            "gif" => Sink::Images(ImageFormat::Gif, Vec::new()),
            "webp" => Sink::Images(ImageFormat::Webp, Vec::new()),
            "html" | "htm" => {
                let stem = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .ok_or_else(|| AnimationError::NotHtml(path.clone()))?;
                Sink::Html(FrameDirectory::new(stem, &path)?)
            }
            "" => return Err(AnimationError::UnsupportedFormat(extension)),
            _ => Sink::Ffmpeg(FfmpegPipe::spawn(&path, size, fps, &options)?),
        };
        Ok(Self {
            figure: figure.clone(),
            path,
            dpi,
            fps,
            size,
            frames: 0,
            sink,
        })
    }

    /// The size of a frame in pixels (matplotlib's `frame_size`).
    pub fn frame_size(&self) -> (usize, usize) {
        self.size
    }

    /// The number of frames grabbed so far.
    pub fn frame_count(&self) -> usize {
        self.frames
    }

    /// Renders the figure as it is now and adds it as the next frame
    /// (matplotlib's `grab_frame`): the whole figure, as `savefig` draws
    /// it at the writer's resolution.
    ///
    /// # Errors
    ///
    /// When the figure cannot be rendered, or the frame cannot be written.
    pub fn grab_frame(&mut self) -> Result<(), AnimationError> {
        let rendered = || render(&self.figure, self.dpi).map_err(AnimationError::Save);
        match &mut self.sink {
            Sink::Images(_, images) => images.push(rendered()?),
            Sink::Html(frames) => frames
                .write(&self.figure, self.frames)
                .map_err(AnimationError::Save)?,
            Sink::Ffmpeg(pipe) => pipe.write(&rendered()?).map_err(AnimationError::Save)?,
        }
        self.frames += 1;
        Ok(())
    }

    /// Writes the file from the frames grabbed (matplotlib's `finish`):
    /// encodes the animated image, writes the HTML page, or waits for
    /// ffmpeg to finish the video.
    ///
    /// # Errors
    ///
    /// When the frames cannot be encoded (an animated image needs one
    /// frame at least), the file cannot be written, or ffmpeg fails.
    pub fn finish(self) -> Result<(), AnimationError> {
        let interval_ms = 1000.0 / self.fps;
        let write = |data: &[u8]| {
            std::fs::write(&self.path, data).map_err(|err| AnimationError::Save(SaveError::Io(err)))
        };
        match self.sink {
            Sink::Images(ImageFormat::Apng, images) => {
                let delay = interval_ms.trunc().clamp(0.0, f64::from(u16::MAX)) as u16;
                write(&encode_apng(&images, delay)?)
            }
            Sink::Images(ImageFormat::Gif, images) => write(&encode_gif(&images, interval_ms)?),
            Sink::Images(ImageFormat::Webp, images) => write(&encode_webp(&images, interval_ms)?),
            Sink::Html(frames) => {
                let page = player_page(
                    &frames.fill_frames(self.frames),
                    self.frames,
                    &html_interval(self.fps),
                    PlayMode::Loop,
                );
                write(page.as_bytes())
            }
            Sink::Ffmpeg(pipe) => pipe.finish(Ok(())),
        }
    }
}

/// The figure rendered whole at `dpi`, as movie writers render frames
/// (they turn `savefig.bbox = "tight"` off so that every frame has the
/// same size).
fn render(figure: &Figure, dpi: f64) -> Result<RgbaImage, SaveError> {
    let (width, height) = figure.size_inches();
    let options = SaveOptions {
        dpi: Some(dpi),
        bbox_inches: Some(BboxInches::Inches(Bbox::from_bounds(
            0.0, 0.0, width, height,
        ))),
        ..SaveOptions::default()
    };
    Ok(figure.render_raster_with(options)?.to_image())
}

/// The milliseconds between frames an HTML page plays at `fps`:
/// matplotlib's `1000 // fps`, an integer for a whole number of frames
/// per second and a float otherwise, as Python prints them.
fn html_interval(fps: f64) -> String {
    let interval = floor_div(1000.0, fps);
    if fps.fract() == 0.0 {
        format!("{interval}")
    } else {
        pyfmt::repr(interval)
    }
}

#[cfg(test)]
mod tests;
