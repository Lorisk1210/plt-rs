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

from ._program_classes import takes_from_program


def draw(figure, native, renderer):
    """Draw *figure* as matplotlib's canvases do: through its class'
    ``draw`` when the program defines one, called with the renderer
    *renderer()* gives, whose call of the base ``Figure.draw`` runs
    *native* (pltrs (Rust)'s draw of the figure); else *native* alone.
    *native* runs once: also when the program's ``draw`` never reaches the
    base one (matplotlib then draws nothing, which pltrs (Rust) has no way
    to draw)."""
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
