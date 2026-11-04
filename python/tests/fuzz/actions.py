"""
The fuzzer's own programs: random calls of the public API, chosen while
the program runs.

A program starts from `HEADER` and takes `steps` actions, each one line
of Python source that `Program.next_line` writes from what the program
has made so far (its figures, axes, artists and other objects, found by
looking at them without running their code) and from a `random.Random`
seeded with the program's number: a plotting call, a property set or read, a public method called
with arguments for its signature, a draw or a save in some format, a
pickle round trip, mouse and key events, a callback that changes the
figure while it is drawn, an rcParam, a removal. The lines run in the
worker as they are written, so the program can always be written out and
run again as a plain script.
"""

import inspect
import random

from . import values

#: The first lines of every program.
HEADER = [
    "import io, pickle",
    "import numpy as np",
    "import pltrs",
    "pltrs.use('agg')",
    "import pltrs.pyplot as plt",
    "from pltrs import colors, ticker, transforms, patches, lines, text, collections, path",
    "from pltrs import patheffects, markers, scale",
    "from pltrs.backend_bases import MouseEvent, KeyEvent",
    "fig, ax = plt.subplots()",
]

#: Plotting methods of `Axes` and the positional arguments they get.
PLOTTING = {
    "plot": "aa", "scatter": "aa", "bar": "aa", "barh": "aa", "hist": "a", "step": "aa",
    "stem": "a", "fill_between": "aa", "fill_betweenx": "aa", "errorbar": "aa", "pie": "a",
    "stackplot": "aa", "stairs": "a", "eventplot": "a", "hlines": "aaa", "vlines": "aaa",
    "axhline": "n", "axvline": "n", "axhspan": "nn", "axvspan": "nn", "axline": "pp",
    "text": "nns", "annotate": "sp", "imshow": "f", "matshow": "f", "pcolormesh": "f",
    "pcolor": "f", "pcolorfast": "f", "contour": "f", "contourf": "f", "spy": "f",
    "hexbin": "aa", "hist2d": "aa", "quiver": "aaaa", "barbs": "aaaa", "boxplot": "a",
    "violinplot": "a", "fill": "aa", "loglog": "aa", "semilogx": "aa", "semilogy": "aa",
    "broken_barh": "pp", "tricontour": "aaa", "tripcolor": "aaa", "triplot": "aa",
    "specgram": "a", "psd": "a", "magnitude_spectrum": "a", "acorr": "a", "streamplot": "aaff",
    "ecdf": "a", "legend": "", "table": "", "grid": "", "set_title": "s", "set_xlabel": "s",
    "secondary_xaxis": "s", "inset_axes": "p", "indicate_inset_zoom": "", "bar_label": "",
}

#: Plotting methods of 3-D axes and the positional arguments they get.
PLOTTING_3D = {
    "plot": "aaa", "scatter": "aaa", "plot_surface": "fff", "plot_wireframe": "fff",
    "plot_trisurf": "aaa", "contour": "fff", "contourf": "fff", "bar3d": "aaaaaa",
    "quiver": "aaaaaa", "voxels": "f", "text": "nnns", "stem": "aaa", "fill_between": "aaaaaa",
    "errorbar": "aaa", "set_zlim": "nn", "view_init": "nnn", "set_box_aspect": "p",
}

#: Values of an axes' rectangle, in figure fractions: an axes ten or a
#: hundred figures wide makes the tight box of a save gigabytes large, in
#: matplotlib as in pltrs, so the large ones are those either refuses.
RECT_VALUES = ["0", "0.1", "0.5", "0.9", "1", "-0.5", "2", "1e-9", "float('nan')",
               "float('inf')", "1e300", "-1e300"]

#: Keywords plotting methods get now and then.
PLOT_KEYWORDS = ["color", "alpha", "linewidth", "linestyle", "marker", "label", "zorder",
                 "cmap", "norm", "vmin", "vmax", "transform", "clip_on", "hatch", "edgecolor",
                 "facecolor", "s", "c", "bins", "levels", "orientation", "interpolation",
                 "aspect", "align", "width", "picker", "rasterized", "snap", "fontsize"]

#: Methods the fuzzer never calls: they block, open windows, or end the
#: program.
BLOCKING = {"show", "pause", "ginput", "waitforbuttonpress", "start_event_loop",
            "stop_event_loop", "flush_events", "draw_idle", "close", "mainloop", "connect",
            "savefig", "print_figure", "set_window_title", "new_timer", "manager"}

