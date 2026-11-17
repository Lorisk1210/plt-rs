"""
The 3-D artists' own state.

matplotlib's `~mpl_toolkits.mplot3d.art3d` classes keep what they are
given for the third dimension in plain attributes (``_verts3d``,
``_segments3d``, ``_segment3d``, ``_offsets3d``, ``_faces``, ...), project
it at every draw and leave the projected values behind (``_vzs``,
``_facecolors2d``, ``_path2d``, ...). Programs and libraries read these
attributes, change them in place, assign them, and subclass the artists
to draw from them (qiskit's Bloch sphere draws its arrows from
``_segment3d``).

This module measures that state for every way a 3-D artist is made: the
classes made directly, the functions that convert a 2-D artist, and the
`.Axes3D` methods that return one (`SUBJECTS`). For each subject:

* every field of `FIELDS` matplotlib's artist has, before or after a
  draw (which those are is matplotlib's to say: `discover`): its type and
  value at both moments (a `State` row), the objects inside it changed in
  place and the field assigned a changed copy (ordinary rows: read back,
  drawn, kept, pickled, before and after the first draw);
* every getter and setter the `art3d` classes define (`CALLS`): what a
  getter hands out, changed in place; what a setter leaves in the fields,
  in the drawing and in a pickle; what ``do_3d_projection()`` returns;
* for every class, a subclass of the program's that overrides
  ``do_3d_projection`` and ``draw`` and reads the fields there (a
  `Subclass` row): how often it is called and what it sees; and qiskit's
  arrow itself (`Arrow` row): what it reads and whether its head is drawn.
"""

import copy
import pickle
import warnings

import numpy as np

import pltrs.pyplot as plt
from pltrs import collections as mcoll, lines as mlines, patches as mpatches, text as mtext
from pltrs.mpl_toolkits.mplot3d import art3d, proj3d
from pltrs.path import Path

from . import surface
from .measure import Row, Scene, number, pixels, plain

#: The fields matplotlib's 3-D artists keep for the third dimension and
#: those a projection leaves behind, measured where a subject has them.
#: (``_sort_zpos`` is left out: the position a program sorts an artist at
#: has checks of its own.)
FIELDS = (
    "_verts3d", "_segments3d", "_segment3d", "_code3d", "_path2d", "_3dverts_codes",
    "_offsets3d", "_vzs", "_z_markers_idx", "_depthshade", "_depthshade_minalpha",
    "_sizes3d", "_linewidths3d", "_offset_zordered", "_data_scale", "_in_draw",
    "_faces", "_invalid_vertices", "_codes3d", "_vec", "_segslices", "_closed",
    "_facecolor3d", "_edgecolor3d", "_alpha3d", "_facecolors2d", "_edgecolors2d",
    "_zsortfunc", "_x", "_y", "_z", "_dir_vec", "_axlim_clip",
)

XS = np.linspace(0.0, 3.0, 7)
YS = np.sin(XS) + 1.5
ZS = np.array([1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0])

#: The names of the arrays above, which the runner puts back after each
#: row (see `.scenes.DATA`).
DATA = ("XS", "YS", "ZS")
_PRISTINE = {name: globals()[name].copy() for name in DATA}


def _grid():
    """A small grid of x and y, and heights on it."""
    xs, ys = np.meshgrid(np.linspace(-1.0, 2.0, 6), np.linspace(-1.0, 1.0, 5))
    return xs, ys, np.add.outer(np.linspace(-1.0, 1.0, 5), np.linspace(-1.0, 2.0, 6))


def _segments3d():
    return [np.array([[0.0, 0.0, 0.0], [1.0, 2.0, 1.0], [2.0, 1.0, 3.0]]),
            np.array([[3.0, 0.0, 1.0], [2.0, 2.0, 2.0], [1.0, 3.0, 0.0]])]


