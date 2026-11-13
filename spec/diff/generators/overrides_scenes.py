"""The lives of the classes the ``overrides`` programs subclass.

A scene says how a program makes an object of ``Sub`` (the subclass
overriding one member) and puts it where matplotlib uses it: ``make``
leaves the figure in ``fig``, an Axes in ``ax``, the object in ``obj`` and
the artist carrying it in ``art`` (the object itself for an artist, the
line drawn through a transform, the image normalised by a norm...).
``impl`` is the source of ``Impl``, a working subclass of an abstract base
of matplotlib's (a ``Locator`` computes no ticks, a ``Transform`` has no
dimensions) that ``Sub`` derives from instead; ``hosts`` is the source of
the classes putting ``Sub`` in place (an Axes whose ``_init_axis`` makes
an axis of ``Sub``). ``kind`` chooses the life's end: an ``artist`` is
removed from its Axes, an ``axes`` cleared and removed, a ``figure``
cleared, a ``value`` dropped with its Axes' clear.
"""

from __future__ import annotations

from dataclasses import dataclass

from . import plotting


@dataclass(frozen=True)
class Scene:
    """How a program makes and places an object of one class."""

    #: The base class as the program spells it.
    base: str
    #: The statements making ``fig``, ``ax``, ``obj`` and ``art``.
    make: tuple[str, ...]
    #: A property set between two draws.
    prop: str
    #: The life's end: ``artist``, ``axes``, ``figure`` or ``value``.
    kind: str = "artist"
    #: The source of ``Impl``, a working class over an abstract base.
    impl: str = ""
    #: The source of the classes placing ``Sub``.
    hosts: str = ""
    #: Steps of the class' own kind, between the draws and the measuring.
    steps: tuple[str, ...] = ()
    #: Imports the classes need (statements of the program, so that they
    #: import pltrs' modules under pltrs).
    imports: tuple[str, ...] = ()


#: The figure every scene draws on.
FIGURE = "fig, ax = plt.subplots(figsize=(3, 2))"


def _artist(make: str, add: str, prop: str = "obj.set_alpha(0.5)", art: str = "obj", **more) -> Scene:
    return Scene("", (FIGURE, f"obj = {make}", add, f"art = {art}"), prop, **more)


def _value(make: str, use: tuple[str, ...], art: str, prop: str, **more) -> Scene:
    return Scene("", (FIGURE, f"obj = {make}", *use, f"art = {art}"), prop, kind="value", **more)


#: The steps of an Axes' life: plotting, limits, coordinates, a twin.
AXES_STEPS = ("ax.scatter(x, y)", "ax.set_xlim(ax.get_xlim()[0], ax.get_xlim()[1] + 1)", "ax.format_coord(0.5, 0.5)",
              "ax.twinx()")


def _axes(data: str = "ax.plot(x, y)", steps: tuple[str, ...] = AXES_STEPS) -> Scene:
    return Scene("", ("fig = plt.figure(figsize=(3, 2))", "obj = ax = art = fig.add_subplot(axes_class=Sub)",
                      data), "ax.set_title('title')", kind="axes", steps=steps)


TRANSFORM_IMPL = '''
class Impl(transforms.Transform):
    input_dims = output_dims = 2
    is_separable = False
    has_inverse = True
    def __init__(self, shift=0.1):
        super().__init__()
        self.shift = shift
    def transform_non_affine(self, values):
        values = np.array(values, dtype=float)
        values[:, 1] += self.shift
        return values
    def inverted(self):
        return Impl(-self.shift)
'''

SCALE_IMPL = '''
class Impl(scale.ScaleBase):
    name = 'overridden'
    def get_transform(self):
        return transforms.IdentityTransform()
    def set_default_locators_and_formatters(self, axis):
        axis.set_major_locator(ticker.AutoLocator())
        axis.set_major_formatter(ticker.ScalarFormatter())
        axis.set_minor_locator(ticker.NullLocator())
        axis.set_minor_formatter(ticker.NullFormatter())
'''

LOCATOR_IMPL = '''
class Impl(ticker.Locator):
    def __call__(self):
        return self.tick_values(*self.axis.get_view_interval())
    def tick_values(self, vmin, vmax):
        return np.linspace(vmin, vmax, 4)
'''

