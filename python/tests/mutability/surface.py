"""
The whole surface: every public getter and attribute of every subject
(`scenes.SUBJECTS`), and the private attributes libraries read, each
followed into the objects it hands out (the paths of a list, the vertices
of a path, the points of a box, the matrix of a transform), each of
those changed in place the way its type allows.

Which members a subject has is matplotlib's to say: `discover` lists them
where the reference is recorded, and `rows` makes the rows of the names
recorded, so that both libraries are asked the same.
"""

import inspect
import re
import warnings

import numpy as np

from pltrs import transforms
from pltrs.path import Path

from . import scenes
from .measure import Row, Scene

#: Private attributes holding state that libraries read (the census
#: `spec/diff/private_census.json`) or that are the plain attribute behind
#: a getter, swept where a subject has them.
PRIVATE = (
    "_A", "_cells", "_children", "_coordinates", "_edgecolor3d", "_edgecolors", "_extent",
    "_facecolor3d", "_facecolors", "_linewidths", "_locs", "_major_tick_kw", "_mtx", "_offsets",
    "_offsets3d", "_path", "_paths", "_points", "_posA_posB", "_segments3d", "_sizes",
    "_verts3d", "_x", "_xorig", "_xy", "_y", "_yorig",
)

#: Members left out: they answer with the subject's surroundings, which
#: other subjects sweep, or cost a layout each.
SKIPPED = frozenset({
    "axes", "figure", "get_figure()", "get_tightbbox()", "get_window_extent()", "get_renderer()",
    "canvas", "callbacks", "mouseover", "stale_callback", "get_default_bbox_extra_artists()",
})

#: What a row follows into: the types whose objects can change in place.
_FOLLOWED = (np.ndarray, Path, list, transforms.Bbox, transforms.Affine2D)


class Unchangeable(Exception):
    """An object the sweep has no change for (an empty array)."""


def change_array(array):
    """Change every value of *array* in place (the last one of integers)."""
    if array.size == 0:
        raise Unchangeable
    kind = array.dtype.kind
    if kind == "f":
        array *= 0.5
        array += 0.25
    elif kind in "iu":
        array.flat[-1] = array.flat[0] if array.flat[-1] != array.flat[0] else array.flat[0] + 1
    elif kind == "b":
        np.logical_not(array, out=array)
    else:
        raise Unchangeable


def change_matrix(matrix):
    """Halve what the matrix of an affine transform does along x."""
    matrix[0, 0] *= 0.5


def change_list(items):
    """Change the first item of a list of numbers; lengthen a list of
    names; shorten any other."""
    first = items[0]
    if isinstance(first, bool):
        items[0] = not first
    elif isinstance(first, (int, float, np.number)):
        items[0] = first * 0.5 + 0.25
    elif isinstance(first, str):
        items.append(first)
    else:
        items.pop()


def change_dict(mapping):
    """Drop the last entry of *mapping*."""
    mapping.pop(next(reversed(mapping)))


def _immutable(value):
    """Whether *value* is nothing a program could change in place."""
    return (value is None or callable(value) or inspect.ismodule(value)
            or isinstance(value, (bool, int, float, complex, str, bytes, np.generic, frozenset)))


def _inside(prefix, pick, value, depth):
    """The targets inside *value*, the object *pick* reaches."""
    for suffix, inner, mutate in targets(value, depth + 1):
        yield prefix + suffix, (lambda top, pick=pick, inner=inner: inner(pick(top))), mutate


def targets(value, depth=0):
    """
    The objects reached from *value* that the sweep measures: for each,
    the suffix naming it, the function reaching it from *value*, and the
    function changing it in place (None where only identity is measured).
    """
    def top(value):
        return value
    if _immutable(value):
        return
    if isinstance(value, np.ndarray):
        yield "", top, change_array if value.size else None
    elif isinstance(value, Path):
        yield "", top, None
        yield ".vertices", lambda path: path.vertices, change_array if value.vertices.size else None
        if value.codes is not None:
            yield ".codes", lambda path: path.codes, change_array
    elif isinstance(value, transforms.Bbox):
        yield "", top, None
        yield ".get_points()", lambda box: box.get_points(), change_array
    elif isinstance(value, transforms.Affine2D):
        yield "", top, None
        yield ".get_matrix()", lambda affine: affine.get_matrix(), change_matrix
    elif isinstance(value, list):
        yield "", top, change_list if value else None
        if value and depth < 2 and isinstance(value[0], _FOLLOWED):
            yield from _inside("[0]", lambda items: items[0], value[0], depth)
    elif isinstance(value, tuple):
        for index, item in enumerate(value[:4]):
            if depth < 2 and isinstance(item, _FOLLOWED):
                yield from _inside(f"[{index}]", lambda items, index=index: items[index],
                                   item, depth)
    elif isinstance(value, dict):
        yield "", top, change_dict if value else None
    else:
        yield "", top, None


