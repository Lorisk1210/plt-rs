//! Animations of prepared artists (matplotlib's `ArtistAnimation`).

use super::{AnimationOptions, Frames, FuncAnimation};
use crate::artist::{ArtistHandle, RawHandle};
use crate::figure::Figure;

impl FuncAnimation {
    /// Animates `figure` through prepared artists (matplotlib's
    /// `ArtistAnimation`): every artist of any frame starts hidden, and
    /// frame `i` shows the artists of `frames[i]` after hiding those of the
    /// frame shown before.
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
        for artist in frames.iter().flatten() {
            artist.set_visible(false);
        }
        let count = frames.len();
        let mut shown: Option<usize> = None;
        Self::new(figure, Frames::Count(count), options, move |frame| {
            for artist in shown
                .and_then(|previous| frames.get(previous))
                .into_iter()
                .flatten()
            {
                artist.set_visible(false);
            }
            for artist in frames.get(frame).into_iter().flatten() {
                artist.set_visible(true);
            }
            shown = Some(frame);
        })
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
        assert_eq!(visible(), [true, false, false], "the first frame is drawn");
        let (timer, _) = fig.timers()[0];
        fig.fire_timer(timer);
        fig.fire_timer(timer);
        assert_eq!(visible(), [false, true, false]);
        assert!(
            shared.visible(),
            "an artist in consecutive frames stays shown"
        );
        drop(animation);
    }
}