def _segments2d():
    return [np.array([[0.0, 0.0], [1.0, 2.0], [2.0, 1.0]]),
            np.array([[3.0, 0.0], [2.0, 2.0], [1.0, 3.0]])]


def _faces3d():
    return [np.array([[0.0, 0.0, 0.0], [2.0, 0.0, 1.0], [2.0, 2.0, 2.0], [0.0, 2.0, 1.0]]),
            np.array([[1.0, 1.0, 3.0], [3.0, 1.0, 3.0], [3.0, 3.0, 2.0], [1.0, 3.0, 2.0]])]


def _faces2d():
    return [np.array([[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]]),
            np.array([[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]])]


def _outline():
    return Path(np.array([[0.0, 0.0], [2.0, 0.5], [1.0, 2.5], [0.0, 0.0]]),
                [Path.MOVETO, Path.LINETO, Path.LINETO, Path.CLOSEPOLY])


#: Every subject: its name and the function making it in a 3-D Axes. The
#: makers of the classes made directly take the class to make, so that a
#: subclass of the program's is made the same way (`CLASSES`).
SUBJECTS = {}

#: The makers of the `art3d` classes made directly, by class name.
CLASSES = {}


def subject(function):
    """Register *function* as the maker of the subject of its name."""
    SUBJECTS[function.__name__] = function
    return function


def direct(name):
    """Register the function decorated as the maker of the class *name*
    made directly: the subject ``<function name>``, and the class' entry
    of `CLASSES`."""
    def register(function):
        CLASSES[name] = function
        SUBJECTS[function.__name__] = lambda ax: function(ax, getattr(art3d, name))
        return function
    return register


@direct("Line3D")
def line3d_made(ax, cls):
    line = cls(XS, YS, ZS, linewidth=4)
    ax.add_line(line)
    return line


@direct("Line3DCollection")
def line3d_collection_made(ax, cls):
    lines = cls(_segments3d(), linewidths=4)
    ax.add_collection3d(lines)
    return lines


@direct("Patch3D")
def patch3d_made(ax, cls):
    # The class' own constructor does not take what its
    # `set_3d_properties` takes; programs make it as qiskit does.
    patch = cls.__new__(cls)
    mpatches.Patch.__init__(patch, facecolor="tab:orange")
    patch.set_3d_properties([(0.0, 0.0), (2.0, 0.5), (1.0, 2.5)], zs=1.0, zdir="z")
    ax.add_patch(patch)
    return patch


@direct("PathPatch3D")
def pathpatch3d_made(ax, cls):
    patch = cls(_outline(), zs=1.0, zdir="z", facecolor="tab:orange")
    ax.add_patch(patch)
    return patch


@direct("Patch3DCollection")
def patch3d_collection_made(ax, cls):
    # Markers of ten pixels at two points, as a scatter of patches.
    patches = cls([mpatches.Circle((0.0, 0.0), 10)], offsets=[[1.0, 1.0], [2.0, 3.0]],
                  offset_transform=ax.transData, transform=None, zs=[1.0, 2.0], zdir="z")
    ax.add_collection3d(patches)
    return patches


@direct("Path3DCollection")
def path3d_collection_made(ax, cls):
    paths = cls([Path.unit_circle()], sizes=[300.0], offsets=np.column_stack([XS, YS]),
                offset_transform=ax.transData, zs=ZS, zdir="z")
    ax.add_collection3d(paths)
    return paths


@direct("Poly3DCollection")
def poly3d_collection_made(ax, cls):
    polygons = cls(_faces3d(), facecolors=["tab:blue", "tab:green"])
    ax.add_collection3d(polygons)
    return polygons


@direct("Text3D")
def text3d_made(ax, cls):
    label = cls(1.0, 1.0, 1.0, "deep", zdir="x", fontsize=30)
    ax.add_artist(label)
    return label


@subject
def line_2d_to_3d(ax):
    line = mlines.Line2D(XS, YS, linewidth=4)
    ax.add_line(line)
    art3d.line_2d_to_3d(line, zs=1.0, zdir="y")
    return line