FORMATTER_IMPL = '''
class Impl(ticker.Formatter):
    def __call__(self, x, pos=None):
        return f'{x:.2f}'
'''

HANDLER_IMPL = '''
class Impl(legend_handler.HandlerBase):
    def create_artists(self, legend, orig_handle, xdescent, ydescent, width, height, fontsize, trans):
        return [patches.Rectangle((-xdescent, -ydescent), width, height, transform=trans)]
'''

CONVERTER_IMPL = '''
class Impl(units.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        return np.asarray(value, dtype=float) if np.iterable(value) else float(value)
    @staticmethod
    def axisinfo(unit, axis):
        return units.AxisInfo(label='decimals')
    @staticmethod
    def default_units(x, axis):
        return 'decimal'
'''

LAYOUT_IMPL = '''
class Impl(layout_engine.LayoutEngine):
    _adjust_compatible = True
    _colorbar_gridspec = True
    def execute(self, fig):
        fig.subplots_adjust(left=0.2, bottom=0.2)
'''

SPINE_HOST = '''
class Host(matplotlib.axes.Axes):
    def _gen_axes_spines(self, locations=None, offset=0.0, units='inches'):
        return {side: Sub.linear_spine(self, side) for side in ('left', 'right', 'bottom', 'top')}
'''

AXIS_HOST = '''
class Host(matplotlib.axes.Axes):
    def _init_axis(self):
        self.xaxis = Sub(self, clear=False)
        self.spines.bottom.register_axis(self.xaxis)
        self.spines.top.register_axis(self.xaxis)
        self.yaxis = axis.YAxis(self, clear=False)
        self.spines.left.register_axis(self.yaxis)
        self.spines.right.register_axis(self.yaxis)
'''

TICK_HOST = '''
class TickAxis(axis.XAxis):
    def _get_tick(self, major):
        return Sub(self.axes, 0, major=major)
class Host(matplotlib.axes.Axes):
    def _init_axis(self):
        super()._init_axis()
        self.xaxis = TickAxis(self, clear=False)
        self.spines.bottom.register_axis(self.xaxis)
        self.spines.top.register_axis(self.xaxis)
'''

HOSTED = ("fig = plt.figure(figsize=(3, 2))", "ax = fig.add_subplot(axes_class=Host)", "ax.plot(x, y)")

#: Imports the scenes need beyond the generated programs' header.
IMPORTS = ("from matplotlib import artist, axes, axis, colorbar, contour, dates, figure, gridspec, image, "
           "layout_engine, legend, legend_handler, offsetbox, patheffects, projections, spines, units")

DATES = "days = np.arange('2020-01-01', '2020-01-08', dtype='datetime64[D]')"

