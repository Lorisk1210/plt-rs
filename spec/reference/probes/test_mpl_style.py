"""Style sheets, ported from matplotlib's own ``tests/test_style.py``:
``style.use`` and ``style.context`` with library names, style files, URLs
and dicts, the style library and its user directories, the blacklisted
rcParams and ``pyplot.xkcd``.

Area ``mpl_style``. Every op starts from the reference rcParams
(``matplotlib.testing.setup()``) and restores them, and restores the style
library.

Shared encodings
----------------
``library``: ``{name: text}``, style files ``<name>.mplstyle`` written to a
temporary directory that is appended to ``style.USER_LIBRARY_PATHS``
before ``style.reload_library()`` (the upstream ``temp_style`` helper);
without it, the user library is empty. ``style``: a library name (str),
``{"dict": {key: value}}`` (an rcParams dict; JSON values as Python
values), ``{"path": text}`` (a style file ``style.mplstyle`` written to a
temporary directory, passed as a ``pathlib.Path``), ``{"url": text}``
(the same file as a ``file:`` URL), or a list of these. ``rc``: rcParams
set before the op (``rcParams.update``). ``params``: rcParams read back,
as canonical values (colours as RGBA lists, tuples as lists).

Ops
---
``style.available``  {names, library?} -> ``[name in style.available, name
    in style.library]`` per name.
``style.reload_library``  {library} -> the style files named by the
    warnings ``reload_library`` logs, one entry per warning (the basename
    of the file each message mentions).
``style.context``  {style, params, rc?, library?} -> ``{"failed": bool,
    "inside": [values] or null, "after": [values]}``: the params inside
    ``style.context(style)`` (``failed`` when entering it raises), then
    after it.
``style.context.equal``  {styles} -> whether ``rcParams`` is the same
    inside ``style.context(s)`` for every style ``s``.
``pyplot.xkcd``  {scoped} -> ``path.sketch`` before ``plt.xkcd()``, after
    it (inside its ``with`` block when ``scoped``), and at the end.
``style.STYLE_BLACKLIST``  {} -> the sorted rcParams styles ignore.

Cases
-----
Named after the upstream tests. ``test_use_url`` and ``test_single_path``
write their style files as described above; ``test_up_to_date_blacklist``
probes the blacklist (pltrs checks in a unit test that every entry but
``backend``, which pltrs' pyplot handles outside the rcParams, is a
parameter); ``test_invalid_rc_warning_includes_filename`` records which
file each warning names.

Left out: ``test_style_from_module`` (dotted names load styles from
Python packages; its relative path is covered by ``test_single_path``).
"""

import contextlib
import logging
import pathlib
import tempfile
import warnings

import matplotlib as mpl
import matplotlib.colors as mcolors
import matplotlib.pyplot as plt
import matplotlib.style
import numpy as np

from spec_support import ProbeSet

probes = ProbeSet("mpl_style")

PARAM = "image.cmap"
VALUE = "pink"
DUMMY = f"{PARAM}: {VALUE}"
COLOR_PARAMS = {"axes.facecolor"}


def _canon(key, value):
    if key in COLOR_PARAMS:
        return list(mcolors.to_rgba(value))
    if isinstance(value, tuple):
        return list(value)
    return value


def _read(params):
    return [_canon(key, mpl.rcParams[key]) for key in params]


@contextlib.contextmanager
def _user_library(library):
    """The upstream ``temp_style``: ``library`` as the user style directory."""
    saved = matplotlib.style.USER_LIBRARY_PATHS
    with tempfile.TemporaryDirectory() as tmpdir:
        for name, text in (library or {}).items():
            pathlib.Path(tmpdir, f"{name}.mplstyle").write_text(text, encoding="utf-8")
        matplotlib.style.USER_LIBRARY_PATHS = [tmpdir] if library else []
        try:
            matplotlib.style.reload_library()
            yield
        finally:
            matplotlib.style.USER_LIBRARY_PATHS = saved
            matplotlib.style.reload_library()


def _style(spec, tmpdir):
    if isinstance(spec, list):
        return [_style(item, tmpdir) for item in spec]
    if isinstance(spec, dict) and "dict" in spec:
        return spec["dict"]
    if isinstance(spec, dict):
        path = pathlib.Path(tmpdir, "style.mplstyle")
        path.write_text(spec.get("path", spec.get("url")), encoding="utf-8")
        return path if "path" in spec else "file:" + path.resolve().as_posix()
    return spec


def _available(names, library=None):
    with _user_library(library):
        return [[name in matplotlib.style.available, name in matplotlib.style.library]
                for name in names]