#: The formats figures are saved in.
FORMATS = ["png", "pdf", "svg", "svgz", "ps", "eps", "pgf", "raw", "rgba", "jpg", "tiff",
           "webp", "gif"]

#: Places where the program's code runs while a figure is laid out or
#: drawn, as templates around the name of a function to run there.
CALLBACKS = [
    "ax.xaxis.set_major_formatter(ticker.FuncFormatter(lambda v, p: ({f}(), str(v))[1]))",
    "ax.yaxis.set_major_locator(type('L', (ticker.MaxNLocator,), "
    "{{'__call__': lambda self: ({f}(), ticker.MaxNLocator.__call__(self))[1]}})())",
    "fig.canvas.mpl_connect('draw_event', lambda e: {f}())",
    "ax.callbacks.connect('xlim_changed', lambda a: {f}())",
    "fig.canvas.mpl_connect('button_press_event', lambda e: {f}())",
    "fig.canvas.mpl_connect('pick_event', lambda e: {f}())",
    "ax.add_artist(type('A', (pltrs.artist.Artist,), "
    "{{'draw': lambda self, r: {f}()}})())",
    "ax.plot([0, 1], [1, 0], path_effects=[type('E', (patheffects.AbstractPathEffect,), "
    "{{'draw_path': lambda self, r, gc, tp, af, rgb=None: ({f}(), "
    "r.draw_path(gc, tp, af, rgb))}})()])",
    "ax.imshow([[0, 1], [2, 3]], norm=type('N', (colors.Normalize,), "
    "{{'__call__': lambda self, v, clip=None: ({f}(), colors.Normalize.__call__(self, v, clip))[1]}})())",
    "ax.plot([1, 2], [1, 2], transform=type('T', (transforms.Transform,), "
    "{{'input_dims': 2, 'output_dims': 2, 'transform_non_affine': lambda self, v: ({f}(), v)[1]}})()"
    " + ax.transData)",
]


def _is_method(obj, name):
    """Whether *name* is a method of *obj*, found without running any of
    its code (a property's getter would change what the program is)."""
    found = inspect.getattr_static(obj, name, None)
    return (inspect.isfunction(found) or inspect.isbuiltin(found)
            or inspect.ismethoddescriptor(found)
            or isinstance(found, (classmethod, staticmethod)))


