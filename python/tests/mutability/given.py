"""
Rows of objects a program gives an artist: an array given to a setter or
a plotting method, a path given to a patch, a list of paths given to a
collection, a box or a transform given to clip by, a list given as a
position. matplotlib keeps some of them as they are (a later change in
place shows) and copies others (it does not); each row changes the object
after giving it. ``same`` says whether the artist's getter answers with
the very object given.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import collections, colors, patches, ticker, transforms, tri
from pltrs.font_manager import FontProperties
from pltrs.lines import Line2D
from pltrs.path import Path

from . import scenes
from .measure import Row, Scene

#: The rows of this module.
ROWS = []

ALL = slice(None)


def row(name, make, mutate, read=None, back=None, frame=True):
    """
    Add the row *name*: *make* builds the subject in an Axes and returns
    ``(subject, given)``; *mutate* changes the object given in place;
    *read* gives what the subject then answers and *back* what its getter
    hands back for the object given (both default to nothing to read).
    """
    def build():
        fig, ax = scenes.figure_and_axes()
        made, given = make(ax)
        if frame:
            scenes.frame(ax)
        return Scene(fig, made, ax=ax, given=given)

    def same(scene):
        return None if back is None else back(scene) is scene.given
    ROWS.append(Row(f"given_{name}", build, lambda scene: scene.given, mutate,
                    read or back or (lambda scene: None), same))


def setitem(key, value):
    """A change in place: ``target[key] = value``."""
    return lambda target: target.__setitem__(key, value)


def scaled(array):
    """Change every value of *array* in place."""
    array *= 0.5
    array += 0.25


def _xs():
    return np.array([0.0, 1.0, 2.0, 3.0])


def _ys():
    return np.array([0.0, 2.0, 1.0, 3.0])


def _triangle():
    return np.array([[0.0, 0.0], [3.0, 0.0], [1.5, 3.0], [0.0, 0.0]])


# Lines.


def _plot(ax):
    x = _xs()
    return ax.plot(x, _ys())[0], x


def _line2d(ax):
    y = _ys()
    return ax.add_line(Line2D(_xs(), y)), y


def _set_data(ax):
    line, = ax.plot([0, 1], [0, 1])
    y = _ys()
    line.set_data(_xs(), y)
    return line, y


def _set_ydata(ax):
    line, = ax.plot(_xs(), _ys())
    y = _ys()[::-1].copy()
    line.set_ydata(y)
    return line, y


def _markevery(ax):
    every = [0, 2]
    return ax.plot(_xs(), _ys(), "o-", markevery=every)[0], every


def _dashes(ax):
    dashes = [4.0, 2.0]
    line, = ax.plot(_xs(), _ys())
    line.set_dashes(dashes)
    return line, dashes


row("plot_x", _plot, scaled, back=lambda s: s.obj.get_xdata(orig=True))
row("line2d_y", _line2d, scaled, back=lambda s: s.obj.get_ydata(orig=True))
row("line_set_data_y", _set_data, scaled, back=lambda s: s.obj.get_ydata(orig=True))
row("line_set_ydata", _set_ydata, scaled, back=lambda s: s.obj.get_ydata(orig=True))
row("line_markevery", _markevery, setitem(1, 3), back=lambda s: s.obj.get_markevery())
row("line_dashes", _dashes, setitem(0, 1.0), read=lambda s: s.obj.get_linestyle())

# Collections.


def _scatter(ax):
    return ax.scatter(_xs(), _ys(), s=[40, 60, 80, 100], c=[0.1, 0.4, 0.6, 0.9])


def _setter(build, name, value):
    """A builder giving the subject of *build* the value *value* makes
    through its setter *name*."""
    def make(ax):
        subject, given = build(ax), value()
        getattr(subject, name)(given)
        return subject, given
    return make


def _scatter_x(ax):
    x = _xs()
    return ax.scatter(x, _ys()), x


row("scatter_x", _scatter_x, scaled, back=lambda s: s.obj.get_offsets())
row("set_offsets", _setter(_scatter, "set_offsets", lambda: np.column_stack([_ys(), _xs()])),
    scaled, back=lambda s: s.obj.get_offsets())
row("set_sizes", _setter(_scatter, "set_sizes", lambda: np.array([300.0, 20, 20, 20])), scaled,
    back=lambda s: s.obj.get_sizes())
row("set_array", _setter(_scatter, "set_array", lambda: np.array([0.9, 0.1, 0.5, 0.3])), scaled,
    back=lambda s: s.obj.get_array())
row("set_facecolor",
    _setter(lambda ax: ax.scatter(_xs(), _ys(), color="b"), "set_facecolor",
            lambda: np.array([[1.0, 0, 0, 1], [0, 1.0, 0, 1], [0, 0, 1.0, 1], [0, 0, 0, 1]])),
    scaled, back=lambda s: s.obj.get_facecolor())
row("set_linewidths", _setter(_scatter, "set_linewidths", lambda: [4.0, 1.0, 1.0, 1.0]),
    setitem(0, 9.0), back=lambda s: s.obj.get_linewidths())


def _path_collection(ax):
    paths = [Path(_triangle()), Path(_triangle() / 3 + 2)]
    return ax.add_collection(collections.PathCollection(paths, facecolors="c",
                                                        transform=ax.transData)), paths


def _path_collection_vertices(ax):
    vertices = _triangle()
    return ax.add_collection(collections.PathCollection([Path(vertices)], facecolors="c",
                                                        transform=ax.transData)), vertices


def _set_paths(ax):
    collection, _ = _path_collection(ax)
    paths = [Path(_triangle() / 2 + 1)]
    collection.set_paths(paths)
    return collection, paths


def _poly_collection(ax):
    vertices = _triangle()
    return ax.add_collection(collections.PolyCollection([vertices], facecolors="c")), vertices


def _set_verts(ax):
    collection, _ = _poly_collection(ax)
    vertices = _triangle() / 2 + 1
    collection.set_verts([vertices])
    return collection, vertices


def _line_collection(ax):
    segment = np.array([[0.0, 0.0], [3.0, 3.0]])
    return ax.add_collection(collections.LineCollection([segment, segment[::-1] * [1, 0.5]])), segment


def _set_segments(ax):
    collection, _ = _line_collection(ax)
    segment = np.array([[0.0, 3.0], [3.0, 0.0]])
    collection.set_segments([segment])
    return collection, segment


row("path_collection_paths", _path_collection, lambda paths: paths.pop(),
    back=lambda s: s.obj.get_paths())
row("path_collection_path_vertices", _path_collection, lambda paths: scaled(paths[0].vertices),
    read=lambda s: s.obj.get_paths()[0].vertices)
row("path_collection_vertices_array", _path_collection_vertices, scaled,
    back=lambda s: s.obj.get_paths()[0].vertices)
row("set_paths", _set_paths, lambda paths: scaled(paths[0].vertices),
    back=lambda s: s.obj.get_paths())
row("poly_collection_verts", _poly_collection, scaled, read=lambda s: s.obj.get_paths()[0].vertices)
row("set_verts", _set_verts, scaled, read=lambda s: s.obj.get_paths()[0].vertices)
row("line_collection_segment", _line_collection, scaled,
    back=lambda s: s.obj.get_paths()[0].vertices)
row("set_segments", _set_segments, scaled, back=lambda s: s.obj.get_paths()[0].vertices)

# Patches.


def _polygon(ax):
    xy = _triangle()
    return ax.add_patch(patches.Polygon(xy)), xy


def _set_xy(ax):
    polygon, _ = _polygon(ax)
    xy = _triangle() / 2 + 1
    polygon.set_xy(xy)
    return polygon, xy


def _path_patch(ax):
    path = Path(_triangle(), [1, 2, 2, 79])
    return ax.add_patch(patches.PathPatch(path)), path


def _path_patch_vertices(ax):
    vertices = _triangle()
    return ax.add_patch(patches.PathPatch(Path(vertices, [1, 2, 2, 79]))), vertices


def _set_path(ax):
    patch, _ = _path_patch(ax)
    path = Path(_triangle() / 2 + 1)
    patch.set_path(path)
    return patch, path


def _circle(ax):
    center = [1.5, 1.5]
    return ax.add_patch(patches.Circle(center, 1.0)), center


def _arrow_patch(ax):
    start = [0.2, 0.2]
    return ax.add_patch(patches.FancyArrowPatch(start, [2.8, 2.5], arrowstyle="-|>",
                                                mutation_scale=20)), start


def _annotation(ax):
    point = [0.5, 0.5]
    return ax.annotate("note", xy=point, xytext=(2.5, 3), arrowprops=dict(arrowstyle="->")), point


row("polygon_xy", _polygon, scaled, back=lambda s: s.obj.get_xy())
row("set_xy", _set_xy, scaled, back=lambda s: s.obj.get_xy())
row("path_patch_path", _path_patch, lambda path: scaled(path.vertices),
    back=lambda s: s.obj.get_path())
row("path_patch_path_assigned", _path_patch,
    lambda path: setattr(path, "vertices", _triangle() / 2 + 1), back=lambda s: s.obj.get_path())
row("path_patch_vertices_array", _path_patch_vertices, scaled,
    back=lambda s: s.obj.get_path().vertices)
row("set_path", _set_path, lambda path: scaled(path.vertices), back=lambda s: s.obj.get_path())
row("circle_center_list", _circle, setitem(0, 0.5), back=lambda s: s.obj.get_center())
row("arrow_patch_position", _arrow_patch, setitem(1, 2.5),
    read=lambda s: s.obj.get_path().vertices)
row("annotation_xy", _annotation, setitem(0, 2.0), back=lambda s: s.obj.xy)

# Images and meshes.


def _data():
    return np.arange(12.0).reshape(3, 4)


def _imshow(ax):
    data = _data()
    return ax.imshow(data), data


def _extent(ax):
    extent = [0.0, 3.0, 0.0, 3.0]
    return ax.imshow(_data(), extent=extent), extent


def _mesh(ax):
    return ax.pcolormesh(_data())


def _mesh_coordinates(ax):
    x = np.arange(5.0)
    return ax.pcolormesh(x, np.arange(4.0), _data()), x


row("imshow_data", _imshow, scaled, back=lambda s: s.obj.get_array())
row("image_set_data", _setter(lambda ax: ax.imshow(_data()), "set_data", lambda: _data()[::-1].copy()),
    scaled, back=lambda s: s.obj.get_array())
row("image_set_array", _setter(lambda ax: ax.imshow(_data()), "set_array", lambda: _data()[::-1].copy()),
    scaled, back=lambda s: s.obj.get_array())
row("imshow_extent", _extent, setitem(1, 2.0), back=lambda s: s.obj.get_extent())
row("image_set_extent",
    _setter(lambda ax: ax.imshow(_data()), "set_extent", lambda: [0.0, 3.0, 0.0, 3.0]),
    setitem(1, 2.0), back=lambda s: s.obj.get_extent())
row("mesh_set_array", _setter(_mesh, "set_array", lambda: _data()[::-1].copy()), scaled,
    back=lambda s: s.obj.get_array())
row("mesh_x", _mesh_coordinates, scaled, read=lambda s: s.obj.get_coordinates())

# Clipping and transforms.


def _clipped(ax):
    return ax.add_patch(patches.Rectangle((0, 0), 3, 3))


def _clip_box(ax):
    box = transforms.Bbox([[20.0, 20.0], [70.0, 70.0]])
    patch = _clipped(ax)
    patch.set_clip_box(box)
    return patch, box


def _clip_path(ax):
    path = Path(_triangle())
    patch = _clipped(ax)
    patch.set_clip_path(path, ax.transData)
    return patch, path


def _clip_path_transform(ax):
    moved = transforms.Affine2D().scale(0.5)
    patch = _clipped(ax)
    patch.set_clip_path(Path(_triangle()), moved + ax.transData)
    return patch, moved


def _clip_patch(ax):
    circle = ax.add_patch(patches.Circle((1.5, 1.5), 1.2, visible=False))
    patch = _clipped(ax)
    patch.set_clip_path(circle)
    return patch, circle


def _transformed(ax):
    moved = transforms.Affine2D().translate(0.5, 0)
    return ax.plot(_xs(), _ys(), transform=moved + ax.transData)[0], moved


def _set_transform(ax):
    moved = transforms.Affine2D().scale(0.5)
    patch = _clipped(ax)
    patch.set_transform(moved + ax.transData)
    return patch, moved


def _offset_transform(ax):
    moved = transforms.Affine2D().scale(0.5)
    return ax.scatter(_xs(), _ys(), transform=moved + ax.transData), moved


row("clip_box_points", _clip_box, lambda box: setitem((1, 0), 40.0)(box.get_points()),
    back=lambda s: s.obj.get_clip_box())
row("clip_box_x1", _clip_box, lambda box: setattr(box, "x1", 40.0),
    back=lambda s: s.obj.get_clip_box())
row("clip_path_vertices", _clip_path, lambda path: scaled(path.vertices),
    read=lambda s: s.obj.get_clip_path().get_transformed_path_and_affine()[0].vertices)
row("clip_path_transform", _clip_path_transform, lambda moved: moved.translate(1, 1),
    read=lambda s: s.given.get_matrix())
row("clip_patch_radius", _clip_patch, lambda circle: circle.set_radius(0.5),
    read=lambda s: s.given.get_radius())
row("transform_translate", _transformed, lambda moved: moved.translate(0.5, 0.5),
    read=lambda s: s.given.get_matrix())
row("transform_matrix", _transformed, lambda moved: setitem((1, 2), 0.7)(moved.get_matrix()),
    read=lambda s: s.given.get_matrix())
row("set_transform_scale", _set_transform, lambda moved: moved.scale(1.5),
    read=lambda s: s.given.get_matrix())
row("offset_transform_translate", _offset_transform, lambda moved: moved.translate(1, 0),
    read=lambda s: s.given.get_matrix())

# Texts.


def _boxed(ax):
    box = dict(boxstyle="round", fc="y")
    return ax.text(1, 1, "text", bbox=box), box


def _font(ax):
    font = FontProperties(size=10)
    return ax.text(1, 1, "text", fontproperties=font), font


def _position(ax):
    position = [1.0, 1.0]
    label = ax.text(0, 0, "text")
    label.set_position(position)
    return label, position


row("text_bbox_dict", _boxed, setitem("fc", "r"),
    read=lambda s: list(s.obj.get_bbox_patch().get_facecolor()))
row("text_fontproperties", _font, lambda font: font.set_size(30), read=lambda s: s.obj.get_fontsize(),
    back=lambda s: s.obj.get_fontproperties())
row("text_position", _position, setitem(0, 2.0), read=lambda s: list(s.obj.get_position()))

# Colour mapping.


def _norm(ax):
    norm = colors.Normalize(0, 11)
    return ax.imshow(_data(), norm=norm), norm


def _cmap(ax):
    cmap = plt.get_cmap("viridis").copy()
    return ax.imshow(np.ma.masked_less(_data(), 4), cmap=cmap), cmap


def _mesh_cmap(ax):
    cmap = plt.get_cmap("viridis").copy()
    return ax.pcolormesh(_data(), cmap=cmap), cmap


def _lut(cmap):
    cmap(0.5)
    cmap._lut[:, 0] = 1.0


def _lut_of(scene):
    scene.given(0.5)
    return scene.given._lut


row("norm_vmax", _norm, lambda norm: setattr(norm, "vmax", 5.0), read=lambda s: s.obj.get_clim(),
    back=lambda s: s.obj.norm)
row("cmap_set_bad", _cmap, lambda cmap: cmap.set_bad("red"), read=lambda s: list(s.given.get_bad()),
    back=lambda s: s.obj.get_cmap())
row("cmap_lut", _cmap, _lut, read=_lut_of, back=lambda s: s.obj.get_cmap())
row("mesh_cmap_lut", _mesh_cmap, _lut, read=_lut_of,
    back=lambda s: s.obj.get_cmap())

# Ticks.


def _plotted(ax):
    ax.plot([0, 10], [0, 1])
    return ax


def _fixed_locator(ax):
    locs = np.array([1.0, 4.0, 8.0])
    _plotted(ax).xaxis.set_major_locator(ticker.FixedLocator(locs))
    return ax, locs


def _set_xticks(ax):
    locs = np.array([1.0, 4.0, 8.0])
    _plotted(ax).set_xticks(locs)
    return ax, locs


def _set_xticklabels(ax):
    names = ["a", "b", "c"]
    _plotted(ax).set_xticks([1, 4, 8])
    ax.set_xticklabels(names)
    return ax, names


def _labels(scene):
    scene.fig.canvas.draw()
    return [[label.get_text() for label in scene.obj.get_xticklabels()],
            list(scene.obj.get_xticks())]


row("fixed_locator_locs", _fixed_locator, setitem(1, 6.0), read=_labels, frame=False)
row("set_xticks", _set_xticks, setitem(1, 6.0), read=_labels, frame=False)
row("set_xticklabels", _set_xticklabels, setitem(1, "changed"), read=_labels, frame=False)

# Fields, triangulations and layouts.


def _quiver(ax):
    u = np.array([1.0, -1.0])
    return ax.quiver([0.5, 2.0], [0.5, 2.0], u, [1.0, 2.0]), u


def _gouraud(ax):
    triangulation = tri.Triangulation(np.array([0.0, 3.0, 0.0, 3.0]), np.array([0.0, 0.0, 3.0, 3.0]))
    return ax.tripcolor(triangulation, [1.0, 2.0, 3.0, 4.0], shading="gouraud"), triangulation.x


def _triplot(ax):
    x = np.array([0.0, 3.0, 0.0, 3.0])
    ax.triplot(tri.Triangulation(x, np.array([0.0, 0.0, 3.0, 3.0])))
    return ax, x


def _ratios(ax):
    ratios = [1, 3]
    fig = ax.figure
    fig.clear()
    grid = fig.add_gridspec(1, 2, width_ratios=ratios)
    fig.add_subplot(grid[0])
    fig.add_subplot(grid[1])
    return grid, ratios


def _fill_between(ax):
    y = _ys()
    return ax.fill_between(_xs(), y, 0.0), y


def _bar(ax):
    heights = np.array([1.0, 3.0, 2.0])
    return ax.bar([0, 1, 2], heights), heights


def _contour(ax):
    grid = np.linspace(-1.0, 1.0, 10)
    field = np.add.outer(grid, grid)
    return ax.contourf(grid, grid, field, levels=3), field


row("quiver_u", _quiver, scaled, back=lambda s: s.obj.U)
row("tripcolor_gouraud_x", _gouraud, scaled)
row("triplot_x", _triplot, scaled)
row("gridspec_width_ratios", _ratios, setitem(0, 6), back=lambda s: s.obj.get_width_ratios(),
    frame=False)
row("fill_between_y", _fill_between, scaled, read=lambda s: s.obj.get_paths()[0].vertices)
row("bar_heights", _bar, scaled, read=lambda s: [patch.get_height() for patch in s.obj])
row("contourf_field", _contour, scaled, read=lambda s: s.obj.get_paths()[1].vertices)


# Values handed back. matplotlib stores what many setters are given as it
# is, and the getter returns that very object: a tuple, a slice, a float
# or a string as much as an array or a list. Nothing can be changed in
# them; the rows only ask whether the getter answers with the object.


def back(name, make, getter):
    """Add the row ``back_<name>``: *make* builds the subject in an Axes
    with a value and returns ``(subject, value)``; ``same`` says whether
    *getter* then answers with the very value."""
    def build():
        fig, ax = scenes.figure_and_axes()
        made, given = make(ax)
        return Scene(fig, made, ax=ax, given=given)
    ROWS.append(Row(f"back_{name}", build, getter,
                    same=lambda scene: getter(scene) is scene.given))


def _back_value(kind):
    """A new object of *kind*, no constant of the interpreter's."""
    return {
        "float": lambda: float("0.125"),
        "int": lambda: int("12345"),
        "tuple": lambda: tuple([3, 7]),
        "floats": lambda: tuple([0.05, 0.2]),
        "slice": lambda: slice(2, 60, 9),
        "rgb": lambda: tuple([0.25, 0.5, 0.75]),
        "str": lambda: "".join(["na", "me"]),
        "point": lambda: tuple([1.5, 2.5]),
        "colour": lambda: "".join(["#12", "3456"]),
    }[kind]()