def _reload_library(library):
    records = []
    handler = logging.Handler()
    handler.emit = records.append
    logger = logging.getLogger("matplotlib")
    logger.addHandler(handler)
    try:
        with _user_library(library):
            pass
    finally:
        logger.removeHandler(handler)
    files = [f"{name}.mplstyle" for name in library]
    return [next((f for f in files if f in record.getMessage()), None) for record in records]


def _context(style, params, rc=None, library=None):
    with mpl.rc_context(), _user_library(library), tempfile.TemporaryDirectory() as tmpdir:
        mpl.rcParams.update(rc or {})
        inside = None
        try:
            with warnings.catch_warnings():
                # Blacklisted parameters are ignored with a warning.
                warnings.simplefilter("ignore", UserWarning)
                with matplotlib.style.context(_style(style, tmpdir)):
                    inside = _read(params)
        except (KeyError, OSError, ValueError):
            pass
        return {"failed": inside is None, "inside": inside, "after": _read(params)}


def context(style, params, **args):
    probes.probe("style.context", _context, style=style, params=params, **args)


def _equal(styles):
    snapshots = []
    for style in styles:
        with mpl.rc_context(), matplotlib.style.context(style):
            snapshots.append(mpl.rcParams.copy())
    return all(snapshot == snapshots[0] for snapshot in snapshots)


def _xkcd(scoped):
    with mpl.rc_context():
        before = _read(["path.sketch"])
        if scoped:
            with plt.xkcd():
                inside = _read(["path.sketch"])
        else:
            plt.xkcd()
            inside = _read(["path.sketch"])
            np.testing.break_cycles()
        return [before, inside, _read(["path.sketch"])]


def test_invalid_rc_warning_includes_filename():
    probes.probe("style.reload_library", _reload_library, library={"basename": "foo: bar"})
    probes.probe("style.reload_library", _reload_library,
                 library={"a_good": DUMMY, "b_bad": "lines.linewidth: wide\nimage.cmap: pink"})


def test_available():
    names = ["_classic_test_patch", "classic", "dummy", "_test_"]
    probes.probe("style.available", _available, names=names)
    probes.probe("style.available", _available, names=names,
                 library={"_test_": DUMMY, "dummy": DUMMY})


def test_use():
    context("test", [PARAM], rc={PARAM: "gray"}, library={"test": DUMMY})


def test_use_url():
    context({"url": "axes.facecolor: adeade"}, ["axes.facecolor"], library={"test": DUMMY})


def test_single_path():
    context({"path": f"{PARAM} : {VALUE}"}, [PARAM], rc={PARAM: "gray"})


def test_context():
    context("test", [PARAM], rc={PARAM: "gray"}, library={"test": DUMMY})
    # A user sheet adds to the bundled sheet of the same name.
    context("ggplot", [PARAM, "axes.facecolor"], library={"ggplot": DUMMY})


def test_context_with_dict():
    context({"dict": {PARAM: "blue"}}, [PARAM], rc={PARAM: "gray"})


def test_context_with_dict_after_namedstyle():
    context(["test", {"dict": {PARAM: "blue"}}], [PARAM], rc={PARAM: "gray"},
            library={"test": DUMMY})


def test_context_with_dict_before_namedstyle():
    context([{"dict": {PARAM: "blue"}}, "test"], [PARAM], rc={PARAM: "gray"},
            library={"test": DUMMY})


def test_context_with_union_of_dict_and_namedstyle():
    context(["test", {"dict": {"text.usetex": True}}], [PARAM, "text.usetex"],
            rc={PARAM: "gray", "text.usetex": False}, library={"test": DUMMY})


def test_context_with_badparam():
    context({"dict": {PARAM: "gray", "badparam": None}}, [PARAM], rc={PARAM: "blue"})
    # Unknown names, missing files and blacklisted parameters.
    context("no-such-style", [PARAM])
    context({"path": "timezone: Europe/Paris\nlines.linewidth: 3\nfoo: bar"},
            ["timezone", "lines.linewidth"])


def test_alias():
    for styles in (["mpl20", "default"], ["mpl15", "classic"], ["classic", "default"]):
        probes.probe("style.context.equal", _equal, styles=styles)


def test_xkcd_no_cm():
    probes.probe("pyplot.xkcd", _xkcd, scoped=False)


def test_xkcd_cm():
    probes.probe("pyplot.xkcd", _xkcd, scoped=True)


def test_up_to_date_blacklist():
    probes.probe("style.STYLE_BLACKLIST", lambda: sorted(matplotlib.style._STYLE_BLACKLIST))