class Program:
    """A random program being written and run: `next_line` writes its
    next line, which the caller runs in `namespace`, then hands the
    outcome to `ran`."""

    def __init__(self, seed):
        self.rng = random.Random(seed)
        self.values = values.Values(self.rng, self.rng.choice([0.05, 0.2, 0.5]))
        self.namespace = {}
        self.figures = ["fig"]
        self.axes = ["ax"]
        self.artists = []
        self.others = ["ax.xaxis", "ax.yaxis", "fig.canvas"]
        self.count = 0

    def _new_name(self):
        self.count += 1
        return f"v{self.count}"

    def _live(self, names):
        """The names of *names* still bound to something."""
        return [name for name in names if self._value(name) is not None]

    def _value(self, expression):
        try:
            return eval(expression, self.namespace)  # noqa: S307 - the program's own names
        except Exception:  # noqa: BLE001 - a name whose object went wrong
            return None

    def _choose(self, names):
        live = self._live(names)
        return values.choose(self.rng, live) if live else None

    def next_line(self):
        """The program's next line, an action chosen at random."""
        rng = self.rng
        roll = rng.random()
        for weight, action in [(0.22, self._plot), (0.20, self._set), (0.08, self._get),
                               (0.12, self._method), (0.12, self._draw_or_save),
                               (0.03, self._pickle), (0.05, self._event), (0.05, self._callback),
                               (0.05, self._new_axes), (0.03, self._rc), (0.03, self._remove)]:
            if roll < weight:
                line = action()
                if line is not None:
                    return line
                break
            roll -= weight
        return self._plot() or "fig.canvas.draw()"

    def ran(self, line):
        """Records what the line just run made: its result's figures, axes
        and artists become choices of later actions."""
        if not line.startswith("v") or " = " not in line:
            return
        name = line.split(" = ", 1)[0]
        value = self.namespace.get(name)
        artist = self.namespace["pltrs"].artist.Artist
        figure = self.namespace["pltrs"].figure.FigureBase
        axes = self.namespace["pltrs"].axes.Axes
        kinds = [(figure, self.figures), (axes, self.axes), (artist, self.artists)]
        array = self.namespace["np"].ndarray
        if isinstance(value, array) and value.dtype == object:
            # Grids of subfigures and axes: their items are pltrs's; the
            # arrays themselves are numpy's, whose methods are not fuzzed
            # here (some crash numpy, as under matplotlib).
            for index, item in enumerate(value.flat[:4]):
                for kind, names in kinds:
                    if isinstance(item, kind):
                        names.append(f"{name}.flat[{index}]")
                        break
        elif isinstance(value, (list, tuple)):
            items = list(value)[:3]
            self.artists += [f"{name}[{index}]" for index, item in enumerate(items)
                             if isinstance(item, artist)]
            if not any(isinstance(item, artist) for item in items):
                self.others.append(name)
        elif not isinstance(value, array) and value is not None:
            for kind, names in kinds:
                if isinstance(value, kind):
                    names.append(name)
                    break
            else:
                self.others.append(name)

    # The actions: each returns a line, or None when it has nothing to act on.

    def _plot(self):
        ax = self._choose(self.axes)
        if ax is None:
            return None
        table = PLOTTING_3D if getattr(self._value(ax), "name", None) == "3d" else PLOTTING
        method = values.choose(self.rng, list(table))
        args = [self._positional(kind) for kind in table[method]]
        for _ in range(self.rng.randrange(3)):
            keyword = values.choose(self.rng, PLOT_KEYWORDS)
            args.append(f"{keyword}={self.values.for_parameter(keyword)}")
        return f"{self._new_name()} = {ax}.{method}({', '.join(args)})"

    def _positional(self, kind):
        if kind == "a":
            return self.values.array()
        if kind == "f":
            return self.values.field()
        if kind == "n":
            return self.values.number()
        if kind == "s":
            return self.values.string()
        return f"({self.values.number()}, {self.values.number()})"

    def _target(self):
        return self._choose(self.artists + self.axes + self.figures + self.others)

    def _set(self):
        target = self._target()
        if target is None:
            return None
        setters = [name for name in dir(self._value(target)) if name.startswith("set_")]
        if not setters:
            return None
        setter = values.choose(self.rng, setters)
        value = self.values.for_parameter(setter[4:])
        return f"{target}.{setter}({value})"

    def _get(self):
        target = self._target()
        if target is None:
            return None
        getters = [name for name in dir(self._value(target))
                   if name.startswith(("get_", "is_")) and name not in BLOCKING]
        return f"{target}.{values.choose(self.rng, getters)}()" if getters else None

    def _method(self):
        target = self._target()
        if target is None:
            return None
        obj = self._value(target)
        names = [name for name in dir(obj)
                 if not name.startswith("_") and name not in BLOCKING
                 and _is_method(obj, name)]
        if not names:
            return None
        name = values.choose(self.rng, names)
        try:
            parameters = list(inspect.signature(getattr(obj, name)).parameters.values())
        except (TypeError, ValueError):
            parameters = []
        args = []
        for parameter in parameters:
            if parameter.kind in (parameter.VAR_POSITIONAL, parameter.VAR_KEYWORD):
                continue
            required = parameter.default is parameter.empty
            if required or self.rng.random() < 0.3:
                value = self.values.for_parameter(parameter.name)
                keyword = parameter.kind == parameter.KEYWORD_ONLY or not required
                args.append(f"{parameter.name}={value}" if keyword else value)
        return f"{self._new_name()} = {target}.{name}({', '.join(args)})"

    def _draw_or_save(self):
        fig = self._choose(self.figures)
        if fig is None:
            return None
        rng = self.rng
        kind = rng.randrange(5)
        if kind == 0:
            return f"{fig}.canvas.draw()"
        if kind == 1:
            return f"{fig}.tight_layout()"
        if kind == 2:
            return f"{fig}.get_tightbbox()"
        keywords = [f"format={values.choose(rng, FORMATS)!r}"]
        if rng.random() < 0.5:
            keywords.append(f"dpi={values.choose(rng, ['10', '72', '100', '0', '-5', '1e-3', 'float(\"nan\")'])}")
        if rng.random() < 0.4:
            keywords.append(f"bbox_inches={self.values.named('bbox_inches')}")
        if rng.random() < 0.3:
            keywords.append(f"transparent={values.choose(rng, ['True', 'False'])}")
        if rng.random() < 0.2:
            keywords.append(f"facecolor={self.values.color()}")
        return f"{fig}.savefig(io.BytesIO(), {', '.join(keywords)})"

    def _pickle(self):
        fig = self._choose(self.figures[:1] + [f for f in self.figures if f.startswith("v")])
        if fig is None:
            return None
        name = self._new_name()
        return f"{name} = pickle.loads(pickle.dumps({fig})); {name}.canvas.draw()"

    def _event(self):
        rng = self.rng
        x, y = rng.choice(["50", "200", "-10", "1e9", "float('nan')"]), rng.choice(["50", "300", "0"])
        kind = rng.randrange(4)
        if kind == 0:
            button = rng.choice(["1", "3", "'up'", "None"])
            name = rng.choice(["button_press_event", "button_release_event", "motion_notify_event"])
            return f"MouseEvent({name!r}, fig.canvas, {x}, {y}, {button})._process()"
        if kind == 1:
            step = rng.choice(["1", "-1", "0", "1e300"])
            return f"MouseEvent('scroll_event', fig.canvas, {x}, {y}, 'up', step={step})._process()"
        if kind == 2:
            key = rng.choice(["'q'", "'ctrl+z'", "'l'", "'k'", "'g'", "'f'", "'s'", "'home'",
                              "'left'", "None", "'p'", "'o'"])
            return f"KeyEvent('key_press_event', fig.canvas, {key}, {x}, {y})._process()"
        artist = self._choose(self.artists)
        if artist is None:
            return None
        return f"{artist}.set_picker({self.values.named('picker')})"

    def _callback(self):
        """Has a function run while the figure is drawn (in a formatter,
        a locator, a path effect, a norm, a transform, a callback), whose
        body is another random action of the program's."""
        body = None
        for _ in range(5):
            body = self.next_line() if self.rng.random() < 0.9 else None
            if body and not body.startswith(("fig.canvas.draw", "v", "def")) and ";" not in body:
                break
            body = None
        if body is None:
            return None
        self.count += 1
        function = f"_act{self.count}"
        template = values.choose(self.rng, CALLBACKS)
        return f"def {function}():\n    {body}\n{template.format(f=function)}"

    def _new_axes(self):
        rng = self.rng
        fig = self._choose(self.figures)
        if fig is None:
            return None
        kind = rng.randrange(6)
        projection = rng.choice(["None", "'polar'", "'3d'", "'aitoff'", "'rectilinear'"])
        if kind == 0:
            line = f"{fig}.add_subplot({rng.choice(['111', '2, 2, 3', '1, 2, 2', '0', '5, 5, 99'])}, projection={projection})"
        elif kind == 1:
            rect = ", ".join(rng.choice(RECT_VALUES) for _ in range(4))
            line = f"{fig}.add_axes([{rect}])"
        elif kind == 2:
            ax = self._choose(self.axes)
            line = f"{ax}.{rng.choice(['twinx', 'twiny'])}()" if ax else None
        elif kind == 3:
            line = f"plt.figure(figsize=({self.values.number()}, {self.values.number()}), dpi={rng.choice(['50', '100', '0', '1e-9', '1e9'])})"
        elif kind == 4:
            line = f"{fig}.subfigures({rng.choice(['1', '2', '0', '-1'])}, {rng.choice(['1', '2'])})"
        else:
            line = f"plt.subplots({rng.choice(['2', '1', '0'])}, {rng.choice(['2', '1'])}, layout={rng.choice(['None', repr('constrained'), repr('tight'), repr('compressed')])})"
        return f"{self._new_name()} = {line}" if line else None

    def _rc(self):
        rcparams = self.namespace["pltrs"].rcParams
        key = values.choose(self.rng, sorted(rcparams.keys()))
        return f"plt.rcParams[{key!r}] = {self.values.for_parameter(key.split('.')[-1])}"

    def _remove(self):
        rng = self.rng
        kind = rng.randrange(4)
        if kind == 0:
            artist = self._choose(self.artists)
            return f"{artist}.remove()" if artist else None
        if kind == 1:
            ax = self._choose(self.axes)
            return f"{ax}.{rng.choice(['cla', 'clear', 'remove', 'relim', 'autoscale_view'])}()" if ax else None
        if kind == 2:
            fig = self._choose(self.figures)
            return f"{fig}.{rng.choice(['clf', 'clear'])}()" if fig else None
        return "plt.close('all')" if rng.random() < 0.2 else "plt.gcf().canvas.draw()"