#: matplotlib's class (as the census names it) -> its scene.
SCENES: dict[str, Scene] = {
    "matplotlib.artist.Artist": _artist("Sub()", "ax.add_artist(obj)", "obj.set_zorder(5)"),
    "matplotlib.lines.Line2D": _artist("Sub(x, y)", "ax.add_line(obj)", "obj.set_linewidth(3)"),
    "matplotlib.text.Text": _artist("Sub(0.5, 0.5, 'text')", "ax.add_artist(obj)", "obj.set_fontsize(15)"),
    "matplotlib.patches.PathPatch": _artist("Sub(path.Path([(0, 0), (1, 0), (1, 1), (0, 0)]))", "ax.add_patch(obj)"),
    "matplotlib.patches.Polygon": _artist("Sub([(0, 0), (1, 0), (1, 1)])", "ax.add_patch(obj)"),
    "matplotlib.patches.Rectangle": _artist("Sub((0.2, 0.2), 0.5, 0.3)", "ax.add_patch(obj)"),
    "matplotlib.patches.Circle": _artist("Sub((0.5, 0.5), 0.2)", "ax.add_patch(obj)"),
    "matplotlib.patches.FancyBboxPatch": _artist("Sub((0.2, 0.2), 0.5, 0.3, boxstyle='round,pad=0.1')",
                                                 "ax.add_patch(obj)"),
    "matplotlib.patches.FancyArrowPatch": _artist("Sub((0.1, 0.1), (0.8, 0.8), arrowstyle='->', mutation_scale=20)",
                                                  "ax.add_patch(obj)"),
    "matplotlib.collections.Collection": _artist("Sub(facecolors='C0')",
                                                 "obj.set_paths([path.Path.unit_rectangle()]); "
                                                 "ax.add_collection(obj)"),
    "matplotlib.collections.PathCollection": _artist("Sub([path.Path.unit_circle()], sizes=[40], offsets=list(zip(x, y)), "
                                                     "offset_transform=ax.transData)", "ax.add_collection(obj)"),
    "matplotlib.collections.LineCollection": _artist("Sub([list(zip(x, y)), list(zip(x, -y))])",
                                                     "ax.add_collection(obj); ax.autoscale_view()"),
    "matplotlib.collections.QuadMesh": _artist("Sub(np.stack(np.meshgrid(np.arange(7), np.arange(6)), -1))",
                                               "obj.set_array(z); ax.add_collection(obj); ax.autoscale_view()"),
    "matplotlib.image.AxesImage": _artist("Sub(ax)", "obj.set_data(z); obj.set_extent(obj.get_extent()); ax.add_image(obj); ax.set_xlim(-0.5, 5.5); "
                                          "ax.set_ylim(4.5, -0.5)", "obj.set_cmap('magma')"),
    "matplotlib.legend.Legend": _artist("Sub(ax, ax.plot(x, y), ['line'])", "ax.add_artist(obj)",
                                        "obj.set_title('title')"),
    "matplotlib.offsetbox.AnchoredOffsetbox": _artist("Sub(loc='upper left', child=offsetbox.TextArea('box'))",
                                                      "ax.add_artist(obj)"),
    "matplotlib.offsetbox.DrawingArea": _artist("Sub(40, 20)", "obj.add_artist(patches.Circle((10, 10), 5)); "
                                                "ax.add_artist(offsetbox.AnnotationBbox(obj, (0.5, 0.5)))",
                                                art="ax.artists[0]"),
    "matplotlib.offsetbox.AuxTransformBox": _artist("Sub(ax.transData)",
                                                    "obj.add_artist(patches.Rectangle((0, 0), 1, 0.5)); "
                                                    "ax.add_artist(offsetbox.AnchoredOffsetbox('upper right', "
                                                    "child=obj))", art="ax.artists[0]"),
    "matplotlib.spines.Spine": Scene("", (*HOSTED, "obj = art = ax.spines['bottom']"), "obj.set_linewidth(3)",
                                     kind="axes", hosts=SPINE_HOST),
    "matplotlib.axis.XAxis": Scene("", (*HOSTED, "obj = art = ax.xaxis"), "obj.set_label_text('x')",
                                   kind="axes", hosts=AXIS_HOST),
    "matplotlib.axis.XTick": Scene("", (*HOSTED, "obj = art = ax.xaxis.get_major_ticks()[0]"),
                                   "ax.tick_params(labelsize=14)", kind="axes", hosts=TICK_HOST),
    "matplotlib.figure.Figure": Scene("", ("fig = plt.figure(FigureClass=Sub, figsize=(3, 2))",
                                           "obj = art = fig", "ax = fig.add_subplot()", "ax.plot(x, y)"),
                                      "fig.suptitle('title')", kind="figure"),
    "matplotlib.axes.Axes": _axes(),
    # Polar and geographic Axes have no twins, geographic ones no limits.
    "matplotlib.projections.PolarAxes": _axes(steps=AXES_STEPS[:3]),
    "matplotlib.projections.LambertAxes": _axes("ax.plot(x / 4 - 0.8, y / 2)", AXES_STEPS[:1] + AXES_STEPS[2:3]),
    "matplotlib.transforms.Transform": _value("Sub()", ("ax.plot(x, y, transform=obj + ax.transData)",),
                                              "ax.lines[0]", "art.set_linewidth(3)", impl=TRANSFORM_IMPL),
    # A line through the transform composed with the data's, and a text
    # placed by the transform alone (matplotlib maps its point through the
    # class' ``transform``).
    "matplotlib.transforms.Affine2DBase": _value("Sub()", ("ax.plot(x, y, transform=obj + ax.transData)",
                                                           "ax.text(150, 120, 'placed', transform=obj)"),
                                                 "ax.lines[0]", "art.set_linewidth(3)",
                                                 impl="class Impl(transforms.Affine2DBase):\n"
                                                      "    def get_matrix(self):\n"
                                                      "        return np.array([[1, 0, 0], [0, 0.5, 0], [0, 0, 1.0]])\n"),
    "matplotlib.transforms.Affine2D": _value("Sub().scale(1, 0.5)", ("ax.plot(x, y, transform=obj + ax.transData)",
                                                                     "ax.text(150, 120, 'placed', transform=obj)"),
                                             "ax.lines[0]", "art.set_linewidth(3)"),
    "matplotlib.ticker.Locator": _value("Sub()", ("ax.plot(x, y)", "ax.xaxis.set_major_locator(obj)"), "ax.xaxis",
                                        "ax.set_xlim(0, 4)", impl=LOCATOR_IMPL),
    "matplotlib.ticker.MaxNLocator": _value("Sub(4)", ("ax.plot(x, y)", "ax.xaxis.set_major_locator(obj)"),
                                            "ax.xaxis", "ax.set_xlim(0, 4)"),
    "matplotlib.ticker.AutoMinorLocator": _value("Sub()", ("ax.plot(x, y)", "ax.xaxis.set_minor_locator(obj)"),
                                                 "ax.xaxis", "ax.set_xlim(0, 4)"),
    "matplotlib.dates.DateLocator": _value("Sub()", (DATES, "ax.plot(days, values)",
                                                     "ax.xaxis.set_major_locator(obj)"), "ax.xaxis",
                                           "ax.set_xlim(days[1], days[5])", impl="Impl = dates.DayLocator"),
    "matplotlib.dates.AutoDateLocator": _value("Sub()", (DATES, "ax.plot(days, values)",
                                                         "ax.xaxis.set_major_locator(obj)"), "ax.xaxis",
                                               "ax.set_xlim(days[1], days[5])"),
    "matplotlib.ticker.Formatter": _value("Sub()", ("ax.plot(x, y)", "ax.xaxis.set_major_formatter(obj)"),
                                          "ax.xaxis", "ax.set_xlim(0, 4)", impl=FORMATTER_IMPL),
    "matplotlib.ticker.ScalarFormatter": _value("Sub()", ("ax.plot(x, y * 1e5)", "ax.yaxis.set_major_formatter(obj)"),
                                                "ax.yaxis", "ax.set_ylim(0, 2e5)"),
    "matplotlib.colors.Normalize": _value("Sub()", ("ax.imshow(z, norm=obj)", "fig.colorbar(ax.images[0])"),
                                          "ax.images[0]", "obj.vmax = 3"),
    "matplotlib.colors.BoundaryNorm": _value("Sub([-1, 0, 1, 2], 256)", ("ax.imshow(z, norm=obj)",
                                                                         "fig.colorbar(ax.images[0])"),
                                             "ax.images[0]", "art.set_cmap('magma')"),
    "matplotlib.colors.Colormap": _value("Sub(['r', 'g', 'b'])", ("ax.imshow(z, cmap=obj)",
                                                                  "fig.colorbar(ax.images[0])"),
                                         "ax.images[0]", "art.set_clim(0, 1)", impl="Impl = colors.ListedColormap"),
    "matplotlib.colors.ListedColormap": _value("Sub(['r', 'g', 'b'])", ("ax.imshow(z, cmap=obj)",
                                                                        "fig.colorbar(ax.images[0])"),
                                               "ax.images[0]", "art.set_clim(0, 1)"),
    "matplotlib.colors.LinearSegmentedColormap": _value("Sub.from_list('mine', ['k', 'r', 'w'])",
                                                        ("ax.imshow(z, cmap=obj)", "fig.colorbar(ax.images[0])"),
                                                        "ax.images[0]", "art.set_clim(0, 1)"),
    "matplotlib.scale.ScaleBase": _value("Sub", ("scale.register_scale(obj)", "ax.plot(x, y)",
                                                 "ax.set_xscale('overridden')"), "ax.xaxis", "ax.set_xlim(1, 5)",
                                         impl=SCALE_IMPL),
    "matplotlib.scale.LinearScale": _value("Sub", ("scale.register_scale(obj)", "ax.plot(x, y)",
                                                   "ax.set_xscale(obj.name)"), "ax.xaxis", "ax.set_xlim(1, 5)"),
    "matplotlib.scale.LogScale": _value("Sub", ("scale.register_scale(obj)", "ax.plot(x + 1, values)",
                                                "ax.set_yscale(obj.name)"), "ax.yaxis", "ax.set_ylim(0.5, 20)"),
    "matplotlib.scale.SymmetricalLogScale": _value("Sub", ("scale.register_scale(obj)", "ax.plot(x, y * 100)",
                                                           "ax.set_yscale(obj.name)"), "ax.yaxis",
                                                   "ax.set_ylim(-50, 200)"),
    "matplotlib.scale.LogitScale": _value("Sub", ("scale.register_scale(obj)", "ax.plot(x, (y + 1.5) / 3)",
                                                  "ax.set_yscale(obj.name)"), "ax.yaxis", "ax.set_ylim(0.1, 0.9)"),
    "matplotlib.units.ConversionInterface": _value("Sub()", ("import decimal", "units.registry[decimal.Decimal] = obj",
                                                             "ax.plot([decimal.Decimal('1.5'), decimal.Decimal('2.5'), "
                                                             "decimal.Decimal('4')], [1, 3, 2])"),
                                                   "ax.xaxis", "ax.set_xlim(1, 5)", impl=CONVERTER_IMPL),
    "matplotlib.dates.DateConverter": _value("Sub()", (DATES, "units.registry[np.datetime64] = obj",
                                                       "ax.plot(days, values)"), "ax.xaxis",
                                             "ax.set_xlim(days[1], days[5])"),
    "matplotlib.gridspec.GridSpec": Scene("", ("fig = plt.figure(figsize=(3, 2))", "obj = Sub(2, 2, figure=fig)",
                                               "ax = art = fig.add_subplot(obj[0, 0])", "fig.add_subplot(obj[1, :])",
                                               "ax.plot(x, y)"), "obj.update(wspace=0.5)", kind="value"),
    "matplotlib.gridspec.GridSpecBase": Scene("", ("fig = plt.figure(figsize=(3, 2))", "obj = Sub(2, 2, figure=fig)",
                                                   "ax = art = fig.add_subplot(obj[0, 0])",
                                                   "fig.add_subplot(obj[1, :])", "ax.plot(x, y)"),
                                              "obj.set_height_ratios([2, 1])", kind="value",
                                              impl="Impl = gridspec.GridSpec"),
    "matplotlib.gridspec.SubplotSpec": Scene("", ("fig = plt.figure(figsize=(3, 2))",
                                                  "obj = Sub(gridspec.GridSpec(2, 2, figure=fig), 0)",
                                                  "ax = art = fig.add_subplot(obj)", "ax.plot(x, y)"),
                                             "fig.subplots_adjust(left=0.3)", kind="value"),
    "matplotlib.layout_engine.LayoutEngine": _value("Sub()", ("fig.set_layout_engine(obj)", "ax.plot(x, y)"), "ax",
                                                    "ax.set_ylabel('y')", impl=LAYOUT_IMPL),
    "matplotlib.legend_handler.HandlerBase": _value("Sub()", ("ax.plot(x, y, label='line')",
                                                              "ax.legend(handler_map={lines.Line2D: obj})"),
                                                    "ax.get_legend()", "art.set_title('title')", impl=HANDLER_IMPL),
    "matplotlib.legend_handler.HandlerLine2D": _value("Sub()", ("ax.plot(x, y, label='line')",
                                                                "ax.legend(handler_map={lines.Line2D: obj})"),
                                                      "ax.get_legend()", "art.set_title('title')"),
    "matplotlib.legend_handler.HandlerPatch": _value("Sub()", ("ax.bar([1, 2], [3, 4], label='bars')",
                                                               "ax.legend(handler_map={patches.Rectangle: obj})"),
                                                     "ax.get_legend()", "art.set_title('title')"),
    "matplotlib.legend_handler.HandlerLineCollection": _value(
        "Sub()", ("ax.add_collection(collections.LineCollection([list(zip(x, y))], label='lines'))",
                  "ax.autoscale_view()", "ax.legend(handler_map={collections.LineCollection: obj})"),
        "ax.get_legend()", "art.set_title('title')"),
    "matplotlib.patheffects.AbstractPathEffect": _value("Sub()", ("ax.plot(x, y, path_effects=[obj])",),
                                                        "ax.lines[0]", "art.set_linewidth(3)"),
    "matplotlib.colorbar.Colorbar": Scene("", (FIGURE, "ax.imshow(z)", "fig.subplots_adjust(right=0.8)",
                                               "obj = Sub(fig.add_axes((0.85, 0.1, 0.04, 0.8)), ax.images[0])",
                                               "art = obj.ax"), "obj.set_label('label')", kind="value"),
    "matplotlib.contour.QuadContourSet": _artist("Sub(ax, np.arange(6), np.arange(5), z, levels=4)",
                                                 "ax.clabel(obj)", "obj.set_linewidth(2)"),
    "matplotlib.dates.AutoDateFormatter": _value("Sub(dates.AutoDateLocator())",
                                                 (DATES, "ax.plot(days, values)",
                                                  "ax.xaxis.set_major_formatter(obj)"), "ax.xaxis",
                                                 "ax.set_xlim(days[1], days[5])"),
    "matplotlib.ticker.FixedFormatter": _value("Sub(['a', 'b', 'c'])",
                                               ("ax.plot(x, y)", "ax.xaxis.set_major_locator(ticker.FixedLocator([1, 2, 3]))",
                                                "ax.xaxis.set_major_formatter(obj)"), "ax.xaxis",
                                               "ax.set_xlim(0, 4)"),
    "matplotlib.legend_handler.HandlerPathCollection": _value(
        "Sub()", ("ax.scatter(x, y, label='points')",
                  "ax.legend(handler_map={collections.PathCollection: obj})"),
        "ax.get_legend()", "art.set_title('title')"),
    "mpl_toolkits.axes_grid1.axes_size.Scaled": _value(
        "Sub(0.5)", ("make_axes_locatable(ax).append_axes('right', size=obj, pad=0.1)", "ax.plot(x, y)"),
        "ax", "ax.set_xlim(0, 4)", impl="Impl = axes_size.Scaled\n",
        imports=("from mpl_toolkits.axes_grid1 import axes_size, make_axes_locatable",)),
}

