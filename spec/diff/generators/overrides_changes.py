"""The second variant of an ``overrides`` program: the override changes the
result, within what the member's contract allows, and the program reads
what the figure then shows.

``CHANGES`` maps (matplotlib's class, member) to the override's source,
a method of ``Sub`` that records its call through ``call`` (as the first
variant's does) and returns another result: another transform, path,
extent, tick list, colour, or nothing drawn. ``ATTRIBUTES`` gives class
attributes their changed values.
"""

from __future__ import annotations

#: What every changed program reads once the figure is drawn.
SHOWS = (
    "(ax.get_xlim(), ax.get_ylim())",
    "[t.get_text() for t in ax.get_xticklabels()]",
    "[t.get_text() for t in ax.get_yticklabels()]",
    "ax.get_position().bounds",
    "art.get_window_extent(fig.canvas.get_renderer()).bounds",
    "fig.get_tightbbox(fig.canvas.get_renderer()).bounds",
)

#: An override drawing nothing.
_DRAWS_NOTHING = "def draw(self, renderer):\n    call('draw', lambda renderer: None, (renderer,))\n"
#: An override measuring a box of its own.
_EXTENT = ("def get_window_extent(self, renderer=None):\n"
           "    call('get_window_extent', super().get_window_extent, (renderer,))\n"
           "    return transforms.Bbox([[10, 10], [60, 40]])\n")
_TIGHT = ("def get_tightbbox(self, renderer=None, *args, **kwargs):\n"
          "    call('get_tightbbox', super().get_tightbbox, (renderer, *args), kwargs)\n"
          "    return transforms.Bbox([[-30, -20], [320, 230]])\n")
_CONTAINS = ("def contains(self, mouseevent):\n"
             "    call('contains', super().contains, (mouseevent,))\n"
             "    return True, {'changed': True}\n")
_TICKS = ("def __call__(self):\n"
          "    return list(call('__call__', super().__call__))[::2]\n")
_TICK_VALUES = ("def tick_values(self, vmin, vmax):\n"
                "    return list(call('tick_values', super().tick_values, (vmin, vmax)))[::2]\n")
_FORMAT = ("def __call__(self, x, pos=None):\n"
           "    return '<' + call('__call__', super().__call__, (x, pos)) + '>'\n")
_TRANSFORM = ("def transform_non_affine(self, values):\n"
              "    return np.asarray(call('transform_non_affine', super().transform_non_affine, (values,))) * [1, 0.5]\n")
_MATRIX = ("def get_matrix(self):\n"
           "    matrix = np.array(call('get_matrix', super().get_matrix), dtype=float)\n"
           "    matrix[0, 0] *= 2\n"
           "    return matrix\n")
_NORM = ("def __call__(self, value, clip=None):\n"
         "    return call('__call__', super().__call__, (value, clip)) ** 2\n")
_CMAP = ("def __call__(self, X, alpha=None, bytes=False):\n"
         "    return call('__call__', super().__call__, (X[..., ::-1] if np.ndim(X) else X, alpha, bytes))\n")
_TRANSFORM_GETTER = ("def get_transform(self):\n"
                     "    return transforms.Affine2D().translate(10, 5) + call('get_transform', super().get_transform)\n")
_PATHS = ("def get_paths(self):\n"
          "    call('get_paths', super().get_paths)\n"
          "    return [path.Path.unit_circle()]\n")

