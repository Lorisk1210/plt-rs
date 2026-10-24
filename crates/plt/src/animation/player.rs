//! Playing an animation (matplotlib's `Animation`, `TimedAnimation` and
//! `FuncAnimation` machinery): it starts once its figure is first drawn,
//! a timer then shows the frames, drawn in full or blitted, and a
//! resized figure starts the blitting over.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Instant;

use super::blit::Blitter;
use super::{AnimationOptions, Frames};
use crate::artist::{ArtistHandle, RawHandle};
use crate::events::{ConnectionId, EventKind, TimerKey};
use crate::figure::Figure;

/// A function giving the artists it changed.
pub(super) type FrameFn = Box<dyn FnMut(usize) -> Vec<RawHandle>>;

/// An init function giving the artists it changed.
pub(super) type InitFn = Box<dyn FnMut() -> Vec<RawHandle>>;

/// The figure event connections of an animation.
#[derive(Debug, Default)]
struct Connections {
    /// Starts the animation at the first draw (`_first_draw_id`).
    first_draw: Option<ConnectionId>,
    /// Stops it when the window closes (`_close_id`).
    close: Option<ConnectionId>,
    /// While blitting: the resize callback, or after a resize the draw
    /// callback that resumes playing (`_resize_id`).
    resize: Option<ConnectionId>,
}

/// The frame function and where playback stands.
pub(super) struct Player {
    /// The player itself, for the callbacks and timers it connects.
    this: Weak<RefCell<Player>>,
    pub(super) figure: Figure,
    pub(super) frames: Frames,
    pub(super) options: AnimationOptions,
    init: Option<InitFn>,
    update: FrameFn,
    next: usize,
    /// When playback resumes after the repeat delay.
    resume_at: Option<Instant>,
    /// The timer showing the frames, while playing.
    timer: Option<TimerKey>,
    connections: Connections,
    /// The blitting state, when frames are blitted.
    blitter: Option<Blitter>,
}

impl Player {
    /// A player of `update` on `figure`, connected to its events: it
    /// starts at the figure's first draw (matplotlib's `_start`) and, when
    /// blitting, draws the figure at once (matplotlib's `_setup_blit`).
    pub(super) fn connect(
        figure: &Figure,
        frames: Frames,
        options: AnimationOptions,
        init: Option<InitFn>,
        update: FrameFn,
    ) -> Rc<RefCell<Self>> {
        let player = Rc::new_cyclic(|this| {
            RefCell::new(Self {
                this: this.clone(),
                figure: figure.clone(),
                frames,
                options,
                init,
                update,
                next: 0,
                resume_at: None,
                timer: None,
                connections: Connections::default(),
                blitter: options.blit.then(Blitter::default),
            })
        });
        {
            let mut playing = player.borrow_mut();
            playing.connections.first_draw = Some(playing.on(EventKind::Draw, Self::start));
            playing.connections.close = Some(playing.on(EventKind::Close, Self::stop));
        }
        if options.blit {
            // `_post_draw` with nothing drawn yet draws the figure, which
            // may start the animation: the player must not be borrowed.
            // Problems drawing it come back whenever the figure is drawn.
            let _ = figure.canvas().draw_idle();
            let mut playing = player.borrow_mut();
            playing.init_draw();
            playing.connections.resize = Some(playing.on(EventKind::Resize, Self::on_resize));
        }
        player
    }

    /// Calls `act` on the player with every event of `kind`, unless the
    /// player is busy (the event comes from inside it) or gone.
    fn on(&self, kind: EventKind, act: fn(&mut Self)) -> ConnectionId {
        let this = self.this.clone();
        self.figure.connect(kind, move |_| {
            if let Some(player) = this.upgrade()
                && let Ok(mut player) = player.try_borrow_mut()
            {
                act(&mut player);
            }
        })
    }

    /// The first draw (matplotlib's `_start`): the initial frame, then the
    /// timer; not while the figure is saved.
    fn start(&mut self) {
        if self.figure.state().saving {
            return;
        }
        disconnect(&self.figure, self.connections.first_draw.take());
        self.init_draw();
        self.start_timer();
    }

    /// Starts the timer showing the frames, unless it runs (matplotlib
    /// restarts its one timer).
    fn start_timer(&mut self) {
        if self.timer.is_some() {
            return;
        }
        let this = self.this.clone();
        let timer = self.figure.add_timer(self.options.interval, move || {
            this.upgrade()
                .is_some_and(|player| player.try_borrow_mut().map_or(true, |mut p| p.step()))
        });
        self.timer = Some(timer);
    }

