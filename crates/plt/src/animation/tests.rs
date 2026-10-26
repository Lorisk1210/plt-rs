use std::cell::Cell;

use super::*;
use crate::artist::ArtistHandle;
use crate::events::{Event, EventKind};
use crate::lines::Line2D;

/// A log of the frames a frame function was called with.
fn frame_log() -> (Rc<RefCell<Vec<usize>>>, impl FnMut(usize) + 'static) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&seen);
    (seen, move |i| log.borrow_mut().push(i))
}

#[test]
fn playing_starts_at_the_first_draw() {
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let (seen, update) = frame_log();
    let options = AnimationOptions {
        repeat: false,
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(&fig, Frames::Count(3), options, update);
    assert_eq!(fig.timers(), [], "waits for the figure to be drawn");
    assert!(seen.borrow().is_empty());
    fig.canvas().draw().unwrap();
    let (timer, _) = fig.timers()[0];
    while fig.fire_timer(timer) {}
    assert_eq!(
        *seen.borrow(),
        [0, 0, 1, 2],
        "the first frame is the initial one"
    );
    assert_eq!(fig.timers(), []);
    fig.canvas().draw().unwrap();
    assert_eq!(seen.borrow().len(), 4, "started once");
    drop(animation);
}

#[test]
fn saving_neither_starts_playing_nor_needs_it() {
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let (seen, update) = frame_log();
    let animation = FuncAnimation::new(&fig, Frames::Count(2), AnimationOptions::default(), update);
    animation.to_gif().unwrap();
    assert_eq!(fig.timers(), [], "not started by the frames' draws");
    assert_eq!(*seen.borrow(), [0, 0, 1], "the initial frame, then each");
}

#[test]
fn repeating_starts_over_from_the_initial_frame() {
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let inits = Rc::new(Cell::new(0));
    let counted = Rc::clone(&inits);
    let (seen, update) = frame_log();
    let animation = FuncAnimation::with_init(
        &fig,
        Frames::Count(2),
        AnimationOptions::default(),
        move || counted.set(counted.get() + 1),
        update,
    );
    fig.canvas().draw().unwrap();
    assert_eq!(inits.get(), 1);
    let (timer, _) = fig.timers()[0];
    for _ in 0..3 {
        assert!(fig.fire_timer(timer));
    }
    assert_eq!(*seen.borrow(), [0, 1, 0], "frame 0 again after the init");
    assert_eq!(inits.get(), 2);
    drop(animation);
}

#[test]
fn dropping_stops_playback() {
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let ticks = Rc::new(Cell::new(0));
    let count = Rc::clone(&ticks);
    let animation = FuncAnimation::new(
        &fig,
        Frames::Endless,
        AnimationOptions::default(),
        move |_| count.set(count.get() + 1),
    );
    fig.canvas().draw().unwrap();
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer));
    drop(animation);
    assert_eq!(fig.timers(), []);
    assert_eq!(ticks.get(), 2);
    fig.process_event(&Event::Close);
}

/// A figure with a thick line moved by a blitted animation of `frames`
/// frames, and the line.
fn blitted(frames: usize, repeat: bool) -> (Figure, Line2D, FuncAnimation) {
    let fig = Figure::with_size(1.0, 1.0, 50.0);
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 1.0))
        .expect("finite limits");
    let props = crate::lines::LineProps::default().linewidth(10.0);
    let line = ax.plot_with(vec![0.0, 1.0], vec![0.1, 0.1], props).unwrap();
    // Animated from the start, the line stays out of the first draw, whose
    // pixels become the background.
    line.set_animated(true);
    let moved = line.clone();
    let options = AnimationOptions {
        blit: true,
        repeat,
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(&fig, Frames::Count(frames), options, move |i| {
        let y = 0.1 + 0.2 * i as f64;
        moved.set_data([0.0, 1.0], [y, y]);
        moved.clone()
    });
    (fig, line, animation)
}

/// Whether `artist` is animated.
fn animated(artist: &impl ArtistHandle) -> bool {
    artist.raw().with_base(|base| base.animated)
}

/// Whether the canvas shows the line at pixel row `row`.
fn line_at(fig: &Figure, row: usize) -> bool {
    let image = fig.canvas().buffer_rgba().unwrap();
    image
        .pixel(25, row)
        .is_some_and(|p| p != [255, 255, 255, 255])
}

#[test]
fn blitted_frames_draw_only_the_changed_artists() {
    let (fig, line, animation) = blitted(3, false);
    assert!(animated(&line), "the frame function's artists are animated");
    let (timer, _) = fig.timers()[0];
    let draws = Rc::new(Cell::new(0));
    let counted = Rc::clone(&draws);
    fig.connect(EventKind::Draw, move |_| counted.set(counted.get() + 1));
    assert!(fig.fire_timer(timer));
    assert!(!fig.is_stale());
    assert_eq!(draws.get(), 0, "no full draw");
    // y = 0.1 is pixel row 45 of 50.
    assert!(line_at(&fig, 45), "frame 0");
    assert!(fig.fire_timer(timer));
    assert!(!line_at(&fig, 45), "the background was put back");
    assert!(line_at(&fig, 35), "frame 1 at y = 0.3");
    assert!(fig.fire_timer(timer));
    assert!(!fig.fire_timer(timer), "the end");
    assert!(!animated(&line), "an ordinary artist again");
    drop(animation);
}