@subject
def patch_2d_to_3d(ax):
    patch = mpatches.Circle((1.0, 1.0), 0.8)
    ax.add_patch(patch)
    art3d.patch_2d_to_3d(patch, z=1.0, zdir="x")
    return patch


@subject
def pathpatch_2d_to_3d(ax):
    patch = mpatches.PathPatch(_outline())
    ax.add_patch(patch)
    art3d.pathpatch_2d_to_3d(patch, z=1.0, zdir="y")
    return patch


@subject
def collection_2d_to_3d(ax):
    paths = mcoll.PathCollection([_outline()])
    ax.add_collection(paths)
    art3d.collection_2d_to_3d(paths, zs=1.0, zdir="z")
    return paths


@subject
def line_collection_2d_to_3d(ax):
    lines = mcoll.LineCollection(_segments2d(), linewidths=4)
    ax.add_collection3d(lines, zs=1.0, zdir="y")
    return lines


@subject
def patch_collection_2d_to_3d(ax):
    # (In the units of the projection, as matplotlib draws such patches.)
    patches = mcoll.PatchCollection([mpatches.Circle((0.0, 0.0), 0.02)])
    ax.add_collection3d(patches, zs=1.0, zdir="z")
    return patches


@subject
def poly_collection_2d_to_3d(ax):
    polygons = mcoll.PolyCollection(_faces2d())
    ax.add_collection3d(polygons, zs=[1.0, 2.0], zdir="y")
    return polygons


@subject
def text_2d_to_3d(ax):
    label = mtext.Text(1.0, 1.0, "deep", fontsize=30)
    ax.add_artist(label)
    art3d.text_2d_to_3d(label, z=1.0, zdir="y")
    return label


@subject
def plot(ax):
    return ax.plot(XS, YS, ZS, linewidth=4)[0]


@subject
def scatter(ax):
    return ax.scatter(XS, YS, ZS, c=ZS, s=200)


@subject
def scatter_plain(ax):
    return ax.scatter(XS, YS, ZS, s=200, depthshade=False)


@subject
def bar3d(ax):
    return ax.bar3d([0.0, 1.0], [0.0, 1.0], [0.0, 0.0], 0.8, 0.8, [1.0, 2.0])


@subject
def plot_surface(ax):
    return ax.plot_surface(*_grid())


@subject
def plot_surface_mapped(ax):
    return ax.plot_surface(*_grid(), cmap="viridis")


@subject
def plot_wireframe(ax):
    return ax.plot_wireframe(*_grid())


@subject
def plot_trisurf(ax):
    return ax.plot_trisurf(XS, YS, ZS)