def _takes_no_argument(function):
    """Whether *function* can be called without arguments."""
    try:
        parameters = inspect.signature(function).parameters.values()
    except (TypeError, ValueError):
        return False
    return all(parameter.default is not parameter.empty
               or parameter.kind in (parameter.VAR_POSITIONAL, parameter.VAR_KEYWORD)
               for parameter in parameters)


def _is_alias(function):
    """Whether *function* is one of matplotlib's aliases of a getter."""
    return (getattr(function, "__doc__", None) or "").startswith("Alias for")


def value_of(obj, member):
    """What *member* of *obj* gives: ``name()`` a call, ``name`` a read."""
    if member.endswith("()"):
        return getattr(obj, member[:-2])()
    return getattr(obj, member)


def _members(obj):
    """The members of *obj* the sweep reads: its public getters taking no
    argument, its public attributes and properties (not what its class
    shares among all its objects), and the private ones of `PRIVATE`."""
    for name in dir(obj):
        if name.startswith("_") and name not in PRIVATE:
            continue
        try:
            attribute = getattr(obj, name)
        except Exception:
            continue
        static = inspect.getattr_static(type(obj), name, None)
        if callable(attribute) and not isinstance(static, property):
            if (name.startswith("get_") and _takes_no_argument(attribute)
                    and not _is_alias(attribute)):
                yield name + "()"
        elif static is None or hasattr(static, "__get__"):
            yield name


def discover():
    """
    The rows of the surface, by subject: ``{subject: {"changed": [member +
    suffix], "identity": [...]}}``, the objects the sweep changes in place
    and those it only reads twice, found on the library at hand
    (matplotlib, when the reference is recorded).
    """
    found = {}
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        for name in scenes.SUBJECTS:
            fig, obj = scenes.build(name)
            changed, identity = [], []
            for member in _members(obj):
                if member in SKIPPED:
                    continue
                try:
                    value = value_of(obj, member)
                except Exception:
                    continue
                for suffix, _, mutate in targets(value):
                    (identity if mutate is None else changed).append(member + suffix)
            found[name] = {"changed": changed, "identity": identity}
            scenes.plt.close(fig)
    return found


_MEMBER = re.compile(r"([A-Za-z_][A-Za-z0-9_]*(?:\(\))?)(.*)")


def _change(target, suffix):
    """Change *target*, the object the recorded *suffix* reached, in
    place, the way its type allows."""
    if suffix.endswith(".get_matrix()"):
        return change_matrix(target)
    for found, _, mutate in targets(target):
        if found == "" and mutate is not None:
            return mutate(target)
    raise Unchangeable


def row(subject, path, changed=True, then=None):
    """The row of the recorded name *path* of *subject*, changing the
    object it reaches unless *changed* is false; *then*, a pair of a
    name and a function of the subject, is called after the change
    (`.measure.Row`), and the row is named after both."""
    member, suffix = _MEMBER.fullmatch(path).groups()

    def build():
        return Scene(*scenes.build(subject))

    def reach(scene):
        value = value_of(scene.obj, member)
        for found, pick, _ in targets(value):
            if found == suffix:
                return pick(value)
        raise LookupError(f"{member} hands out nothing at {suffix!r}")

    name = f"{subject}.{path}"
    if then is not None:
        called, call = then
        return Row(f"{name} then {called}", build, reach,
                   lambda target: _change(target, suffix), then=lambda scene: call(scene.obj))
    return Row(name, build, reach,
               (lambda target: _change(target, suffix)) if changed else None)


def rows(found):
    """The rows of the names *found* (`discover`'s, or the recorded ones)."""
    return [row(subject, path, changed)
            for subject, kinds in found.items()
            for changed, kind in ((True, "changed"), (False, "identity"))
            for path in kinds[kind]]
