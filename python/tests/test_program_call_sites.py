"""
Smoke tests of the program's code pltrs (Rust) calls where matplotlib
calls it, as often and with the arrays matplotlib hands over: the norm of
a colorbar (its `_process_values`, `_mesh`, `_do_extends`, axes locator
and long-axis scale, and the meshes normalised at draw), the transforms of
a projection of the program's (tick labels measured and drawn, gridlines
mapped as a path, then as points); and the program's code that reads and
changes the figure while pltrs lays it out or draws it: a projection's
data ratio, a patch's transform and box style, the transforms of a
curvilinear grid, a text's transform measured for a tight layout and a
function scale. The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import axis as maxis, colors as mcolors, patches as mpatches
from pltrs import spines as mspines, ticker as mticker, transforms as mtransforms
from pltrs.axes import Axes
from pltrs.mpl_toolkits.axisartist import Axes as AAxes
from pltrs.mpl_toolkits.axisartist.grid_helper_curvelinear import GridHelperCurveLinear
from pltrs.projections import register_projection

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def array(value):
    """What the program is handed: the kind of array and its shape."""
    kind = "masked" if isinstance(value, np.ma.MaskedArray) else type(value).__name__
    return kind, np.shape(value)


def ticked(row, column, times, row_times):
    """An axis' tick update as matplotlib makes it through a scale: the
    view as a column, then each of `times` ticks; then `row_times` maps of
    one tick label or mark, as rows."""
    return [column(2)] + [column(1)] * times + [row(1)] * row_times


def test_a_colorbar_calls_the_program_s_norm_where_matplotlib_s_does():
    seen = []

    class Midpoint(mcolors.Normalize):
        def __call__(self, value, clip=None):
            seen.append(("call", array(value)))
            return np.ma.masked_array(np.interp(value, [self.vmin, 0, self.vmax], [0, 0.5, 1]))

        def inverse(self, value):
            seen.append(("inverse", array(value)))
            return np.interp(value, [0, 0.5, 1], [self.vmin, 0, self.vmax])

    fig, ax = plt.subplots()
    z = np.linspace(-1, 2, 12).reshape(3, 4)
    mesh = ax.pcolormesh(z, norm=Midpoint(vmin=-1, vmax=2), cmap="RdBu_r")
    fig.colorbar(mesh, extend="both", ticks=[-1, 0, 2])
    seen.append("draw")
    fig.canvas.draw()

    def call(kind, *shape):
        return ("call", (kind, shape))

    def row(n):
        return call("ndarray", n)

    def column(n):
        return call("ndarray", n, 1)

    # `_process_values` twice, `_mesh`, `_do_extends` (the extensions'
    # values alone); then the locator in `_get_draw_artists`, the mesh, the
    # locator in `Axes.draw`, the solids, their coordinates, the view and
    # the parts of the long axis.
    assert seen == [
        ("inverse", ("ndarray", (259,))), ("inverse", ("ndarray", (259,))), row(259),
        ("inverse", ("ndarray", (257,))), row(259), call("float64"), call("float64"),
        "draw", row(259), call("masked", 3, 4), row(259), call("masked", 256, 1),
        call("masked", 514), row(4), row(2),
        *ticked(row, column, 3, 12), *ticked(row, column, 3, 3), row(2),
        *ticked(row, column, 3, 0)]


def test_a_colorbar_of_a_function_norm_maps_its_long_axis_through_the_functions():
    seen = []

    def forward(x):
        seen.append(array(x))
        return np.sqrt(x)

    fig, ax = plt.subplots()
    norm = mcolors.FuncNorm((forward, lambda x: x ** 2), vmin=0, vmax=4)
    mesh = ax.pcolormesh(np.linspace(0, 4, 6).reshape(2, 3), norm=norm)
    fig.colorbar(mesh, ticks=[0, 1, 4])
    seen.append("draw")
    fig.canvas.draw()

    def row(n):
        return ("ndarray", (n,))

    def column(n):
        return ("ndarray", (n, 1))

    assert seen == [
        ("masked", (6, 1)), ("masked", (6, 1)), column(2), ("masked", (6, 1)), column(2),
        ("masked", (257, 1)), column(2), column(2), ("masked", (257, 1)), column(2),
        ("masked", (256, 1)), "draw", ("masked", (6, 1)), column(2), ("masked", (256, 1)),
        column(2), ("masked", (514,)), row(4), row(2),
        *ticked(row, column, 3, 12), *ticked(row, column, 3, 3), row(2),
        *ticked(row, column, 3, 0)]


class Squash(mtransforms.Transform):
    """Squashes the plane of a projection, recording what it maps."""
    input_dims = output_dims = 2
    seen = []

    def transform_non_affine(self, values):
        values = np.asarray(values, dtype=float)
        Squash.seen.append(values.shape)
        x, y = values[:, 0], values[:, 1]
        return np.column_stack([x * np.cos(y / 2), y])

    def transform_path_non_affine(self, path):
        Squash.seen.append("path")
        ipath = path.interpolated(4)
        return path.__class__(self.transform(ipath.vertices), ipath.codes)

    def inverted(self):
        return mtransforms.Affine2D()


class Squashed(Axes):
    """Axes squashing the plane, as the gallery's Hammer axes do: their
    gridlines and tick labels go through the projection."""
    name = "squash_probe"

    class Labels(mticker.Formatter):
        def __call__(self, x, pos=None):
            return f"{x:.1f}"

    def _init_axis(self):
        self.xaxis = maxis.XAxis(self)
        self.yaxis = maxis.YAxis(self)

    def _gen_axes_patch(self):
        return mpatches.Circle((0.5, 0.5), 0.5)

    def _gen_axes_spines(self):
        return {"geo": mspines.Spine.circular_spine(self, (0.5, 0.5), 0.5)}

    def clear(self):
        super().clear()
        self.xaxis.set_major_formatter(self.Labels())
        self.yaxis.set_major_formatter(self.Labels())
        self.xaxis.set_ticks_position("none")
        self.yaxis.set_ticks_position("none")
        self.grid(True)
        Axes.set_xlim(self, -1, 1)
        Axes.set_ylim(self, -1, 1)

    def _set_lim_and_transforms(self):
        self.transProjection = Squash()
        self.transAffine = mtransforms.Affine2D().scale(0.5).translate(0.5, 0.5)
        self.transAxes = mtransforms.BboxTransformTo(self.bbox)
        self.transData = self.transProjection + self.transAffine + self.transAxes
        self._xaxis_transform = (mtransforms.Affine2D().scale(1, 2).translate(0, -1)
                                 + self.transData)
        self._xaxis_text1_transform = (mtransforms.Affine2D().scale(1, 0) + self.transData
                                       + mtransforms.Affine2D().translate(0, -4))
        self._xaxis_text2_transform = (mtransforms.Affine2D().scale(1, 0) + self.transData
                                       + mtransforms.Affine2D().translate(0, 4))
        stretch = mtransforms.Affine2D().scale(2, 1).translate(-1, 0)
        self._yaxis_transform = stretch + self.transData
        base = stretch + self.transProjection + (self.transAffine + self.transAxes)
        self._yaxis_text1_transform = base + mtransforms.Affine2D().translate(-8, 0)
        self._yaxis_text2_transform = base + mtransforms.Affine2D().translate(8, 0)

    def get_xaxis_transform(self, which="grid"):
        return self._xaxis_transform

    def get_xaxis_text1_transform(self, pad):
        return self._xaxis_text1_transform, "top", "center"

    def get_xaxis_text2_transform(self, pad):
        return self._xaxis_text2_transform, "bottom", "center"

    def get_yaxis_transform(self, which="grid"):
        return self._yaxis_transform

    def get_yaxis_text1_transform(self, pad):
        return self._yaxis_text1_transform, "center", "right"

    def get_yaxis_text2_transform(self, pad):
        return self._yaxis_text2_transform, "center", "left"


register_projection(Squashed)


def test_a_projection_s_transforms_are_called_at_matplotlib_s_sites():
    fig = plt.figure()
    ax = fig.add_subplot(projection="squash_probe")
    ax.set_xticks([-0.5, 0.5])
    ax.set_yticks([0.0])
    Squash.seen = ["draw"]
    fig.canvas.draw()
    point, points, path = (1, 2), (2, 2), "path"
    gridline = [path, (5, 2), points]
    mark = [path, point, point]
    # Each axis: its labels measured; each tick's gridline (a path, then
    # its points), mark and label drawn; its labels measured again.
    x_axis = [point, point, *gridline, *mark, point, *gridline, *mark, point, point, point]
    y_axis = [point, *gridline, *mark, point, point]
    assert Squash.seen == ["draw", *x_axis, *y_axis]


def test_a_projection_s_data_ratio_reads_and_changes_the_figure():
    seen = []

    class Ratio(Axes):
        name = "ratio_probe_sites"

        def get_data_ratio(self):
            seen.append((tuple(map(float, self.get_xlim())), self.get_title()))
            self.set_title("ratio %d" % len(seen))
            return 2.0

    register_projection(Ratio)
    fig = plt.figure(figsize=(4, 4))
    ax = fig.add_subplot(projection="ratio_probe_sites")
    ax.set_aspect("equal")
    fig.canvas.draw()
    # The draw asks for it twice, `get_position` applying the aspect once.
    assert [round(v, 4) for v in ax.get_position().bounds] == [0.32, 0.11, 0.385, 0.77]
    assert seen == [((0.0, 1.0), ""), ((0.0, 1.0), "ratio 1"), ((0.0, 1.0), "ratio 2")]
    assert ax.get_title() == "ratio 3"


def test_a_patch_s_transform_and_box_style_read_and_change_the_figure():
    seen = []
    fig, ax = plt.subplots()

    class Shift(mtransforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            seen.append(("shift", np.shape(values), ax.get_title()))
            ax.set_title("shift %d" % len(seen))
            return np.asarray(values) + 0.1

    ax.add_patch(mpatches.Rectangle((0.2, 0.2), 0.3, 0.3, transform=Shift() + ax.transAxes))

    def style(x0, y0, width, height, mutation_size):
        seen.append(("box", ax.get_xlabel()))
        ax.set_xlabel("box %d" % len(seen))
        return mpatches.Path([(x0, y0), (x0 + width, y0), (x0 + width, y0 + height),
                              (x0, y0)], closed=True)

    # The box style makes the path for the data limits, then each draws.
    ax.add_patch(mpatches.FancyBboxPatch((0.6, 0.6), 0.2, 0.2, boxstyle=style,
                                         transform=ax.transAxes))
    fig.canvas.draw()
    assert seen == [("box", ""), ("shift", (5, 2), ""), ("box", "box 1")]
    assert (ax.get_title(), ax.get_xlabel()) == ("shift 2", "box 3")


def test_the_transforms_of_a_curvilinear_grid_read_and_change_the_figure():
    seen = []
    fig = plt.figure()

    def tr(x, y):
        seen.append(("tr", np.shape(x), len(fig.texts)))
        if not fig.texts:
            fig.text(0, 0, "made")
        return np.asarray(x), np.asarray(y) + np.asarray(x) * 0.1

    def inv(x, y):
        seen.append(("inv", np.shape(x), len(fig.texts)))
        return np.asarray(x), np.asarray(y) - np.asarray(x) * 0.1

    ax = fig.add_subplot(axes_class=AAxes, grid_helper=GridHelperCurveLinear((tr, inv)))
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    # matplotlib's `AxisArtist._init_line` computes the grid and the line.
    ax.new_floating_axis(0, 3)
    assert seen == ([("inv", (400,), 0), ("tr", (100,), 0)] + [("tr", (100,), 1)] * 19
                    + [("inv", (400,), 1), ("tr", (100,), 1)])
    fig.canvas.draw()
    assert [t.get_text() for t in fig.texts] == ["made"]


def test_a_text_s_transform_measured_for_a_tight_layout_changes_the_figure():
    seen = []
    fig, ax = plt.subplots()

    class Shift(mtransforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            seen.append((np.shape(values), ax.get_title()))
            ax.set_title("shift %d" % len(seen))
            return np.asarray(values) + 0.1

    ax.text(0.2, 0.2, "hello", transform=Shift() + ax.transAxes)
    fig.tight_layout()
    assert seen == [((1, 2), "")]
    assert ax.get_title() == "shift 1"


def test_a_function_scale_reads_and_changes_the_figure():
    seen = []
    fig, ax = plt.subplots()

    def forward(x):
        seen.append((np.shape(x), ax.get_title()))
        ax.set_title("f%d" % len(seen))
        return x

    ax.plot([1, 2], [1, 4])
    ax.set_yscale("function", functions=(forward, lambda x: x))
    fig.canvas.draw()
    assert seen[:6] == [((2, 1), ""), ((2, 1), "f1"), ((1, 1), "f2"), ((1, 1), "f3"),
                        ((1, 1), "f4"), ((1, 1), "f5")]
    assert (len(seen), ax.get_title()) == (150, "f150")
