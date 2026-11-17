"""
Figures whose class draws them its own way.

matplotlib's canvases draw a figure by calling ``figure.draw(renderer)``,
and its prints too, with the renderer of the format; a `.Figure` subclass
of the program's overriding ``draw`` (ultraplot snaps its Axes to the
pixel grid of the renderer there, and shares tick labels) runs its code
around ``super().draw(renderer)``, which draws the figure. pltrs (Rust)
draws figures itself: for such a figure, a draw or a print calls the
class' ``draw`` with the renderer matplotlib's would hand it, and the base
``Figure.draw`` it reaches runs pltrs (Rust)'s draw there (`draw`, `take`).
"""

import contextlib
import threading

from ._program_classes import takes_from_program


class DryRunDrawn(Exception):
    """A print method of the program's, run as matplotlib's
    ``print_figure`` runs it before a print cut to its tight box or laid
    out by an engine (``_get_renderer``), reached the figure's draw: its
    argument is the renderer the draw was given."""


class _DryRuns(threading.local):
    """Whether this thread runs a print method dry (`dry_run`)."""

    running = False


_dry_runs = _DryRuns()


@contextlib.contextmanager
def dry_run():
    """Run a print method of the program's dry: the first draw it reaches
    raises `DryRunDrawn` with its renderer, as matplotlib's
    ``_get_renderer`` stops its print at ``figure.draw``."""
    _dry_runs.running = True
    try:
        yield
    finally:
        _dry_runs.running = False


def stop_dry_run(renderer):
    """Stop the print method running dry, if one is, at the draw it
    reached with *renderer* (`dry_run`)."""
    if _dry_runs.running:
        _dry_runs.running = False
        raise DryRunDrawn(renderer)


def draw(figure, native, renderer):
    """Draw *figure* as matplotlib's canvases do: through its class'
    ``draw`` when the program defines one, called with the renderer
    *renderer()* gives, whose call of the base ``Figure.draw`` runs
    *native* (pltrs (Rust)'s draw of the figure); else *native* alone.
    *native* runs once: also when the program's ``draw`` never reaches the
    base one (matplotlib then draws nothing, which pltrs (Rust) has no way
    to draw). A print method run dry stops there (`dry_run`)."""
    if _dry_runs.running:
        stop_dry_run(renderer())
    if not takes_from_program(type(figure), "draw"):
        native()
        return
    ran = []

    def run():
        ran.append(True)
        native()

    pending = figure.__dict__.setdefault("_native_draws", [])
    pending.append(run)
    try:
        figure.draw(renderer())
    finally:
        if run in pending:
            pending.remove(run)
    if not ran:
        native()


def take(figure):
    """The native draw a canvas' draw of *figure* waits for (see `draw`),
    which the base ``Figure.draw`` runs; None outside such a draw, or once
    it ran (a second call draws the figure as an artist)."""
    pending = figure.__dict__.get("_native_draws")
    return pending.pop() if pending else None
