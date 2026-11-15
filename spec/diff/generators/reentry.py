"""Programs whose callbacks enter their figure once: a callback that
native code calls (a formatter, a transform, a norm, a draw event, an
artist's ``draw`` of the program's...) asks the figure something, changes
it or draws it, a single time, as programs do.

matplotlib's figure is plain Python, so such a call works there, or fails
with matplotlib's own error. pltrs (Rust) may hold the figure while it
calls back; the tester compares what the operation gave or raised inside
the callback, what the figure holds afterwards and the figure's image.

Each program makes the scene (a figure with a line and a text), installs
one callback (``KINDS``), whose first call runs one operation
(``OPERATIONS``) and keeps its value or exception in ``box``, then runs
what makes native code call back and reads the figure. The callbacks are
defined through ``exec`` so that the tracer records only ``box``, not how
often each implementation happens to call them.
"""

from __future__ import annotations

from typing import Iterator

#: The helper every program has: ``once()`` runs ``operation`` at its
#: first call and keeps what it gave in ``box``.
HELPERS = '''
box = {"called": False}
def once():
    if box["called"]:
        return
    box["called"] = True
    try:
        box["value"] = operation()
    except Exception as error:
        box["raised"] = f"{type(error).__name__}: {error}"
'''

#: The scene: a figure with a line and a text (made by ``FIGURE``).
SCENE = [
    "ax = fig.add_subplot()",
    "line, = ax.plot([1, 2, 3], [1, 4, 9], label='line')",
    "text = ax.text(0.5, 0.5, 'text')",
    "fig.canvas.draw()",
]


class Kind:
    """A kind of callback: the code defining and installing it (run by
    ``exec``; it calls ``once()``), what makes native code call it, and
    how the figure is made (another class for a ``Figure.draw``)."""

    def __init__(self, install: str, trigger: list[str], figure: str = "fig = plt.figure()"):
        self.install = install
        self.trigger = trigger
        self.figure = figure


DRAW = ["fig.canvas.draw()"]

#: A transform of the program's whose non-affine part calls back.
SHIFTING = '''
class Shifting(transforms.Transform):
    input_dims = output_dims = 2
    def transform_non_affine(self, values):
        once()
        return values'''

#: A locator of the program's whose ``tick_values`` calls back.
LOCATING = '''
class Locating(ticker.MaxNLocator):
    def tick_values(self, vmin, vmax):
        once()
        return super().tick_values(vmin, vmax)'''

#: A scale of the program's whose transform calls back, registered.
PLAIN_SCALE = '''
class Identity(transforms.Transform):
    input_dims = output_dims = 1
    def transform_non_affine(self, values):
        once()
        return values
    def inverted(self):
        return Identity()
class Plain(scale.ScaleBase):
    name = "plain"
    def get_transform(self):
        return Identity()
    def set_default_locators_and_formatters(self, axis):
        axis.set_major_locator(ticker.AutoLocator())
        axis.set_major_formatter(ticker.ScalarFormatter())
scale.register_scale(Plain)'''

#: Values of a class of the program's, and the head of their converter.
THINGS = '''
class Thing:
    def __init__(self, value):
        self.value = value
class Converting(units.ConversionInterface):'''
TIGHT = ["fig.savefig(io.BytesIO(), format='png', bbox_inches='tight')"]