#: (matplotlib's class, member) -> the changing override.
CHANGES: dict[tuple[str, str], str] = {
    **{(cls, "draw"): _DRAWS_NOTHING for cls in (
        "matplotlib.artist.Artist", "matplotlib.axes.Axes", "matplotlib.text.Text", "matplotlib.lines.Line2D",
        "matplotlib.collections.Collection", "matplotlib.spines.Spine", "matplotlib.image.AxesImage",
        "matplotlib.patches.PathPatch", "matplotlib.patches.FancyArrowPatch", "matplotlib.patches.FancyBboxPatch",
        "matplotlib.patches.Rectangle", "matplotlib.axis.XTick", "matplotlib.offsetbox.DrawingArea",
        "matplotlib.offsetbox.AuxTransformBox", "matplotlib.figure.Figure")},
    **{(cls, "get_window_extent"): _EXTENT for cls in (
        "matplotlib.artist.Artist", "matplotlib.spines.Spine", "matplotlib.image.AxesImage",
        "matplotlib.patches.FancyBboxPatch")},
    **{(cls, "get_tightbbox"): _TIGHT for cls in ("matplotlib.artist.Artist", "matplotlib.axes.Axes")},
    ("matplotlib.artist.Artist", "contains"): _CONTAINS,
    **{(cls, "__call__"): _TICKS for cls in ("matplotlib.ticker.Locator", "matplotlib.ticker.MaxNLocator",
                                             "matplotlib.ticker.AutoMinorLocator", "matplotlib.dates.DateLocator")},
    **{(cls, "tick_values"): _TICK_VALUES for cls in ("matplotlib.ticker.Locator", "matplotlib.ticker.MaxNLocator",
                                                      "matplotlib.ticker.AutoMinorLocator")},
    **{(cls, "__call__"): _FORMAT for cls in ("matplotlib.ticker.Formatter", "matplotlib.ticker.ScalarFormatter")},
    ("matplotlib.ticker.ScalarFormatter", "get_offset"): (
        "def get_offset(self):\n    return 'offset ' + call('get_offset', super().get_offset)\n"),
    ("matplotlib.transforms.Transform", "transform_non_affine"): _TRANSFORM,
    ("matplotlib.transforms.Affine2DBase", "get_matrix"): _MATRIX,
    ("matplotlib.transforms.Affine2D", "get_matrix"): _MATRIX,
    ("matplotlib.colors.Normalize", "__call__"): _NORM,
    ("matplotlib.colors.BoundaryNorm", "__call__"): (
        "def __call__(self, value, clip=None):\n"
        "    return 255 - np.asarray(call('__call__', super().__call__, (value, clip)))\n"),
    ("matplotlib.colors.Colormap", "__call__"): _CMAP,
    ("matplotlib.image.AxesImage", "get_transform"): _TRANSFORM_GETTER,
    ("matplotlib.image.AxesImage", "get_extent"): (
        "def get_extent(self):\n    call('get_extent', super().get_extent)\n    return (0, 3, 0, 2)\n"),
    ("matplotlib.patches.PathPatch", "set_transform"): (
        "def set_transform(self, t):\n"
        "    call('set_transform', super().set_transform, (transforms.Affine2D().scale(0.5) + t,))\n"),
    ("matplotlib.collections.Collection", "get_paths"): _PATHS,
    ("matplotlib.scale.ScaleBase", "limit_range_for_scale"): (
        "def limit_range_for_scale(self, vmin, vmax, minpos):\n"
        "    vmin, vmax = call('limit_range_for_scale', super().limit_range_for_scale, (vmin, vmax, minpos))\n"
        "    return max(vmin, 1.5), vmax\n"),
    ("matplotlib.scale.ScaleBase", "get_transform"): (
        "def get_transform(self):\n"
        "    call('get_transform', super().get_transform)\n"
        "    return scale.FuncTransform(np.sqrt, np.square)\n"),
    ("matplotlib.scale.ScaleBase", "set_default_locators_and_formatters"): (
        "def set_default_locators_and_formatters(self, axis):\n"
        "    call('set_default_locators_and_formatters', super().set_default_locators_and_formatters, (axis,))\n"
        "    axis.set_major_locator(ticker.MultipleLocator(2))\n"),
    ("matplotlib.units.ConversionInterface", "convert"): (
        "@staticmethod\ndef convert(value, unit, axis):\n"
        "    return np.asarray(call('convert', Impl.convert, (value, unit, axis))) * 2\n"),
    ("matplotlib.units.ConversionInterface", "axisinfo"): (
        "@staticmethod\ndef axisinfo(unit, axis):\n"
        "    call('axisinfo', Impl.axisinfo, (unit, axis))\n"
        "    return units.AxisInfo(majloc=ticker.MultipleLocator(0.5), label='changed')\n"),
    ("matplotlib.dates.DateConverter", "axisinfo"): (
        "def axisinfo(self, unit, axis):\n"
        "    info = call('axisinfo', super().axisinfo, (unit, axis))\n"
        "    info.label = 'changed'\n"
        "    return info\n"),
    ("matplotlib.legend_handler.HandlerBase", "create_artists"): (
        "def create_artists(self, legend, orig_handle, xdescent, ydescent, width, height, fontsize, trans):\n"
        "    call('create_artists', super().create_artists, (legend, orig_handle, xdescent, ydescent, width, height,"
        " fontsize, trans))\n"
        "    return [patches.Circle((width / 2, height / 2), height / 2, transform=trans, color='r')]\n"),
    ("matplotlib.legend_handler.HandlerLine2D", "create_artists"): (
        "def create_artists(self, *args):\n"
        "    made = call('create_artists', super().create_artists, args)\n"
        "    for artist in made:\n"
        "        artist.set_color('r')\n"
        "    return made\n"),
    ("matplotlib.layout_engine.LayoutEngine", "execute"): (
        "def execute(self, fig):\n"
        "    call('execute', super().execute, (fig,))\n"
        "    fig.subplots_adjust(left=0.4, top=0.7)\n"),
    ("matplotlib.gridspec.GridSpec", "get_grid_positions"): (
        "def get_grid_positions(self, fig):\n"
        "    bottoms, tops, lefts, rights = call('get_grid_positions', super().get_grid_positions, (fig,))\n"
        "    return bottoms, tops, lefts * 0.5, rights * 0.9\n"),
    ("matplotlib.gridspec.GridSpec", "get_subplot_params"): (
        "def get_subplot_params(self, figure=None):\n"
        "    call('get_subplot_params', super().get_subplot_params, (figure,))\n"
        "    return matplotlib.figure.SubplotParams(left=0.3, right=0.95, wspace=0.6)\n"),
    ("matplotlib.gridspec.SubplotSpec", "get_position"): (
        "def get_position(self, figure):\n"
        "    call('get_position', super().get_position, (figure,))\n"
        "    return transforms.Bbox([[0.3, 0.3], [0.8, 0.8]])\n"),
    ("matplotlib.axis.XAxis", "get_tick_space"): (
        "def get_tick_space(self):\n    call('get_tick_space', super().get_tick_space)\n    return 2\n"),
    ("matplotlib.axis.XAxis", "get_view_interval"): (
        "def get_view_interval(self):\n"
        "    low, high = call('get_view_interval', super().get_view_interval)\n"
        "    return low, (low + high) / 2\n"),
    ("matplotlib.axis.XTick", "update_position"): (
        "def update_position(self, loc):\n    call('update_position', super().update_position, (loc + 0.5,))\n"),
    ("matplotlib.axes.Axes", "_gen_axes_patch"): (
        "def _gen_axes_patch(self):\n"
        "    call('_gen_axes_patch', super()._gen_axes_patch)\n"
        "    return patches.Circle((0.5, 0.5), 0.5)\n"),
    ("matplotlib.axes.Axes", "_update_title_position"): (
        "def _update_title_position(self, renderer):\n"
        "    call('_update_title_position', lambda renderer: None, (renderer,))\n"),
    ("matplotlib.axes.Axes", "autoscale_view"): (
        "def autoscale_view(self, tight=None, scalex=True, scaley=True):\n"
        "    call('autoscale_view', lambda *a: None, (tight, scalex, scaley))\n"),
    ("matplotlib.axes.Axes", "get_xaxis_transform"): (
        "def get_xaxis_transform(self, which='grid'):\n"
        "    return transforms.Affine2D().translate(0, 10) + call('get_xaxis_transform', "
        "super().get_xaxis_transform, (which,))\n"),
    ("matplotlib.axes.Axes", "format_coord"): (
        "def format_coord(self, x, y):\n"
        "    return 'at ' + call('format_coord', super().format_coord, (x, y))\n"),
    ("matplotlib.patheffects.AbstractPathEffect", "draw_path"): (
        "def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):\n"
        "    call('draw_path', super().draw_path, (renderer, gc, tpath, affine + transforms.Affine2D()"
        ".translate(5, -5), rgbFace))\n"),
    ("matplotlib.figure.Figure", "get_tightbbox"): _TIGHT,
}

#: (matplotlib's class, member) -> the class attribute's changed value.
ATTRIBUTES: dict[tuple[str, str], str] = {
    ("matplotlib.artist.Artist", "zorder"): "0.5",
    ("matplotlib.axes.Axes", "_axis_names"): "('x', 'y')",
    ("matplotlib.transforms.Transform", "is_separable"): "True",
    ("matplotlib.transforms.Transform", "has_inverse"): "False",
    ("matplotlib.layout_engine.LayoutEngine", "_adjust_compatible"): "False",
    ("matplotlib.layout_engine.LayoutEngine", "_colorbar_gridspec"): "False",
}
