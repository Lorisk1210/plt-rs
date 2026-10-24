//! Animations (matplotlib's `animation.FuncAnimation` and
//! `ArtistAnimation`): a function updates the figure for each frame, or
//! each frame shows prepared artists. A figure window plays the frames on
//! a timer once it has drawn the figure, either redrawing the figure for
//! each frame or blitting: drawing only the artists the frame changed
//! over the saved background of their axes. The animation can be saved
//! as an animated PNG or GIF, as a web page with matplotlib's JavaScript
//! player, or as a video through ffmpeg.
//!
//! ```
//! use plt::animation::{AnimationOptions, Frames, FuncAnimation};
//! use plt::figure::Figure;
//! let fig = Figure::with_size(2.0, 2.0, 50.0);
//! let ax = fig.subplot();
//! let line = ax.plot(vec![0.0, 1.0], vec![0.0, 0.0])?;
//! let animation = FuncAnimation::new(&fig, Frames::Count(5), AnimationOptions::default(), move |i| {
//!     let y = i as f64 / 4.0;
//!     line.set_data(&[0.0, 1.0], &[0.0, y]);
//! });
//! let apng = animation.to_apng()?;
//! assert!(apng.windows(4).any(|chunk| chunk == b"acTL"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! With [`AnimationOptions::blit`], the frame function returns the
//! artists it changed. They should be animated from the start, so that
//! the background saved when the figure is first drawn leaves them out:
//!
//! ```
//! use plt::animation::{AnimationOptions, Frames, FuncAnimation};
//! use plt::artist::ArtistHandle;
//! use plt::figure::Figure;
//! let fig = Figure::with_size(2.0, 2.0, 50.0);
//! let ax = fig.subplot();
//! let line = ax.plot(vec![0.0, 1.0], vec![0.0, 0.0])?;
//! line.set_animated(true);
//! let options = AnimationOptions { blit: true, ..AnimationOptions::default() };
//! let animation = FuncAnimation::new(&fig, Frames::Count(5), options, move |i| {
//!     line.set_data(&[0.0, 1.0], &[0.0, i as f64 / 4.0]);
//!     vec![line.raw().clone()]
//! });
//! // The figure was drawn, and the animation started playing.
//! let (timer, _) = fig.timers()[0];
//! fig.fire_timer(timer);
//! assert!(!fig.is_stale(), "the frame was blitted, not redrawn");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::cell::RefCell;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use plt_codec::gif::GifError;
use plt_codec::png::EncodeError;
use plt_codec::webp;
use plt_core::Bbox;
use plt_render::RgbaImage;

use crate::artist::RawHandle;
use crate::figure::{BboxInches, Figure, SaveError, SaveOptions};
use player::Player;

mod artists;
mod blit;
mod html;
mod html_template;
mod movie;
mod player;
mod save;
#[cfg(test)]
mod tests;

pub use html::{HtmlOptions, PlayMode};
pub use movie::{MovieError, MovieOptions};

/// Frames saved from an animation without a known length (matplotlib's
/// default `save_count`).
const DEFAULT_SAVE_COUNT: usize = 100;

/// Which frames an animation goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Frames {
    /// Frames `0..n`.
    Count(usize),
    /// Frames `0, 1, 2, ...` without end.
    Endless,
}

/// Timing and drawing of an animation (matplotlib's `TimedAnimation`
/// arguments).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationOptions {
    /// Time between frames.
    pub interval: Duration,
    /// Whether to start over after the last frame.
    pub repeat: bool,
    /// Pause before starting over.
    pub repeat_delay: Duration,
    /// Whether frames are blitted (matplotlib's `blit`): the artists the
    /// frame function returns become animated and only they are drawn,
    /// over the background of their axes saved once, and only those axes
    /// are shown again. Frames giving no artists are drawn in full.
    pub blit: bool,
}

impl Default for AnimationOptions {
    /// matplotlib's defaults: 200 ms per frame, repeating without pause,
    /// without blitting.
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(200),
            repeat: true,
            repeat_delay: Duration::ZERO,
            blit: false,
        }
    }
}

/// What a frame or init function returns: nothing, or the artists it
/// changed, which blitting draws (matplotlib requires them with
/// `blit=True`).
pub trait ChangedArtists {
    /// The artists changed.
    fn into_artists(self) -> Vec<RawHandle>;
}

impl ChangedArtists for () {
    fn into_artists(self) -> Vec<RawHandle> {
        Vec::new()
    }
}

impl ChangedArtists for Vec<RawHandle> {
    fn into_artists(self) -> Vec<RawHandle> {
        self
    }
}

/// Why an animation could not be saved.
#[derive(Debug)]
pub enum AnimationError {
    /// A frame could not be rendered or the file written.
    Save(SaveError),
    /// The frames could not be encoded as an animated PNG.
    Encode(EncodeError),
    /// The frames could not be encoded as a GIF.
    Gif(GifError),
    /// The frames could not be encoded as a WebP.
    Webp(webp::EncodeError),
    /// The file name's extension names no format animations are saved in.
    UnsupportedFormat(String),
    /// An HTML page was to be written to a file not named `.html` or
    /// `.htm`.
    NotHtml(PathBuf),
    /// ffmpeg could not make the video.
    Movie(MovieError),
}

