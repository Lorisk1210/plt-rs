"""
A program for measuring what an event loop costs and how promptly it
answers, under either library (``loop_probe.py LIBRARY BACKEND ACTION``,
as `idling` runs it): it shows a figure, prints what it sees as JSON
lines on stdout, with times of ``time.monotonic()`` (the clock every
process shares), and blocks in the loop as *ACTION* says:

* ``show``, ``pause``, ``start_event_loop``, ``waitforbuttonpress``,
  ``ginput``: blocks in that call, after ``{"name": "drawn"}`` (the
  window's first draw is done) until a signal interrupts it, then prints
  ``{"name": "interrupted"}`` with the time `KeyboardInterrupt` reached
  it;
* ``callback``: the same in `plt.show`, where a timer's callback works
  for a second (``{"name": "working"}`` when it starts), so that the
  signal comes during it;
* ``worker``: counts what a worker thread gets done in a second while
  the window idles in the loop, then while the main thread sleeps
  (``{"name": "worker", "looping": N, "sleeping": M}``);
* ``queued``: a worker thread asks for an idle draw a moment after the
  first draw, then closes the window (``{"name": "asked"}``,
  ``{"name": "redrawn"}``, ``{"name": "closed"}``, which ends `plt.show`);
* ``animation``: a `FuncAnimation` every `INTERVAL` ms, each frame
  printed with how long the main thread waited for a CPU so far, until
  interrupted.
"""

import importlib
import json
import signal
import sys
import threading
import time

#: The animation's interval between frames, in milliseconds.
INTERVAL = 50
#: How long the blocking calls block at most, in seconds: longer than
#: any measurement.
LONG = 60


def say(name, **fields):
    """Print record *name* with its time."""
    print(json.dumps({"name": name, "t": time.monotonic(), **fields}),
          flush=True)


def cpu_wait():
    """How long this thread has waited for a CPU so far, in seconds."""
    with open("/proc/thread-self/schedstat", encoding="ascii") as file:
        return int(file.read().split()[1]) / 1e9


def count_for(seconds):
    """How many rounds of a pure Python loop run in *seconds*."""
    end, rounds = time.monotonic() + seconds, 0
    while time.monotonic() < end:
        rounds += 1
    return rounds


def blocking_calls(plt, fig):
    """The blocking calls by action name."""
    return {
        "show": plt.show,
        "pause": lambda: plt.pause(LONG),
        "start_event_loop": lambda: fig.canvas.start_event_loop(LONG),
        "waitforbuttonpress": lambda: plt.waitforbuttonpress(LONG),
        "ginput": lambda: plt.ginput(1, timeout=LONG),
        "callback": plt.show,
    }


def main(library, backend, action):
    # A shell starting the tests in the background (`cmd &` without job
    # control) has them ignore SIGINT, which Python then leaves ignored.
    signal.signal(signal.SIGINT, signal.default_int_handler)
    mpl = importlib.import_module(library)
    mpl.use(backend)
    plt = importlib.import_module(f"{library}.pyplot")
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.plot([0, 1], [0, 1])
    drawn = []

    def first_draw(_event):
        if not drawn:
            drawn.append(True)
            say("drawn")
    fig.canvas.mpl_connect("draw_event", first_draw)
    if action == "worker":
        worker(plt)
    elif action == "queued":
        queued(plt, fig)
    elif action == "animation":
        animation(plt, fig, ax)
    else:
        if action == "callback":
            fig._probe_timer = callback_timer(fig)
        call = blocking_calls(plt, fig)[action]
        if action not in ("show", "callback"):
            # The calls that do not show the window themselves.
            plt.show(block=False)
        interruptible(call)


def interruptible(call):
    """Run *call* until a signal interrupts it."""
    try:
        call()
    except KeyboardInterrupt:
        say("interrupted")


def callback_timer(fig):
    """A timer whose callback works a second, a second after it starts."""
    timer = fig.canvas.new_timer(interval=1000)
    timer.single_shot = True

    def work():
        say("working")
        count_for(1.0)
    timer.add_callback(work)
    timer.start()
    return timer


def worker(plt):
    """What a worker gets done while the window idles, then while the
    main thread sleeps."""
    counts = {}

    def count():
        counts["looping"] = count_for(1.0)
    plt.show(block=False)
    plt.pause(0.5)
    thread = threading.Thread(target=count)
    thread.start()
    plt.pause(1.5)
    thread.join()
    thread = threading.Thread(target=lambda: counts.update(
        sleeping=count_for(1.0)))
    thread.start()
    time.sleep(1.5)
    thread.join()
    say("worker", **counts)


def queued(plt, fig):
    """An idle draw and a close asked for on a worker thread."""
    redrawn_wanted, redrawn = threading.Event(), threading.Event()

    def on_draw(_event):
        if redrawn_wanted.is_set() and not redrawn.is_set():
            redrawn.set()
            say("redrawn")
    fig.canvas.mpl_connect("draw_event", on_draw)

    def ask():
        time.sleep(1.0)
        redrawn_wanted.set()
        say("asked")
        fig.canvas.draw_idle()
        redrawn.wait(LONG)
        time.sleep(0.2)
        say("closing")
        plt.close(fig)
    threading.Thread(target=ask, daemon=True).start()
    plt.show()
    say("closed")


def animation(plt, fig, ax):
    """Frames every `INTERVAL` ms until interrupted."""
    animation_module = importlib.import_module(
        plt.__name__.rpartition(".")[0] + ".animation")
    line, = ax.plot([], [])

    def update(frame):
        line.set_data([0, frame % 10], [0, 1])
        say("frame", waited=cpu_wait())
        return line,
    fig._probe_animation = animation_module.FuncAnimation(
        fig, update, interval=INTERVAL, cache_frame_data=False)
    interruptible(plt.show)


if __name__ == "__main__":
    main(*sys.argv[1:])
