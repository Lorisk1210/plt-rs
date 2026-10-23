//! Animations of prepared artists (matplotlib's `ArtistAnimation`).

use std::cell::Cell;
use std::rc::Rc;

use super::{AnimationOptions, Frames, FuncAnimation};
use crate::artist::{ArtistHandle, RawHandle};
use crate::figure::Figure;

impl FuncAnimation {
    /// Animates `figure` through prepared artists (matplotlib's
    /// `ArtistAnimation`): every artist of any frame starts hidden, and
    /// frame `i` shows the artists of `frames[i]` after hiding those of the
    /// frame shown before. When blitting, those are the artists drawn.
    ///
    /// ```
    /// use plt::animation::{AnimationOptions, FuncAnimation};
    /// use plt::artist::ArtistHandle;
    /// use plt::figure::Figure;
    /// let fig = Figure::with_size(2.0, 2.0, 50.0);
    /// let ax = fig.subplot();
    /// let frames = (0..3)
    ///     .map(|i| {
    ///         let y = f64::from(i);
    ///         let line = ax.plot(vec![0.0, 1.0], vec![y, y])?;
    ///         Ok(vec![line.raw().clone()])
    ///     })
    ///     .collect::<Result<Vec<_>, plt::axes::PlotError>>()?;
    /// let animation = FuncAnimation::from_artists(&fig, frames, AnimationOptions::default());
    /// assert!(animation.to_gif()?.starts_with(b"GIF89a"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_artists(
        figure: &Figure,
        frames: Vec<Vec<RawHandle>>,
        options: AnimationOptions,
    ) -> Self {
        let frames = Rc::new(frames);
        hide_all(&frames);
        let count = frames.len();
        let shown = Rc::new(Cell::new(None::<usize>));
        let (all, reset) = (Rc::clone(&frames), Rc::clone(&shown));
        // matplotlib's `ArtistAnimation._init_draw` hides every artist.
        let init = move || {
            hide_all(&all);
            reset.set(None);
        };
        let update = move |frame: usize| {
            let previous = shown.get().and_then(|previous| frames.get(previous));
            for artist in previous.into_iter().flatten() {
                artist.set_visible(false);
            }
            let artists = frames.get(frame).cloned().unwrap_or_default();
            for artist in &artists {
                artist.set_visible(true);
            }
            shown.set(Some(frame));
            artists
        };
        Self::with_init(figure, Frames::Count(count), options, init, update)
    }
}

/// Hides every artist of every frame.
fn hide_all(frames: &[Vec<RawHandle>]) {
    for artist in frames.iter().flatten() {
        artist.set_visible(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_frame_shows_its_own_artists() {
        let fig = Figure::with_size(1.0, 1.0, 20.0);
        let ax = fig.subplot();
        let lines: Vec<_> = (0..3)
            .map(|i| {
                let y = f64::from(i);
                ax.plot(vec![0.0, 1.0], vec![y, y]).unwrap()
            })
            .collect();
        let shared = ax.plot(vec![0.0, 1.0], vec![0.5, 0.5]).unwrap();
        let frames = lines
            .iter()
            .map(|line| vec![line.raw().clone(), shared.raw().clone()])
            .collect();
        let animation = FuncAnimation::from_artists(&fig, frames, AnimationOptions::default());
        let visible = || lines.iter().map(ArtistHandle::visible).collect::<Vec<_>>();
        assert_eq!(visible(), [false, false, false], "all hidden at first");
        fig.canvas().draw().unwrap();
        let (timer, _) = fig.timers()[0];
        fig.fire_timer(timer);
        assert_eq!(visible(), [true, false, false], "the first frame");
        fig.fire_timer(timer);
        assert_eq!(visible(), [false, true, false]);
        assert!(
            shared.visible(),
            "an artist in consecutive frames stays shown"
        );
        drop(animation);
    }
}
