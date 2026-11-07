"""
Plotting calls that fail leave the Axes as they found it.

matplotlib's plotting methods make their artists, the properties given
and all, before they add any to the Axes: a call that fails (an invalid
property, say) adds nothing. pltrs (Rust) makes the artists in the Axes,
and the methods then set the properties of the program's on them, so a
method `leaves_nothing_on_failure` takes back what the call added: its
artists, its containers, and the limits, autoscaling and aspect it
changed.
"""

import functools

from .. import _handles
from .._native import axes as _axes

#: The methods of `~.axes.Axes` whose matplotlib versions make all their
#: artists before adding any. Not among them: ``hist`` and ``vlines``,
#: which set the properties on the artists they added, nor ``eventplot``,
#: ``acorr`` and ``xcorr``, which add some artists before making others.
METHODS = (
    "annotate", "axhline", "axhspan", "axline", "axvline", "axvspan", "bar",
    "bar_label", "barbs", "barh", "broken_barh", "contour", "contourf",
    "ecdf", "fill", "fill_between", "fill_betweenx", "grouped_bar",
    "hexbin", "hist2d", "imshow", "loglog", "matshow", "pcolor",
    "pcolorfast", "pcolormesh", "plot", "quiver", "quiverkey", "scatter",
    "semilogx", "semilogy", "spy", "stackplot", "stairs", "step", "table",
    "text", "tricontour", "tricontourf", "tripcolor", "triplot",
    "angle_spectrum", "cohere", "csd", "magnitude_spectrum", "phase_spectrum",
    "psd", "specgram",
)


def leaves_nothing_on_failure(method):
    """*method*, a plotting method of `~.axes.Axes`, taking back what a
    call that fails added to the Axes before it raises."""
    @functools.wraps(method)
    def plotting(self, *args, **kwargs):
        mark = _axes.call_mark(self._native)
        containers = len(self.containers)
        try:
            return method(self, *args, **kwargs)
        except Exception:
            del self.containers[containers:]
            for native in _axes.undo_since(self._native, mark):
                _handles.wrap_any(native, self._figure).remove()
            raise
    return plotting


def guard(cls):
    """Make the `METHODS` of *cls* leave nothing when they fail."""
    for name in METHODS:
        setattr(cls, name, leaves_nothing_on_failure(getattr(cls, name)))