#[test]
fn a_resize_pauses_until_the_figure_is_drawn() {
    let (fig, _line, animation) = blitted(10, true);
    let (timer, _) = fig.timers()[0];
    fig.fire_timer(timer);
    fig.set_size_inches(1.0, 2.0).unwrap();
    fig.process_event(&Event::Resize {
        width: 50.0,
        height: 100.0,
    });
    assert_eq!(fig.timers(), [], "paused");
    fig.canvas().draw().unwrap();
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer));
    // The background was saved again at the new size: the frame shows.
    assert_eq!(fig.canvas().buffer_rgba().unwrap().height(), 100);
    drop(animation);
}

#[test]
fn a_window_starting_a_resized_animation_runs_one_timer() {
    let fig = Figure::with_size(1.0, 1.0, 50.0);
    let line = fig.subplot().plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
    // A window shows the figure: it draws the figure when it refreshes.
    let shown = crate::figure::Presenting::new(&fig);
    let options = AnimationOptions {
        blit: true,
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(&fig, Frames::Endless, options, move |_| line.clone());
    assert_eq!(fig.timers(), [], "not drawn yet");
    // The window opens: a resize, then a draw, which both start playing.
    fig.process_event(&Event::Resize {
        width: 50.0,
        height: 50.0,
    });
    fig.canvas().draw().unwrap();
    assert_eq!(fig.timers().len(), 1);
    drop((animation, shown));
}

#[test]
fn a_full_draw_makes_the_background_be_saved_again() {
    let (fig, line, animation) = blitted(10, true);
    let ax = fig.axes()[0].clone();
    let (timer, _) = fig.timers()[0];
    fig.fire_timer(timer);
    ax.set_facecolor("k");
    fig.canvas().draw().unwrap();
    fig.fire_timer(timer);
    fig.fire_timer(timer);
    let image = fig.canvas().buffer_rgba().unwrap();
    assert_eq!(
        image.pixel(25, 5),
        Some([0, 0, 0, 255]),
        "the new background"
    );
    assert!(animated(&line));
    drop(animation);
}

#[test]
fn paused_playback_resumes_where_it_stopped() {
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let (seen, update) = frame_log();
    let animation = FuncAnimation::new(&fig, Frames::Endless, AnimationOptions::default(), update);
    fig.canvas().draw().unwrap();
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer));
    animation.pause();
    animation.pause();
    assert_eq!(fig.timers(), []);
    animation.resume();
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer));
    animation.resume();
    assert_eq!(fig.timers().len(), 1, "resuming twice runs one timer");
    assert_eq!(*seen.borrow(), [0, 0, 1], "frame 1 follows frame 0");
}

#[test]
fn pausing_blitted_frames_makes_their_artists_ordinary() {
    let (fig, line, animation) = blitted(10, true);
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer));
    animation.pause();
    assert!(!animated(&line), "drawn with the figure while paused");
    fig.canvas().draw().unwrap();
    assert!(line_at(&fig, 45), "a full draw shows frame 0");
    animation.resume();
    assert!(animated(&line));
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer));
    assert!(line_at(&fig, 35), "frame 1 at y = 0.3");
}

#[test]
fn a_pause_holds_through_the_first_draw_and_a_resize() {
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let (seen, update) = frame_log();
    let animation = FuncAnimation::new(&fig, Frames::Count(4), AnimationOptions::default(), update);
    animation.pause();
    fig.canvas().draw().unwrap();
    assert_eq!(fig.timers(), [], "held at the initial frame");
    assert_eq!(*seen.borrow(), [0]);
    animation.resume();
    assert_eq!(fig.timers().len(), 1);

    let (fig, _line, blitting) = blitted(10, true);
    blitting.pause();
    fig.process_event(&Event::Resize {
        width: 50.0,
        height: 50.0,
    });
    fig.canvas().draw().unwrap();
    assert_eq!(fig.timers(), [], "a redraw after a resize keeps it paused");
    blitting.resume();
    assert_eq!(fig.timers().len(), 1);
}

#[test]
fn nothing_resumes_once_playback_is_over() {
    let (fig, line, animation) = blitted(1, false);
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer));
    animation.pause();
    assert!(!fig.fire_timer(timer), "the timer is gone");
    animation.resume();
    let (timer, _) = fig.timers()[0];
    assert!(!fig.fire_timer(timer), "the end");
    animation.pause();
    animation.resume();
    assert_eq!(fig.timers(), []);
    assert!(!animated(&line));

    let (fig, _line, stopped) = blitted(10, true);
    stopped.stop();
    stopped.resume();
    assert_eq!(fig.timers(), []);
}
