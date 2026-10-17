//! Animations (matplotlib's `animation.FuncAnimation`): a function updates
//! the figure for each frame; a figure window plays the frames on a timer,
//! and the animation can be saved as an animated PNG or as a web page with
//! matplotlib's JavaScript player.
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

use std::cell::RefCell;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use plt_codec::png::{AnimatedEncoder, EncodeError, Frame, ImageRef};

use crate::events::TimerKey;
use crate::figure::{Figure, SaveError};

mod html;
mod html_template;

pub use html::{HtmlOptions, PlayMode};

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

/// Timing of an animation (matplotlib's `TimedAnimation` arguments).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationOptions {
    /// Time between frames.
    pub interval: Duration,
    /// Whether to start over after the last frame.
    pub repeat: bool,
    /// Pause before starting over.
    pub repeat_delay: Duration,
}

impl Default for AnimationOptions {
    /// matplotlib's defaults: 200 ms per frame, repeating without pause.
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(200),
            repeat: true,
            repeat_delay: Duration::ZERO,
        }
    }
}

/// Why an animation could not be saved.
#[derive(Debug)]
pub enum AnimationError {
    /// A frame could not be rendered or the file written.
    Save(SaveError),
    /// The frames could not be encoded.
    Encode(EncodeError),
    /// An HTML page was to be written to a file not named `.html` or
    /// `.htm`.
    NotHtml(PathBuf),
}

impl fmt::Display for AnimationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Save(err) => write!(f, "cannot save the animation: {err}"),
            Self::Encode(err) => write!(f, "cannot encode the animation: {err}"),
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
            Self::NotHtml(_) => None,
        }
    }
}

/// The frame function and where playback stands.
struct Player {
    frames: Frames,
    options: AnimationOptions,
    update: Box<dyn FnMut(usize)>,
    next: usize,
    /// When playback resumes after the repeat delay.
    resume_at: Option<Instant>,
}

impl Player {
    /// One timer tick: shows the next frame; returns whether playback
    /// continues.
    fn step(&mut self) -> bool {
        if let Some(resume) = self.resume_at {
            if Instant::now() < resume {
                return true;
            }
            self.resume_at = None;
        }
        if let Frames::Count(count) = self.frames
            && self.next >= count
        {
            if !self.options.repeat || count == 0 {
                return false;
            }
            self.next = 0;
            if !self.options.repeat_delay.is_zero() {
                self.resume_at = Some(Instant::now() + self.options.repeat_delay);
                return true;
            }
        }
        (self.update)(self.next);
        self.next += 1;
        true
    }
}

/// An animation driven by a frame function (matplotlib's
/// `FuncAnimation`). It plays while its figure is shown in a window; it
/// stops when dropped.
pub struct FuncAnimation {
    figure: Figure,
    player: Rc<RefCell<Player>>,
    timer: TimerKey,
}

impl fmt::Debug for FuncAnimation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FuncAnimation")
            .field("timer", &self.timer)
            .finish_non_exhaustive()
    }
}

impl FuncAnimation {
    /// Animates `figure` by calling `update` with each frame number. The
    /// first frame is drawn right away (matplotlib's default `init_func`);
    /// playback then goes through `frames` every `options.interval`.
    pub fn new(
        figure: &Figure,
        frames: Frames,
        options: AnimationOptions,
        update: impl FnMut(usize) + 'static,
    ) -> Self {
        let mut update = update;
        if frames != Frames::Count(0) {
            update(0);
        }
        let player = Rc::new(RefCell::new(Player {
            frames,
            options,
            update: Box::new(update),
            next: 0,
            resume_at: None,
        }));
        let ticking = Rc::downgrade(&player);
        let timer = figure.add_timer(options.interval, move || {
            ticking
                .upgrade()
                .is_some_and(|player| player.borrow_mut().step())
        });
        Self {
            figure: figure.clone(),
            player,
            timer,
        }
    }

    /// Stops playback.
    pub fn stop(&self) {
        self.figure.remove_timer(self.timer);
    }

    /// The number of frames saved: all of them, or matplotlib's default
    /// of 100 for endless animations.
    fn save_count(&self) -> usize {
        match self.player.borrow().frames {
            Frames::Count(count) => count,
            Frames::Endless => DEFAULT_SAVE_COUNT,
        }
    }

    /// Shows every saved frame in turn and hands the figure to `visit`
    /// (matplotlib's `Animation.save` loop).
    fn for_each_frame(
        &self,
        mut visit: impl FnMut(&Figure) -> Result<(), SaveError>,
    ) -> Result<(), AnimationError> {
        for frame in 0..self.save_count() {
            (self.player.borrow_mut().update)(frame);
            visit(&self.figure).map_err(AnimationError::Save)?;
        }
        Ok(())
    }

    /// Renders every saved frame and encodes them as an animated PNG
    /// showing each frame for one interval, looping if the animation
    /// repeats.
    ///
    /// # Errors
    ///
    /// When a frame cannot be rendered or the frames cannot be encoded.
    pub fn to_apng(&self) -> Result<Vec<u8>, AnimationError> {
        let mut images = Vec::with_capacity(self.save_count());
        self.for_each_frame(|figure| {
            images.push(figure.render_raster()?.to_image());
            Ok(())
        })?;
        let millis =
            u16::try_from(self.player.borrow().options.interval.as_millis()).unwrap_or(u16::MAX);
        let frames = images
            .iter()
            .map(|image| {
                let side = |n: usize| u32::try_from(n).map_err(|_| EncodeError::ImageTooLarge);
                let (width, height) = (side(image.width()), side(image.height()));
                Ok(Frame {
                    image: ImageRef::rgba8(width?, height?, image.data())?,
                    delay: (millis, 1000),
                })
            })
            .collect::<Result<Vec<_>, EncodeError>>()
            .map_err(AnimationError::Encode)?;
        let plays = u32::from(!self.player.borrow().options.repeat);
        AnimatedEncoder::new()
            .plays(plays)
            .encode(&frames)
            .map_err(AnimationError::Encode)
    }

    /// Writes [`FuncAnimation::to_apng`] to `path`.
    ///
    /// # Errors
    ///
    /// As [`FuncAnimation::to_apng`], or when the file cannot be written.
    pub fn save_apng(&self, path: impl AsRef<Path>) -> Result<(), AnimationError> {
        let apng = self.to_apng()?;
        std::fs::write(path, apng).map_err(|err| AnimationError::Save(SaveError::Io(err)))
    }
}

impl Drop for FuncAnimation {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn frames_play_in_order_and_repeat() {
        let fig = Figure::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        let options = AnimationOptions {
            repeat: false,
            ..AnimationOptions::default()
        };
        let animation = FuncAnimation::new(&fig, Frames::Count(3), options, move |i| {
            log.borrow_mut().push(i);
        });
        let (timer, _) = fig.timers()[0];
        while fig.fire_timer(timer) {}
        assert_eq!(
            *seen.borrow(),
            [0, 0, 1, 2],
            "the first frame is drawn at once"
        );
        assert_eq!(fig.timers(), []);
        drop(animation);
    }

    #[test]
    fn dropping_stops_playback() {
        let fig = Figure::new();
        let ticks = Rc::new(Cell::new(0));
        let count = Rc::clone(&ticks);
        let animation = FuncAnimation::new(
            &fig,
            Frames::Endless,
            AnimationOptions::default(),
            move |_| {
                count.set(count.get() + 1);
            },
        );
        let (timer, _) = fig.timers()[0];
        assert!(fig.fire_timer(timer));
        drop(animation);
        assert_eq!(fig.timers(), []);
        assert_eq!(ticks.get(), 2);
    }
}