KINDS: dict[str, Kind] = {
    "formatter": Kind(
        "ax.xaxis.set_major_formatter(ticker.FuncFormatter(lambda v, pos: (once(), f'{v:g}')[1]))", DRAW),
    "locator-call": Kind('''
class Locating(ticker.MaxNLocator):
    def __call__(self):
        once()
        return super().__call__()
ax.xaxis.set_major_locator(Locating())''', DRAW),
    "locator-tick_values": Kind(LOCATING + "\nax.xaxis.set_major_locator(Locating())", DRAW),
    # Locators the axes of a projection wraps (polar) or asks otherwise (3-D).
    "locator-tick_values-polar": Kind(LOCATING + '''
polar = fig.add_axes([0.6, 0.6, 0.3, 0.3], projection="polar")
polar.plot([0, 1], [0, 1])
polar.yaxis.set_major_locator(Locating())''', DRAW),
    "locator-tick_values-3d": Kind(LOCATING + '''
space = fig.add_axes([0.6, 0.6, 0.3, 0.3], projection="3d")
space.plot([0, 1], [0, 1], [0, 1])
space.xaxis.set_major_locator(Locating())''', DRAW),
    "transform-non_affine": Kind(SHIFTING + "\nline.set_transform(Shifting() + ax.transData)", DRAW),
    # Callbacks of native calls changing the figure (not drawing it).
    "transform-non_affine-plot": Kind(SHIFTING, ["ax.plot([1, 2], [3, 4], transform=Shifting() + ax.transData)"]),
    "transform-non_affine-scatter": Kind(
        SHIFTING, ["ax.scatter([1, 2], [3, 4], transform=Shifting() + ax.transData)"]),
    "transform-non_affine-relim": Kind(
        SHIFTING + "\nline.set_transform(Shifting() + ax.transData)", ["ax.relim()", "ax.autoscale_view()"]),
    "transform-affine": Kind('''
class Placing(transforms.Affine2D):
    def transform_affine(self, values):
        once()
        return super().transform_affine(values)
text.set_transform(Placing().translate(200, 150))''', DRAW),
    "scale-transform": Kind(PLAIN_SCALE + "\nax.set_xscale('plain')", DRAW),
    "scale-transform-set_xscale": Kind(PLAIN_SCALE, ["ax.set_xscale('plain')"]),
    "transform-inverted": Kind('''
class Identity(transforms.Transform):
    input_dims = output_dims = 1
    def transform_non_affine(self, values):
        return values
    def inverted(self):
        once()
        return Identity()
class Plain(scale.ScaleBase):
    name = "plain"
    def get_transform(self):
        return Identity()
    def set_default_locators_and_formatters(self, axis):
        axis.set_major_locator(ticker.AutoLocator())
        axis.set_major_formatter(ticker.ScalarFormatter())
scale.register_scale(Plain)
ax.set_xscale("plain")
fig.canvas.draw()''', ["MouseEvent('motion_notify_event', fig.canvas, 320, 240)._process()"]),
    "norm-call": Kind('''
class Norming(colors.Normalize):
    def __call__(self, value, clip=None):
        once()
        return super().__call__(value, clip)
image = ax.imshow(np.arange(12.0).reshape(3, 4), norm=Norming(), extent=(1, 3, 1, 9))''', DRAW),
    "norm-autoscale_None": Kind('''
class Norming(colors.Normalize):
    def autoscale_None(self, A):
        once()
        return super().autoscale_None(A)
norm = Norming()''', ["image = ax.imshow(np.arange(12.0).reshape(3, 4), norm=norm, extent=(1, 3, 1, 9))", *DRAW]),
    "colormap-call": Kind('''
class Mapping(colors.ListedColormap):
    def __call__(self, X, alpha=None, bytes=False):
        once()
        return super().__call__(X, alpha, bytes)
image = ax.imshow(np.arange(12.0).reshape(3, 4), cmap=Mapping(["red", "green", "blue"]), extent=(1, 3, 1, 9))''',
        DRAW),
    "units-convert": Kind('''
class Thing:
    def __init__(self, value):
        self.value = value
class Converting(units.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        once()
        return np.asarray([thing.value for thing in np.atleast_1d(value)], dtype=float)
units.registry[Thing] = Converting()
inset = fig.add_axes([0.6, 0.6, 0.25, 0.25])''', ["inset.plot([Thing(1), Thing(2)], [1, 2])", *DRAW]),
    "units-convert-set_xlim": Kind(THINGS + '''
    @staticmethod
    def convert(value, unit, axis):
        if np.ndim(value) == 0:
            once()
        return np.asarray([thing.value for thing in np.atleast_1d(value)], dtype=float)
units.registry[Thing] = Converting()
inset = fig.add_axes([0.6, 0.6, 0.25, 0.25])
inset.plot([Thing(1), Thing(2)], [1, 2])''', ["inset.set_xlim(Thing(0), Thing(5))", *DRAW]),
    "units-axisinfo": Kind('''
class Thing:
    def __init__(self, value):
        self.value = value
class Converting(units.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        return np.asarray([thing.value for thing in np.atleast_1d(value)], dtype=float)
    @staticmethod
    def axisinfo(unit, axis):
        once()
        return units.AxisInfo(label="things")
units.registry[Thing] = Converting()
inset = fig.add_axes([0.6, 0.6, 0.25, 0.25])''', ["inset.plot([Thing(1), Thing(2)], [1, 2])", *DRAW]),
    "units-default_units": Kind('''
class Thing:
    def __init__(self, value):
        self.value = value
class Converting(units.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        return np.asarray([thing.value for thing in np.atleast_1d(value)], dtype=float)
    @staticmethod
    def default_units(x, axis):
        once()
        return "thing"
units.registry[Thing] = Converting()
inset = fig.add_axes([0.6, 0.6, 0.25, 0.25])''', ["inset.plot([Thing(1), Thing(2)], [1, 2])", *DRAW]),
    "path_effect": Kind('''
class Effect(patheffects.AbstractPathEffect):
    def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
        once()
        renderer.draw_path(gc, tpath, affine, rgbFace)
line.set_path_effects([Effect()])''', DRAW),
    "artist-draw": Kind('''
class Drawing(artist.Artist):
    def draw(self, renderer):
        once()
ax.add_artist(Drawing())''', DRAW),
    "artist-get_window_extent": Kind('''
class Measured(artist.Artist):
    def draw(self, renderer):
        pass
    def get_window_extent(self, renderer=None):
        once()
        return transforms.Bbox([[100, 100], [200, 150]])
measured = Measured()
ax.add_artist(measured)
measured.set_clip_on(False)''', TIGHT),
    "artist-get_tightbbox": Kind('''
class Measured(artist.Artist):
    def draw(self, renderer):
        pass
    def get_tightbbox(self, renderer=None):
        once()
        return transforms.Bbox([[100, 100], [200, 150]])
measured = Measured()
ax.add_artist(measured)
measured.set_clip_on(False)''', TIGHT),
    "agg_filter": Kind("line.set_agg_filter(lambda image, dpi: (once(), (image, 0, 0))[1])", DRAW),
    "draw_event": Kind("fig.canvas.mpl_connect('draw_event', lambda event: once())", DRAW),
    "resize_event": Kind("fig.canvas.mpl_connect('resize_event', lambda event: once())",
                         ["ResizeEvent('resize_event', fig.canvas)._process()"]),
    "xlim_changed": Kind("ax.callbacks.connect('xlim_changed', lambda changed: once())", ["ax.set_xlim(0, 4)"]),
    "axes_locator": Kind(
        "ax.set_axes_locator(lambda axes, renderer: (once(), transforms.Bbox.from_bounds(0.1, 0.1, 0.8, 0.8))[1])",
        DRAW),
    "layout_engine": Kind('''
class Laying(layout_engine.LayoutEngine):
    _adjust_compatible = True
    _colorbar_gridspec = False
    def execute(self, fig):
        once()
fig.set_layout_engine(Laying())''', DRAW),
    "figure-draw": Kind('''
class Drawn(figure.Figure):
    def draw(self, renderer):
        once()
        return super().draw(renderer)''', DRAW, "exec(KIND)\nfig = plt.figure(FigureClass=Drawn)"),
    "legend_handler": Kind('''
class Handling(legend_handler.HandlerLine2D):
    def legend_artist(self, legend, orig_handle, fontsize, handlebox):
        once()
        return super().legend_artist(legend, orig_handle, fontsize, handlebox)''',
        ["legend = ax.legend(handler_map={line: Handling()})", *DRAW]),
}