def _back_plotted(keyword, kind):
    """A line plotted with *keyword* a new value of *kind*."""
    def make(ax):
        value = _back_value(kind)
        line, = ax.plot(np.arange(101.0), marker="o", **{keyword: value})
        return line, value
    return make


def _back_set(make_artist, setter, kind):
    """An artist *make_artist* makes in an Axes, its *setter* called with
    a new value of *kind*."""
    def make(ax):
        artist, value = make_artist(ax), _back_value(kind)
        getattr(artist, setter)(value)
        return artist, value
    return make


def _back_line(ax):
    return ax.plot(np.arange(101.0), marker="o")[0]


def _back_rectangle(ax):
    return ax.add_patch(patches.Rectangle((0.5, 0.5), 2.0, 1.0))


def _back_label(ax):
    return ax.text(1.0, 1.0, "text")


def _back_scatter(ax):
    return ax.scatter(_xs(), _ys())


for _kind in ("tuple", "slice", "float", "floats", "int"):
    back(f"plot_markevery_{_kind}", _back_plotted("markevery", _kind),
         lambda scene: scene.obj.get_markevery())
    back(f"set_markevery_{_kind}", _back_set(_back_line, "set_markevery", _kind),
         lambda scene: scene.obj.get_markevery())
for _maker, _artist in ((_back_line, "line"), (_back_rectangle, "patch"), (_back_label, "text"),
                        (_back_scatter, "scatter")):
    for _property, _kind in (("zorder", "float"), ("alpha", "float"), ("gid", "str"),
                             ("url", "str"), ("label", "str"), ("picker", "float")):
        back(f"{_artist}_{_property}", _back_set(_maker, f"set_{_property}", _kind),
             lambda scene, _property=_property: getattr(scene.obj, f"get_{_property}")())
