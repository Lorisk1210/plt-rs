"""
Animations playing in the window: `FuncAnimation` and `ArtistAnimation`,
with and without blitting, stopped by their event source, and closed
while they play. The frames report as they are drawn (with their time,
which the comparison leaves out; `frame_times` gives the intervals).
"""

from . import one_axes, scenario

#: The animations' interval between frames, in milliseconds.
INTERVAL = 50


def _animation_module(plt):
    import importlib
    return importlib.import_module(plt.__name__.rpartition(".")[0]
                                   + ".animation")


def _func(plt, log, *, blit, frames=10, repeat=False, stop_at=None):
    animation = _animation_module(plt)
    fig, ax = one_axes(plt)
    line, = ax.plot([], [])

    def update(frame):
        line.set_data([0, frame], [0, frame])
        log.note("frame", frame=frame)
        if frame == stop_at:
            fig._animation.event_source.stop()
            log.note("stopped")
        return line,

    fig._animation = animation.FuncAnimation(
        fig, update, frames=frames, interval=INTERVAL, blit=blit,
        repeat=repeat)
    return fig


@scenario("func-animation")
def func_animation(plt, log):
    return _func(plt, log, blit=False)


@scenario("func-animation-blit")
def func_animation_blit(plt, log):
    return _func(plt, log, blit=True)


@scenario("animation-stop")
def animation_stop(plt, log):
    return _func(plt, log, blit=True, frames=20, stop_at=4)


def _play(d):
    """Let ten frames and more play, then close the window."""
    d.wait(1.5)
    d.close()


def _timed(records):
    """What is wrong with the timing of an animation's frames: those its
    timer drew (after the first) come every `INTERVAL` milliseconds, give
    or take 40%, none more than 200 ms after the one before."""
    gaps = frame_times(records)[2:]
    if not gaps:
        return ["no frames played"]
    mean = sum(gaps) / len(gaps)
    if not 0.6 * INTERVAL <= 1000 * mean <= 1.4 * INTERVAL or max(gaps) > 0.2:
        return [f"frames {1000 * mean:.0f} ms apart on average, "
                f"{1000 * max(gaps):.0f} ms at most"]
    return []


func_animation.steps(_play)
func_animation_blit.steps(_play)
animation_stop.steps(_play)
func_animation.check(_timed)
func_animation_blit.check(_timed)


def _played(records):
    """An animation's records without its frames and draws, which depend
    on timing: whether it played some frames, and what came after."""
    kept = [r for r in records if r["name"] not in ("frame", "draw_event")]
    frames = sum(r["name"] == "frame" for r in records)
    return [{"name": "played", "frames": frames >= 3}, *kept]


@scenario("animation-close", summarize=_played)
def animation_close(plt, log):
    return _func(plt, log, blit=False, frames=1000, repeat=True)


@animation_close.steps
def _(d):
    # Closed while it plays: the program ends, nothing draws afterwards.
    d.wait(0.33)
    d.close()


def _artists(plt, log, *, blit):
    animation = _animation_module(plt)
    fig, ax = one_axes(plt)
    frames = [[ax.plot([0, i], [0, 10 - i], color="C0")[0]]
              for i in range(8)]
    for i, (artist,) in enumerate(frames):
        artist.set_label(f"frame {i}")

    def shown():
        return [a.get_label() for (a,) in frames if a.get_visible()]
    fig.canvas.mpl_connect("draw_event",
                           lambda event: log.note("drawn", visible=shown()))
    anim = fig._animation = animation.ArtistAnimation(
        fig, frames, interval=INTERVAL, blit=blit, repeat=False)

    def frame():
        """After the animation's own step, which shows the frame: what it
        shows, until the animation has stopped (which stops the timer)."""
        source = anim.event_source
        playing = source is not None and any(
            func == anim._step for func, _, _ in source.callbacks)
        if playing:
            log.note("frame", visible=shown())
        return playing
    anim.event_source.add_callback(frame)
    return fig


@scenario("artist-animation")
def artist_animation(plt, log):
    return _artists(plt, log, blit=False)


@scenario("artist-animation-blit")
def artist_animation_blit(plt, log):
    return _artists(plt, log, blit=True)


artist_animation.steps(_play)
artist_animation_blit.steps(_play)
artist_animation.check(_timed)
artist_animation_blit.check(_timed)


def frame_times(records):
    """The times between the frames of a run's records, in seconds."""
    times = [r["t"] for r in records if r["name"] == "frame"]
    return [later - earlier for earlier, later in zip(times, times[1:])]
