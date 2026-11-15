"""
The text metrics cache of each renderer: the ``lru_cache`` matplotlib's
``text._get_text_metrics_function`` makes for every renderer, kept by
pltrs (Rust) (``pltrs::text::MetricsCache``).

matplotlib measures every line of text it lays out through the cache of
the renderer it lays out with. pltrs (Rust) measures text itself, so the
cache of a renderer is installed natively while pltrs lays out or draws a
figure with that renderer (`measuring_with`): the lines it measures
meanwhile are found in or added to the cache, with no call into Python per
line. Measuring through the cache from Python (``Text._get_layout``'s
``_get_text_metrics_with_cache``) calls the renderer's
``get_text_width_height_descent`` for lines the cache lacks, as
matplotlib's does. The renderer is also the one the program's replacement
of ``_get_text_metrics_with_cache`` receives meanwhile
(`current_renderer`).
"""

from collections import namedtuple
import contextlib
import threading

from ._native import text_overrides as _native
from .font_manager import FontManager

#: What a cache has done (the fields of ``functools.lru_cache``'s
#: ``cache_info()``).
CacheInfo = namedtuple("CacheInfo", ["hits", "misses", "maxsize", "currsize"])


class _Installed(threading.local):
    """What each thread lays out and draws with, as the cache installed
    natively is the thread's."""

    def __init__(self):
        #: The renderers pltrs (Rust) lays out and draws with, innermost
        #: last.
        self.renderers = []


_installed = _Installed()


def current_renderer():
    """The renderer pltrs (Rust) lays out or draws with now on this thread,
    if any."""
    renderers = _installed.renderers
    return renderers[-1] if renderers else None


def _measures_as(renderer):
    """How *renderer* measures the lines pltrs (Rust) lays out with it: as
    its vector backend (``_text_metrics`` of its class: ``"pdf"``,
    ``"ps"``, ``"svg"``), that of its vector renderer for a mixed-mode
    renderer, else as Agg (``"raster"``)."""
    measuring = getattr(renderer, "_vector_renderer", renderer)
    return getattr(type(measuring), "_text_metrics", "raster")


class TextMetrics:
    """
    The text metrics cache of the renderer *renderer_ref* refers to:
    calling it with ``(text, fontprop, ismath, dpi)`` gives the width,
    height and descent of the line, from the cache or measured by the
    renderer.
    """

    def __init__(self, renderer_ref):
        self._renderer_ref = renderer_ref
        self._native = _native.new_metrics_cache(_measures_as(renderer_ref()))

    def __call__(self, text, fontprop, ismath, dpi):
        def measure():
            # this should never happen under normal use, but this is a better
            # error to raise than an AttributeError on `None`
            if (local_renderer := self._renderer_ref()) is None:
                raise RuntimeError(
                    "Trying to get text metrics for a renderer that no longer "
                    "exists.  This should never happen and is evidence of a bug "
                    "elsewhere."
                )
            return local_renderer.get_text_width_height_descent(
                text, fontprop, ismath)

        return _native.metrics_cache_call(
            self._native, text, FontManager._props(fontprop), ismath,
            float(dpi), measure)

    def cache_info(self):
        """Report the cache's statistics."""
        return CacheInfo(*_native.metrics_cache_info(self._native))

    def cache_clear(self):
        """Clear the cache and its statistics."""
        _native.metrics_cache_clear(self._native)


@contextlib.contextmanager
def measuring_with(renderer, metrics):
    """Lay out and draw with *renderer* on this thread, its text metrics
    cache *metrics* (a `TextMetrics`) installed natively."""
    previous = _native.set_metrics_cache(metrics._native)
    renderers = _installed.renderers
    renderers.append(renderer)
    try:
        yield
    finally:
        renderers.pop()
        _native.set_metrics_cache(previous)


@contextlib.contextmanager
def measured_at(dpi):
    """Have pltrs (Rust) measure what matplotlib measures with the renderer
    it is handed at *dpi* on this thread (None: at the resolution the
    figure is laid out at), as a renderer of that resolution measures it:
    every line of text, and the pads matplotlib converts with the
    renderer; the texts are still placed at their figure's (see
    `measured_by`)."""
    previous = _native.set_measured_at(dpi)
    try:
        yield
    finally:
        _native.set_measured_at(previous)
