"""
A row of the sweep and what is measured of it.

A row builds a scene, reaches one object in it (`Row.hand`), changes that
object in place (`Row.mutate`) and reads the artist again (`Row.read`).
`outcome` builds the scene three times, so that each question is asked of
a scene nothing else has touched: is the object the same at every read,
does a read, a draw and the data limits show the change; does a measure
show it; does a pickle carry it.
"""

import pickle
import warnings

import numpy as np

import pltrs.pyplot as plt
from pltrs import transforms
from pltrs.artist import Artist
from pltrs.path import Path


class Scene:
    """A figure, the subject built in it, and what a row keeps beside
    them (the object it gave the subject, say)."""

    def __init__(self, fig, obj, **kept):
        self.fig = fig
        self.obj = obj
        self.__dict__.update(kept)


class Row:
    """
    One object reached from one artist, and how it is changed in place.

    *build* makes a new `Scene`; *hand* gives the object from the scene,
    *mutate* changes it in place (None: only its identity is measured),
    *read* gives what the artist then answers (default: the object
    handed), and *same* says whether two reads give one object (default:
    two calls of *hand* do).
    """

    def __init__(self, name, build, hand, mutate=None, read=None, same=None):
        self.name = name
        self.build = build
        self.hand = hand
        self.mutate = mutate
        self.read = hand if read is None else read
        self.same = (lambda scene: hand(scene) is hand(scene)) if same is None else same


def number(value):
    """*value* to six significant digits, or the name of what is no
    finite number."""
    value = float(value)
    if value != value:
        return "nan"
    if value in (float("inf"), float("-inf")):
        return "inf" if value > 0 else "-inf"
    return float(f"{value:.6g}")


def _plain_array(array):
    """The shape of *array*, its first values and their sum."""
    if array.dtype.kind not in "biuf":
        return {"shape": list(array.shape), "head": [str(item) for item in array.ravel()[:6]]}
    flat = np.ma.filled(np.ma.asarray(array).astype(float), np.nan).ravel()
    finite = flat[np.isfinite(flat)]
    return {"shape": list(array.shape), "head": [number(item) for item in flat[:6]],
            "sum": number(finite.sum())}


def _key(key):
    """A dictionary key as text: a class by its name, so that it reads
    the same whatever the package is called."""
    return key.__name__ if isinstance(key, type) else str(key)


def plain(value, depth=0):
    """*value* as JSON says it: enough of it to see a change in place."""
    if depth > 3:
        return type(value).__name__
    if isinstance(value, np.ndarray):
        return _plain_array(value)
    if isinstance(value, Path):
        return {"vertices": plain(value.vertices), "codes": plain(value.codes)}
    if isinstance(value, transforms.BboxBase):
        return {"points": plain(value.get_points())}
    if isinstance(value, transforms.Transform):
        return {"matrix": plain(value.get_matrix())} if value.is_affine else type(value).__name__
    if isinstance(value, (list, tuple)):
        return {"len": len(value), "items": [plain(item, depth + 1) for item in value[:4]]}
    if isinstance(value, dict):
        return {"len": len(value), "keys": sorted(_key(key) for key in value)[:6]}
    if isinstance(value, (bool, str, type(None))):
        return value
    if isinstance(value, (int, float, np.number)):
        return number(value)
    return type(value).__name__


def pixels(fig):
    """The pixels of *fig*, drawn now."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def _limits(scene):
    """The data limits of every Axes of the scene."""
    return [plain(ax.dataLim.get_points()) for ax in scene.fig.axes]


def _boxes(scene):
    """What measuring the scene gives: the figure's tight box, and the
    subject's own extent if it is an artist."""
    boxes = [plain(scene.fig.get_tightbbox().get_points())]
    if isinstance(scene.obj, Artist):
        boxes.append(plain(scene.obj.get_window_extent().get_points()))
    return boxes


def _step(result, key, function):
    """Run *function* and keep its answer as *key* of *result*, or the
    name of the exception it raised; return whether it ran through."""
    try:
        result[key] = function()
    except Exception as err:
        result[key] = type(err).__name__
        return False
    return True


def _drawn(row, result):
    """Reads, the draw and the data limits of a scene changed in place."""
    scene = row.build()
    values = {}
    if not (_step(values, "before", lambda: plain(row.read(scene)))
            and _step(result, "same", lambda: row.same(scene))):
        result["read"] = values.get("before", result.get("same"))
        return
    if row.mutate is None:
        return
    pixels(scene.fig)
    before, limits = pixels(scene.fig), _limits(scene)
    target = row.hand(scene)
    if not _step(result, "mutate", lambda: row.mutate(target)):
        return
    del result["mutate"]
    if _step(values, "after", lambda: plain(row.read(scene))):
        result["reread"] = values["after"] != values["before"]
        result["value"] = values["after"]
    else:
        result["reread"] = values["after"]
    _step(result, "datalim", lambda: _limits(scene) != limits)
    _step(result, "draw", lambda: bool((pixels(scene.fig) != before).any()))
    _step(result, "kept", lambda: row.hand(scene) is target)

    _step(result, "relim", lambda: _relimited(scene) != _relimited(_untouched(row)))


def _untouched(row):
    """A scene of *row* drawn as often as the one changed, unchanged."""
    scene = row.build()
    for _ in range(3):
        pixels(scene.fig)
    return scene


def _relimited(scene):
    """The data limits of the scene once every Axes recomputed them."""
    for ax in scene.fig.axes:
        ax.relim()
    return _limits(scene)


def _measured(row, result):
    """Whether a measure follows a change in place, no draw between."""
    scene = row.build()
    pixels(scene.fig)

    def measured():
        boxes = _boxes(scene)
        row.mutate(row.hand(scene))
        return _boxes(scene) != boxes
    _step(result, "measure", measured)


def _pickled(row, result):
    """Whether a pickle carries a change in place."""
    scene = row.build()

    def pickled():
        row.mutate(row.hand(scene))
        value = plain(row.read(scene))
        return plain(row.read(pickle.loads(pickle.dumps(scene)))) == value
    _step(result, "pickle", pickled)


def outcome(row):
    """
    What *row* does: ``same`` (two reads give one object), and for a row
    changing its object in place ``reread`` (a read shows the change, and
    ``value`` what it reads), ``datalim`` (the data limits moved at
    once), ``draw`` (the next draw differs), ``kept`` (the object is
    still the one handed out after that draw), ``relim`` (the data limits
    recomputed are not those of the scene unchanged), ``measure`` (a measure with no draw between
    differs) and ``pickle`` (the unpickled artist reads the same). A step
    that raises gives the name of its exception instead.
    """
    result = {}
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        try:
            _drawn(row, result)
            if row.mutate is not None and "mutate" not in result and "read" not in result:
                _measured(row, result)
                _pickled(row, result)
        finally:
            plt.close("all")
    return result