#: The Axes plotting methods and the pyplot function calling each, where
#: its name differs.
PYPLOT_NAMES = {"set_xlabel": "xlabel", "set_ylabel": "ylabel", "set_xscale": "xscale", "set_yscale": "yscale",
                "set_xticks": "xticks", "set_yticks": "yticks", "set_title": "title", "set_xlim": "xlim",
                "set_ylim": "ylim"}


#: The plotting methods matplotlib adds what they make with each ``add_*``
#: method of the Axes.
ADDING = {
    "add_line": ("ax.plot(x, -y)", "ax.axhline(0.5)", "ax.errorbar(x, y, yerr=0.2)"),
    "add_patch": ("ax.bar([1, 2], [1, 2])", "ax.fill([0, 1, 1], [0, 0, 1])"),
    "add_collection": ("ax.fill_between(x, y)", "ax.vlines([1, 2], 0, 1)"),
    "add_image": ("ax.imshow(z)",),
    "_add_text": ("ax.text(1, 0, 'text')", "ax.annotate('note', (1, 0))"),
    "add_table": ("ax.table([['a', 'b']])",),
    "add_container": ("ax.bar([1, 2], [1, 2])", "ax.errorbar(x, y, yerr=0.2)"),
}


def member_steps(cls: str, member: str) -> tuple[str, ...]:
    """Steps reaching ``member`` of an Axes scene the way programs reach
    it: through pyplot, which calls it on the current Axes; for an
    ``add_*`` method, the plotting methods adding with it."""
    if SCENES[cls].kind == "axes" and member in ADDING:
        return ADDING[member]
    if not SCENES[cls].kind == "axes" or member not in plotting.VARIANTS:
        return ()
    import matplotlib.pyplot as plt

    name = PYPLOT_NAMES.get(member, member)
    arguments = plotting.VARIANTS[member][0]
    call = f"plt.{name}({arguments})" if hasattr(plt, name) else f"ax.{member}({arguments})"
    return ("plt.sca(ax)", call)