#: What a program does in the callback once: read, change or draw.
OPERATIONS: dict[str, str] = {
    "get_size_inches": "fig.get_size_inches()",
    "get_xlim": "ax.get_xlim()",
    "transData": "ax.transData.transform([[1, 2]])",
    "axes-extent": "ax.get_window_extent()",
    "figure-tightbbox": "fig.get_tightbbox()",
    "figure-tightbbox-renderer": "fig.get_tightbbox(fig.canvas.get_renderer())",
    "axes-tightbbox-renderer": "ax.get_tightbbox(fig.canvas.get_renderer())",
    "get_children": "[type(child).__name__ for child in ax.get_children()]",
    "text-extent": "text.get_window_extent()",
    "set_xlim": "ax.set_xlim(0, 10)",
    "set_title": "ax.set_title('reentered')",
    "set_data": "line.set_data([0, 1], [1, 0])",
    "add_patch": "ax.add_patch(patches.Rectangle((0, 0), 1, 1))",
    "remove": "text.remove()",
    "clear": "ax.clear()",
    "canvas-draw": "fig.canvas.draw()",
    "draw_without_rendering": "fig.draw_without_rendering()",
    "savefig": "fig.savefig(io.BytesIO())",
    "draw_idle": "fig.canvas.draw_idle()",
    # Artists made and measured, which the figure leaves loose (a locator
    # making a tick to measure its label, as getdist's does).
    "make-tick": "(lambda tick: (tick.label1.set_text('0.5'), "
                 "tick.label1.get_window_extent(fig.canvas.get_renderer()).bounds)[1])"
                 "(ax.xaxis._get_tick(major=True))",
    "make-text": "mtext.Text(0, 0, 'made', figure=fig).get_window_extent(fig.canvas.get_renderer()).bounds",
    "make-line": "(lambda made: (made.set_figure(fig), "
                 "made.get_window_extent(fig.canvas.get_renderer()).bounds)[1])(lines.Line2D([0, 1], [0, 1]))",
    "make-patch": "(lambda made: (made.set_figure(fig), made.set_transform(ax.transData), "
                  "made.get_window_extent(fig.canvas.get_renderer()).bounds)[2])"
                  "(patches.Rectangle((0, 0), 1, 1))",
}