impl fmt::Display for AnimationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Save(err) => write!(f, "cannot save the animation: {err}"),
            Self::Encode(err) => write!(f, "cannot encode the animation: {err}"),
            Self::Gif(err) => write!(f, "cannot encode the animation as a GIF: {err}"),
            Self::Webp(err) => write!(f, "cannot encode the animation as a WebP: {err}"),
            Self::UnsupportedFormat(extension) => write!(
                f,
                "cannot save animations as {extension:?} (supported: apng, gif, htm, html, png, webp)"
            ),
            Self::Movie(err) => err.fmt(f),
            Self::NotHtml(path) => write!(
                f,
                "cannot write the player to {}: the file name must end in .html or .htm",
                path.display()
            ),
        }
    }
}

impl std::error::Error for AnimationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Save(err) => Some(err),
            Self::Encode(err) => Some(err),
            Self::Gif(err) => Some(err),
            Self::Webp(err) => Some(err),
            Self::Movie(err) => Some(err),
            Self::UnsupportedFormat(_) | Self::NotHtml(_) => None,
        }
    }
}

/// An animation driven by a frame function (matplotlib's
/// `FuncAnimation`). It starts playing once its figure is drawn, as when
/// a window shows it: the initial frame is set up, then a timer goes
/// through `frames` every interval. It stops when dropped.
pub struct FuncAnimation {
    figure: Figure,
    player: Rc<RefCell<Player>>,
}

impl fmt::Debug for FuncAnimation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FuncAnimation")
            .field("figure", &self.figure)
            .finish_non_exhaustive()
    }
}

impl FuncAnimation {
    /// Animates `figure` by calling `update` with each frame number; the
    /// first frame is the initial one (matplotlib's default `init_func`).
    /// With [`AnimationOptions::blit`], `update` returns the artists it
    /// changed, and the figure is drawn at once to start playing.
    pub fn new<R: ChangedArtists>(
        figure: &Figure,
        frames: Frames,
        options: AnimationOptions,
        update: impl FnMut(usize) -> R + 'static,
    ) -> Self {
        Self::build(figure, frames, options, None, update)
    }

    /// Animates `figure` by calling `update` with each frame number, after
    /// `init` sets up the initial frame (matplotlib's `init_func`); when
    /// blitting, both return the artists they changed.
    pub fn with_init<I: ChangedArtists, R: ChangedArtists>(
        figure: &Figure,
        frames: Frames,
        options: AnimationOptions,
        mut init: impl FnMut() -> I + 'static,
        update: impl FnMut(usize) -> R + 'static,
    ) -> Self {
        let init: player::InitFn = Box::new(move || init().into_artists());
        Self::build(figure, frames, options, Some(init), update)
    }

    fn build<R: ChangedArtists>(
        figure: &Figure,
        frames: Frames,
        options: AnimationOptions,
        init: Option<player::InitFn>,
        mut update: impl FnMut(usize) -> R + 'static,
    ) -> Self {
        let update: player::FrameFn = Box::new(move |frame| update(frame).into_artists());
        Self {
            figure: figure.clone(),
            player: Player::connect(figure, frames, options, init, update),
        }
    }

    /// Stops playback.
    pub fn stop(&self) {
        if let Ok(mut player) = self.player.try_borrow_mut() {
            player.stop();
        }
    }

    /// The number of frames saved: all of them, or matplotlib's default
    /// of 100 for endless animations.
    fn save_count(&self) -> usize {
        match self.player.borrow().frames {
            Frames::Count(count) => count,
            Frames::Endless => DEFAULT_SAVE_COUNT,
        }
    }

    /// Sets up the initial frame, then shows every saved frame in turn and
    /// hands the figure to `visit` (matplotlib's `Animation.save` loop,
    /// which draws every frame in full; the figure's own canvas is left
    /// as it is).
    fn for_each_frame(
        &self,
        mut visit: impl FnMut(&Figure) -> Result<(), SaveError>,
    ) -> Result<(), AnimationError> {
        self.player.borrow_mut().init_draw();
        for frame in 0..self.save_count() {
            self.player.borrow_mut().draw_frame(frame);
            visit(&self.figure).map_err(AnimationError::Save)?;
        }
        Ok(())
    }
}

/// One frame of `figure`, rendered as `savefig` would but always of the
/// whole figure: matplotlib's movie writers turn `savefig.bbox = "tight"`
/// off so that every frame has the same size.
fn render_frame(figure: &Figure) -> Result<RgbaImage, SaveError> {
    let (width, height) = figure.size_inches();
    let options = SaveOptions {
        bbox_inches: Some(BboxInches::Inches(Bbox::from_bounds(
            0.0, 0.0, width, height,
        ))),
        ..SaveOptions::default()
    };
    Ok(figure.render_raster_with(options)?.to_image())
}

/// `duration` in milliseconds, exact for whole milliseconds.
fn interval_millis(duration: Duration) -> f64 {
    let whole_seconds = Duration::from_secs(duration.as_secs()).as_secs_f64();
    whole_seconds * 1000.0 + f64::from(duration.subsec_nanos()) / 1e6
}

impl Drop for FuncAnimation {
    fn drop(&mut self) {
        self.stop();
    }
}
