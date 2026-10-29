"""
The canvas and manager every figure has (matplotlib's ``backend_bases``).

plt-rs renders figures itself, whatever the backend: the canvas here
draws and prints through the figure's native handle, and the manager is
what pyplot registers in `._pylab_helpers.Gcf`.
"""

import itertools


class FigureCanvasBase:
    """
    The canvas the figure renders into.

    Attributes
    ----------
    figure : `~matplotlib.figure.Figure`
        A high-level figure instance.
    """

    def __init__(self, figure=None):
        self.figure = figure
        self.manager = None
        self._callbacks = {}
        self._cids = itertools.count(1)
        if figure is not None:
            figure.canvas = self

    def draw(self, *args, **kwargs):
        """Render the `.Figure`."""
        self.figure.savefig_bytes("png")

    def draw_idle(self, *args, **kwargs):
        """Request a widget redraw: pltrs draws at once."""
        self.draw()

    def get_width_height(self, *, physical=False):
        """Return the figure width and height in integral points or pixels."""
        width, height = self.figure.get_size_inches()
        dpi = self.figure.get_dpi()
        return int(width * dpi), int(height * dpi)

    def print_figure(self, filename, **kwargs):
        """Render the figure to hardcopy (`.Figure.savefig`)."""
        self.figure.savefig(filename, **kwargs)

    def mpl_connect(self, s, func):
        """
        Bind function *func* to event *s*; return a connection id.

        pltrs's canvases emit no events yet: the connection is kept, so that
        `mpl_disconnect` works, but never called.
        """
        cid = next(self._cids)
        self._callbacks[cid] = (s, func)
        return cid

    def mpl_disconnect(self, cid):
        """Disconnect the callback with id *cid*."""
        self._callbacks.pop(cid, None)


class FigureManagerBase:
    """
    A backend-independent abstraction of a figure container and controller.

    Attributes
    ----------
    canvas : `FigureCanvasBase`
        The backend-specific canvas instance.
    num : int or str
        The figure number.
    """

    def __init__(self, canvas, num):
        self.canvas = canvas
        canvas.manager = self
        self.num = num
        self._window_title = f"Figure {num:d}" if isinstance(num, int) else str(num)

    def destroy(self):
        """Close the figure's window (there is none without a GUI)."""

    def get_window_title(self):
        """Return the title text of the window containing the figure."""
        return self._window_title

    def set_window_title(self, title):
        """Set the title text of the window containing the figure."""
        self._window_title = title