IMPORTS = [
    "import io",
    "import numpy as np",
    "import matplotlib",
    "matplotlib.use('agg')",
    "import matplotlib.pyplot as plt",
    "from matplotlib import (artist, colors, figure, layout_engine, legend_handler, lines, patches, patheffects, "
    "scale, ticker, transforms, units)",
    "import matplotlib.text as mtext",
    "from matplotlib.backend_bases import MouseEvent, ResizeEvent",
]

#: What the program reads once the callback ran: what the operation gave,
#: then the figure, drawn again.
AFTER = [
    "box",
    "ax.get_xlim()",
    "ax.get_title()",
    "[type(child).__name__ for child in ax.get_children()]",
    "line.get_xydata().tolist()",
    "text in ax.texts",
    "fig.canvas.draw()",
    "fig.savefig(io.BytesIO(), format='png')",
    "box['called']",
]


def program(kind: Kind, operation: str) -> list[str]:
    """The statements of the program entering the figure from a callback
    of *kind* with *operation*."""
    install = f'exec("""{kind.install}""")'
    figure = kind.figure.replace("exec(KIND)", install).splitlines()
    installed = [] if install in figure else [install]
    return [*IMPORTS, f'exec("""{HELPERS}""")', f"operation = lambda: {operation}", *figure, *SCENE,
            *installed, *kind.trigger, *AFTER]


def programs() -> Iterator[tuple[str, list[str]]]:
    """A program per kind of callback and operation."""
    for kind_name, kind in KINDS.items():
        for operation_name, operation in OPERATIONS.items():
            yield f"{kind_name}--{operation_name}", program(kind, operation)