@subject
def quiver(ax):
    return ax.quiver([0.0, 1.0], [0.0, 1.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [1.0, 2.0],
                     length=1.0, arrow_length_ratio=0.3)


@subject
def stem_markers(ax):
    return ax.stem(XS, YS, ZS).markerline


@subject
def stem_lines(ax):
    return ax.stem(XS, YS, ZS).stemlines


@subject
def stem_base(ax):
    return ax.stem(XS, YS, ZS).baseline


@subject
def errorbar_lines(ax):
    return ax.errorbar(XS, YS, ZS, zerr=0.5, capsize=3)[0][0]


@subject
def errorbar_caps(ax):
    return ax.errorbar(XS, YS, ZS, zerr=0.5, capsize=3)[1][0]


@subject
def voxels(ax):
    filled = np.zeros((2, 2, 2), dtype=bool)
    filled[0, 0, 0] = filled[1, 1, 1] = True
    return ax.voxels(filled)[(0, 0, 0)]


@subject
def contour(ax):
    return ax.contour(*_grid(), levels=4)


@subject
def contourf(ax):
    return ax.contourf(*_grid(), levels=4)


@subject
def text(ax):
    return ax.text(1.0, 1.0, 1.0, "deep", zdir="x", fontsize=30)


@subject
def fill_between(ax):
    return ax.fill_between(XS, YS, ZS, XS, YS + 1.0, ZS + 1.0)


def frame(ax):
    """Fix the view of *ax*, so that nothing a row changes rescales it."""
    ax.set_xlim(-1, 4)
    ax.set_ylim(-1, 4)
    ax.set_zlim(-1, 4)


def build(name, make=None, drawn=False):
    """A new scene of the subject *name* (made by *make*, if given, in
    place of its registered maker), drawn once already if *drawn*."""
    for data, pristine in _PRISTINE.items():
        # (An artist that keeps the array it is given hands it out to be
        # changed: every scene is made of arrays of its own.)
        globals()[data] = pristine.copy()
    fig = plt.figure(figsize=(3, 3), dpi=40)
    ax = fig.add_subplot(projection="3d")
    made = (SUBJECTS[name] if make is None else make)(ax)
    frame(ax)
    if drawn:
        pixels(fig)
    return Scene(fig, made, ax=ax)


def typed(value, depth=0):
    """*value* as JSON says it, with its type: what `.measure.plain`
    gives, and the names of the types of the value and of the items of a
    sequence, the dtype of an array and whether it is masked."""
    name = type(value).__name__
    if isinstance(value, np.ndarray):
        return {"type": f"{name}[{value.dtype.kind}]", "value": plain(np.ma.getdata(value)),
                "masked": int(np.ma.getmaskarray(value).sum()) if np.ma.isMA(value) else None}
    if isinstance(value, (list, tuple)) and depth < 2:
        return {"type": name, "len": len(value),
                "items": [typed(item, depth + 1) for item in value[:3]]}
    if isinstance(value, slice):
        return {"type": name, "value": str(value)}
    if callable(value):
        return {"type": "callable", "value": getattr(value, "__name__", name)}
    return {"type": name, "value": plain(value)}


def field(artist, name):
    """The field *name* of *artist* as `typed` says it, or the name of the
    exception reading it raises."""
    try:
        return typed(getattr(artist, name))
    except Exception as err:
        return type(err).__name__


def fields(artist, names):
    """The fields *names* of *artist*, each as `field` says it."""
    return {name: field(artist, name) for name in names}


class Probe:
    """A row that answers with what its *measure* finds (a dict of plain
    values), where an ordinary row changes one object in place."""

    def __init__(self, name, measure):
        self.name = name
        self._measure = measure

    def measure(self):
        """What the row finds, or the name of the exception it raises."""
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            try:
                return self._measure()
            except Exception as err:
                return {"error": type(err).__name__}
            finally:
                plt.close("all")


def _state(subject_name, name):
    """The type and value of a field when the artist is made, after a
    draw, and read back from a pickle of the drawn figure."""
    def measure():
        scene = build(subject_name)
        found = {"made": field(scene.obj, name)}
        pixels(scene.fig)
        found["drawn"] = field(scene.obj, name)
        try:
            found["unpickled"] = field(pickle.loads(pickle.dumps(scene)).obj, name)
        except Exception as err:
            found["unpickled"] = f"pickle: {type(err).__name__}"
        return found
    return Probe(f"3d/{subject_name}.{name} state", measure)


def _changed_copy(value):
    """A copy of *value* with something in it changed, or
    `surface.Unchangeable`."""
    if isinstance(value, (bool, np.bool_)):
        return not value
    if isinstance(value, (int, float, np.number)):
        return value * 0.5 + 0.25
    changed = copy.deepcopy(value)
    for _, pick, mutate in surface.targets(changed):
        if mutate is not None:
            mutate(pick(changed))
            return changed
    raise surface.Unchangeable


def _in_place(subject_name, member, suffix, drawn, changed=True):
    """The row changing in place what *member* (a field, or a getter
    called: ``name()``) hands out at *suffix* (only its identity unless
    *changed*)."""
    def reach(scene):
        value = surface.value_of(scene.obj, member)
        for found, pick, _ in surface.targets(value):
            if found == suffix:
                return pick(value)
        raise LookupError(f"{member} hands out nothing at {suffix!r}")
    return Row(f"3d/{subject_name}.{member}{suffix}", lambda: build(subject_name, drawn=drawn),
               reach, (lambda target: surface._change(target, suffix)) if changed else None)


def _assigned(subject_name, name, drawn):
    """The row assigning the field *name* a changed copy of itself."""
    def assign(artist):
        setattr(artist, name, _changed_copy(getattr(artist, name)))
    return Row(f"3d/{subject_name}.{name} assigned", lambda: build(subject_name, drawn=drawn),
               lambda scene: scene.obj, assign,
               read=lambda scene: field(scene.obj, name), same=lambda scene: True)


def _halved(values):
    return np.asarray(values, dtype=float) * 0.5 + 0.25


def _set_data_3d(artist):
    xs, ys, zs = artist.get_data_3d()
    artist.set_data_3d(_halved(xs), _halved(ys), _halved(zs))


def _set_3d_properties(artist):
    if isinstance(artist, art3d.PathPatch3D):
        artist.set_3d_properties(_outline(), zs=2.0, zdir="y")
    elif isinstance(artist, art3d.Patch3D):
        artist.set_3d_properties([(0.0, 0.0), (1.0, 0.0), (1.0, 2.0)], zs=2.0, zdir="y")
    elif isinstance(artist, art3d.Poly3DCollection):
        artist.set_3d_properties()
    elif isinstance(artist, (art3d.Patch3DCollection, art3d.Path3DCollection)):
        artist.set_3d_properties(2.0, "y")
    elif isinstance(artist, art3d.Text3D):
        artist.set_3d_properties(z=2.0, zdir="y")
    else:
        artist.set_3d_properties(zs=2.0, zdir="y")


#: The setters the `art3d` classes define, each with a call of it on a
#: subject that has it.
SETTERS = {
    "set_data_3d": _set_data_3d,
    "set_3d_properties": _set_3d_properties,
    "set_segments": lambda artist: artist.set_segments(
        [[(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)], [(1.0, 0.0, 2.0), (2.0, 2.0, 0.0)]]),
    "set_verts": lambda artist: artist.set_verts(
        [[(0.0, 0.0, 0.0), (2.0, 0.0, 1.0), (2.0, 2.0, 3.0)]]),
    "set_verts_and_codes": lambda artist: artist.set_verts_and_codes(
        [[(0.0, 0.0, 0.0), (2.0, 0.0, 1.0), (2.0, 2.0, 3.0)]],
        [[Path.MOVETO, Path.LINETO, Path.LINETO]]),
    "set_zsort": lambda artist: artist.set_zsort("min"),
    "set_depthshade": lambda artist: artist.set_depthshade(not artist.get_depthshade()),
    "set_position_3d": lambda artist: artist.set_position_3d((2.0, 2.0, 2.0), zdir="y"),
    "set_z": lambda artist: artist.set_z(3.0),
    "set_sizes": lambda artist: artist.set_sizes([600.0]),
    "set_linewidth": lambda artist: artist.set_linewidth(6.0),
    "set_facecolor": lambda artist: artist.set_facecolor("red"),
    "set_edgecolor": lambda artist: artist.set_edgecolor("red"),
    "set_alpha": lambda artist: artist.set_alpha(0.5),
}

#: The getters the `art3d` classes define that take no argument.
GETTERS = ("get_data_3d", "get_position_3d", "get_path", "get_depthshade", "get_facecolor",
           "get_edgecolor")


def _defined_in_art3d(artist):
    """The names the `art3d` classes of *artist* define themselves."""
    names = set()
    for cls in type(artist).__mro__:
        if cls.__module__.endswith("mplot3d.art3d"):
            names.update(vars(cls))
    return names


def _setter(subject_name, name, names, drawn):
    """The row calling the setter *name*: what the fields *names* then
    hold, whether the next draw differs and a pickle carries it."""
    return Row(f"3d/{subject_name}.{name}(...)", lambda: build(subject_name, drawn=drawn),
               lambda scene: scene.obj, SETTERS[name],
               read=lambda scene: fields(scene.obj, names), same=lambda scene: True)


def _projection(subject_name, names):
    """What ``do_3d_projection()`` returns, before any draw and after one,
    and what the fields hold after it."""
    def measure():
        found = {}
        for moment, drawn in (("made", False), ("drawn", True)):
            scene = build(subject_name, drawn=drawn)
            try:
                found[moment] = number(scene.obj.do_3d_projection())
            except Exception as err:
                found[moment] = type(err).__name__
            found[f"{moment}_fields"] = fields(scene.obj, names)
        return found
    return Probe(f"3d/{subject_name}.do_3d_projection()", measure)


def discover():
    """
    What matplotlib's subjects have, by subject: the fields of `FIELDS`
    when made (``made``) and those only a draw leaves (``drawn``), for
    each the suffixes of the objects in it that can be changed in place
    (``changed``) and of those only read twice (``identity``), the same
    for what the getters of `GETTERS` its `art3d` classes define hand out,
    the fields that can be assigned a changed copy (``assigned``), and
    the setters of `SETTERS` its classes define. Found on the library at
    hand (matplotlib, when the reference is recorded).
    """
    found = {}
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        for name in SUBJECTS:
            scene = build(name)
            made = [name_ for name_ in FIELDS if name_ in vars(scene.obj)]
            pixels(scene.fig)
            drawn = [name_ for name_ in FIELDS if name_ in vars(scene.obj) and name_ not in made]
            defined = _defined_in_art3d(scene.obj)
            getters = [f"{getter}()" for getter in GETTERS if getter in defined]
            changed, identity, assigned = [], [], []
            for member in (*made, *drawn, *getters):
                value = surface.value_of(scene.obj, member)
                for suffix, _, mutate in surface.targets(value):
                    (identity if mutate is None else changed).append(member + suffix)
                if member in getters:
                    continue
                try:
                    _changed_copy(value)
                except Exception:
                    continue
                assigned.append(member)
            found[name] = {
                "class": type(scene.obj).__name__, "made": made, "drawn": drawn,
                "changed": changed, "identity": identity, "assigned": assigned,
                "setters": [setter for setter in SETTERS if setter in defined],
                "projects": "do_3d_projection" in defined,
            }
            plt.close(scene.fig)
    return found


def _split(path):
    """The member and the suffix of a recorded *path*."""
    return surface._MEMBER.fullmatch(path).groups()


class _Seen:
    """What a subclass of the program's saw of its fields."""

    def __init__(self, names):
        self.names = names
        self.projections = []
        self.draws = []


def _subclass(cls, seen):
    """A subclass of the `art3d` class *cls* that reads its fields where
    matplotlib calls it: in ``do_3d_projection`` (where the class has
    one) and in ``draw``."""
    members = {}
    if hasattr(cls, "do_3d_projection"):
        def do_3d_projection(self):
            seen.projections.append(fields(self, seen.names))
            return cls.do_3d_projection(self)
        members["do_3d_projection"] = do_3d_projection

    def draw(self, renderer):
        seen.draws.append(fields(self, seen.names))
        return cls.draw(self, renderer)
    members["draw"] = draw
    return type(f"Program{cls.__name__}", (cls,), members)


def _subclassed(class_name, subject_name, names):
    """How often matplotlib calls the methods a subclass of the program's
    overrides in two draws, what the subclass reads of its fields in the
    first call of each and in the last, and whether it is drawn."""
    def measure():
        seen = _Seen(names)
        made = _subclass(getattr(art3d, class_name), seen)
        scene = build(subject_name, make=lambda ax: CLASSES[class_name](ax, made))
        first = pixels(scene.fig)
        pixels(scene.fig)
        empty = build(subject_name, make=lambda ax: None)
        return {
            "projections": len(seen.projections), "draws": len(seen.draws),
            "seen_first_projection": seen.projections[0] if seen.projections else None,
            "seen_first_draw": seen.draws[0] if seen.draws else None,
            "seen_last_draw": seen.draws[-1] if seen.draws else None,
            "drawn": bool((first != pixels(empty.fig)).any()),
            "class": type(scene.obj).__mro__[1].__name__,
        }
    return Probe(f"3d/{class_name} subclassed", measure)


def _arrow_class():
    """qiskit's arrow (``qiskit.visualization.bloch.Arrow3D``): a patch
    that keeps its ends in ``_segment3d`` and draws itself as a fancy
    arrow between their projections."""
    class Arrow3D(art3d.Patch3D, mpatches.FancyArrowPatch):
        __module__ = "mpl_toolkits.mplot3d.art3d"

        def __init__(self, xs, ys, zs, zdir="z", **kwargs):
            mpatches.FancyArrowPatch.__init__(self, (0, 0), (0, 0), **kwargs)
            self.set_3d_properties(tuple(zip(xs, ys)), zs, zdir)
            self._path2d = None
            self.seen = []

        def draw(self, renderer):
            self.seen.append(field(self, "_segment3d"))
            xs3d, ys3d, zs3d = zip(*self._segment3d)
            x_s, y_s, _ = proj3d.proj_transform(xs3d, ys3d, zs3d, self.axes.M)
            self._path2d = Path(np.column_stack([x_s, y_s]))
            self.set_positions((x_s[0], y_s[0]), (x_s[1], y_s[1]))
            mpatches.FancyArrowPatch.draw(self, renderer)
    return Arrow3D


def _red(image):
    """How many pixels of *image* are red."""
    return int(((image[..., 0] > 180) & (image[..., 1] < 90) & (image[..., 2] < 90)).sum())


def _arrow():
    """qiskit's arrow in a 3-D Axes: what its ``draw`` reads, where it
    then stands, and whether its shaft and its head are drawn (a head
    makes the arrow more red pixels than the same arrow without one)."""
    def scene(style):
        made = build("arrow", make=lambda ax: ax.add_artist(_arrow_class()(
            [0.0, 2.5], [0.0, 2.5], [0.0, 3.0], mutation_scale=40, lw=3, arrowstyle=style,
            color="red")))
        return made, pixels(made.fig)

    def measure():
        with_head, headed = scene("-|>")
        _, bare = scene("-")
        arrow = with_head.obj
        return {
            "seen": arrow.seen[0] if arrow.seen else None, "draws": len(arrow.seen),
            "path2d": field(arrow, "_path2d"),
            "shaft_drawn": _red(bare) > 20,
            "head_drawn": _red(headed) > 1.2 * _red(bare) and _red(headed) > 20,
        }
    return Probe("3d/qiskit arrow", measure)


def rows(found):
    """The rows of the subjects *found* (`discover`'s, or the recorded
    ones)."""
    made = []
    for name, has in found.items():
        names = [*has["made"], *has["drawn"]]
        made += [_state(name, name_) for name_ in names]
        for changed, kind in ((True, "changed"), (False, "identity")):
            for path in has[kind]:
                member, suffix = _split(path)
                made.append(_in_place(name, member, suffix, member in has["drawn"], changed))
        made += [_assigned(name, name_, name_ in has["drawn"]) for name_ in has["assigned"]]
        made += [_setter(name, setter, names, False) for setter in has["setters"]]
        if has["projects"]:
            made.append(_projection(name, names))
    for class_name, maker in CLASSES.items():
        has = found.get(maker.__name__)
        if has is not None:
            made.append(_subclassed(class_name, maker.__name__, [*has["made"], *has["drawn"]]))
    if found:
        made.append(_arrow())
    return made