for _property, _kind in (("color", "rgb"), ("markerfacecolor", "rgb"), ("markeredgecolor", "rgb"),
                         ("linewidth", "float"), ("markersize", "float"),
                         ("pickradius", "float"), ("color", "colour")):
    back(f"line_{_property}_{_kind}", _back_set(_back_line, f"set_{_property}", _kind),
         lambda scene, _property=_property: getattr(scene.obj, f"get_{_property}")())
    back(f"plot_{_property}_{_kind}", _back_plotted(_property, _kind),
         lambda scene, _property=_property: getattr(scene.obj, f"get_{_property}")())
for _property, _kind in (("color", "rgb"), ("text", "str"), ("linespacing", "float"),
                         ("backgroundcolor", "rgb"), ("fontfamily", "str")):
    back(f"text_{_property}_{_kind}", _back_set(_back_label, f"set_{_property}", _kind),
         lambda scene, _property=_property: getattr(scene.obj, f"get_{_property}")())
for _property, _kind in (("hatch", "str"), ("linewidth", "float")):
    back(f"patch_{_property}_{_kind}", _back_set(_back_rectangle, f"set_{_property}", _kind),
         lambda scene, _property=_property: getattr(scene.obj, f"get_{_property}")())
back("circle_center_point",
     lambda ax: (lambda point: (ax.add_patch(patches.Circle(point, 0.5)), point))(_back_value("point")),
     lambda scene: scene.obj.center)
back("ellipse_center_point",
     lambda ax: (lambda point: (ax.add_patch(patches.Ellipse(point, 1.0, 0.5)), point))(
         _back_value("point")),
     lambda scene: scene.obj.center)
back("annotation_xy_point",
     lambda ax: (lambda point: (ax.annotate("note", point), point))(_back_value("point")),
     lambda scene: scene.obj.xy)
back("axes_aspect_float", lambda ax: (lambda value: (ax.set_aspect(value), ax)[1:] + (value,))(
    _back_value("float")), lambda scene: scene.obj.get_aspect())
back("axes_title_str", lambda ax: (lambda value: (ax.set_title(value), ax)[1:] + (value,))(
    _back_value("str")), lambda scene: scene.obj.get_title())
back("axes_xlabel_str", lambda ax: (lambda value: (ax.set_xlabel(value), ax)[1:] + (value,))(
    _back_value("str")), lambda scene: scene.obj.get_xlabel())