    /// Stops the timer, if running.
    fn stop_timer(&mut self) {
        if let Some(timer) = self.timer.take() {
            self.figure.remove_timer(timer);
        }
    }

    /// Stops playback and drops every connection (matplotlib's `_stop`).
    pub(super) fn stop(&mut self) {
        self.stop_timer();
        let connections = std::mem::take(&mut self.connections);
        for connection in [
            connections.first_draw,
            connections.close,
            connections.resize,
        ] {
            disconnect(&self.figure, connection);
        }
    }

    /// The figure was resized (matplotlib's `_on_resize`): playback pauses,
    /// the saved backgrounds are dropped and the initial frame is set up
    /// again until the figure is drawn at its new size.
    fn on_resize(&mut self) {
        disconnect(&self.figure, self.connections.resize.take());
        self.stop_timer();
        if let Some(blitter) = &mut self.blitter {
            blitter.forget_backgrounds();
        }
        self.init_draw();
        self.connections.resize = Some(self.on(EventKind::Draw, Self::end_redraw));
    }

    /// The figure was drawn after a resize (matplotlib's `_end_redraw`):
    /// playback resumes.
    fn end_redraw(&mut self) {
        self.post_draw(false);
        self.start_timer();
        disconnect(&self.figure, self.connections.resize.take());
        self.connections.resize = Some(self.on(EventKind::Resize, Self::on_resize));
    }

    /// One timer tick (matplotlib's `TimedAnimation._step`): the next
    /// frame, or at the end the initial frame again and a new pass, or the
    /// end of playback; returns whether playback continues.
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
                self.finish();
                return false;
            }
            self.init_draw();
            self.next = 0;
            if !self.options.repeat_delay.is_zero() {
                self.resume_at = Some(Instant::now() + self.options.repeat_delay);
                return true;
            }
        }
        self.draw_next_frame(self.next);
        self.next += 1;
        true
    }

    /// The end of a pass without repetition (matplotlib's `pause` and
    /// what follows it): the blitted artists become ordinary ones again.
    fn finish(&mut self) {
        // The ending tick removes the timer itself.
        self.timer = None;
        if let Some(blitter) = &self.blitter {
            for artist in blitter.drawn.iter().filter(|a| a.is_attached()) {
                artist.set_animated(false);
            }
        }
        disconnect(&self.figure, self.connections.resize.take());
        disconnect(&self.figure, self.connections.close.take());
    }

    /// Sets up the initial frame (matplotlib's `_init_draw`): what the
    /// init function changes, or the first frame.
    pub(super) fn init_draw(&mut self) {
        match &mut self.init {
            Some(init) => {
                let artists = init();
                if let Some(blitter) = &mut self.blitter {
                    animate(&artists);
                    blitter.drawn = artists;
                }
            }
            None if self.frames != Frames::Count(0) => self.draw_frame(0),
            None => {}
        }
    }

    /// Calls the frame function (matplotlib's `_draw_frame`); when
    /// blitting, the artists it changed become animated, in z-order.
    pub(super) fn draw_frame(&mut self, frame: usize) {
        let mut artists = (self.update)(frame);
        if let Some(blitter) = &mut self.blitter {
            artists.retain(RawHandle::is_attached);
            artists.sort_by(|a, b| a.zorder().total_cmp(&b.zorder()));
            animate(&artists);
            blitter.drawn = artists;
        }
    }

    /// Shows the next frame (matplotlib's `_draw_next_frame`): the last
    /// frame's artists erased when blitting, the frame function called,
    /// then the frame drawn.
    fn draw_next_frame(&mut self, frame: usize) {
        if let Some(blitter) = &mut self.blitter {
            blitter.clear(&self.figure);
        }
        self.draw_frame(frame);
        self.post_draw(true);
    }

    /// Draws what the frame changed (matplotlib's `_post_draw`): blitted
    /// when `blit` and the frame gave artists, else as a full draw.
    fn post_draw(&mut self, blit: bool) {
        let figure = &self.figure;
        match &mut self.blitter {
            Some(blitter) if blit && !blitter.drawn.is_empty() => {
                figure.canvas().report(blitter.draw(figure));
            }
            // Problems of a full draw come back whenever the figure is
            // drawn.
            _ => {
                let _ = figure.canvas().draw_idle();
            }
        }
    }
}

/// Drops a connection to the events of `figure`, if made.
fn disconnect(figure: &Figure, connection: Option<ConnectionId>) {
    if let Some(id) = connection {
        figure.disconnect(id);
    }
}

/// Makes `artists` animated, as blitting draws them.
fn animate(artists: &[RawHandle]) {
    for artist in artists.iter().filter(|a| a.is_attached()) {
        artist.set_animated(true);
    }
}
