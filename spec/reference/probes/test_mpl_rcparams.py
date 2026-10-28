"""matplotlib's own tests of rcParams (``tests/test_rcparams.py``) that the
``rcsetup`` area does not already hold: ``rc_context`` with dictionaries
and rc files, ``rc_file``, ``rc``'s short names, the colours that rcParams
give legends, markers and titles, the validator cases of
``test_validator_valid``/``test_validator_invalid`` and friends on one
parameter per validator, and every default value.

Area ``mpl_rcparams``. Values are canonical as in the ``rcsetup`` area
(``test_rcsetup.py``); colours of artists are ``[r, g, b, a]``.

Ops
---
``rc.validate``  {key, value} -> the canonical value of parameter ``key``
    set to the rc-file text ``value``, or the error (the ``rcsetup``
    area's).
``matplotlib.rcParamsDefault``  {key} -> the canonical default of ``key``.
``matplotlib.rc_context``  {setup, params, rc?, file?} -> ``{"inside",
    "after"}``: the canonical values of ``params`` inside ``rc_context(rc=rc,
    fname=<file>)`` and after it, the parameters ``setup`` (rc-file texts)
    being set first; ``file`` is the text of the rc file.
``matplotlib.rc_file``  {setup, file, params} -> the canonical values of
    ``params`` after ``rc_file(<file>)``, ``setup`` being set first.
``matplotlib.rc``  {group, kwargs, key} -> the canonical value of ``key``
    after ``rc(group, **kwargs)``.
``legend.Legend.legendPatch.get_color``  {which, rc} -> the face or edge
    colour (``which`` is ``"face"`` or ``"edge"``) of the frame of
    ``ax.legend()`` for ``ax.plot(range(3), label='test')`` under ``rc``.
``lines.Line2D.get_markerfacecolor`` / ``lines.Line2D.get_markeredgecolor``
    {rc} -> the marker colour of a line made under ``rc``.
``text.Text.get_color``  {rc} -> the colour of ``ax.set_title("Title")``
    under ``rc``.

Cases
-----
Named after the upstream tests. The validator cases give each validator's
rc-file texts to one parameter it validates; Python values that are not
text (lists, tuples, numpy arrays, ``Cycler`` objects, slices, bytes) have
no rc-file form and are left out. ``hatch``, ``color`` lists and
``markevery`` are validated through ``axes.prop_cycle`` cyclers, the only
parameter taking them.

Already covered
---------------
By the ``rcsetup`` area: ``test_rcparams_update`` and ``test_rcparams_init``
(``figure.figsize`` with three values), ``test_animation_frame_formats``,
``test_keymaps``, the colour and colour-or-linecolor and histogram-bins
cases of ``test_validator_valid``/``test_validator_invalid``. By the
``mpl_style`` area: restoring the parameters after a failing context
(``test_rcparams_reset_after_fail``; its failure is probed here).

Left out
--------
``test_RcParams_class`` (Python ``repr``/``str``, and ``find_all``, which
takes a regular expression), the decorator form of ``rc_context`` in
``test_rcparams``, ``test_Bug_2543``'s copies and ``validate_bool(None)``
(Python object semantics; its ``svg.fonttype`` case is probed),
``test_Issue_1713`` (a mocked locale encoding), ``test_no_backend_reset_rccontext``
and the ``test_backend_fallback_*`` tests (backends, subprocesses and
environment variables), ``test_deprecation`` (deprecation machinery),
``test_all_params_defined_as_code`` and ``test_validators_defined_as_code``
(matplotlib's internal parameter tables; every default is probed).
"""

import pathlib
import tempfile

import matplotlib as mpl
import matplotlib.colors as mcolors
from matplotlib.figure import Figure

from probes.test_rcsetup import SKIP, _canon, _name, _validate
from spec_support import ProbeSet

probes = ProbeSet("mpl_rcparams")


def _canonical(key, value):
    return _canon(_name(mpl.rcParams.validate[key]), value)


def _current(params):
    return [_canonical(key, mpl.rcParams[key]) for key in params]


def _rc_path(directory, text):
    path = pathlib.Path(directory) / "test_rcparams.rc"
    path.write_text(text, encoding="utf-8")
    return path


def _rc_context(setup, params, rc=None, file=None):
    with mpl.rc_context(), tempfile.TemporaryDirectory() as directory:
        mpl.rcParams.update(setup)
        fname = None if file is None else _rc_path(directory, file)
        with mpl.rc_context(rc=rc, fname=fname):
            inside = _current(params)
        return {"inside": inside, "after": _current(params)}


def _rc_file(setup, file, params):
    with mpl.rc_context(), tempfile.TemporaryDirectory() as directory:
        mpl.rcParams.update(setup)
        mpl.rc_file(_rc_path(directory, file))
        return _current(params)


def _validate_all(key, values):
    for value in values:
        probes.probe("rc.validate", _validate, key=key, value=value)


SETUP = {"text.usetex": "False", "lines.linewidth": "22"}


def test_rcparams():
    usetex = ["text.usetex"]
    linewidth = ["lines.linewidth"]
    r = probes.probe("matplotlib.rc_context", _rc_context, setup=SETUP, params=usetex,
                     rc={"text.usetex": "True"})
    assert r == {"inside": [True], "after": [False]}
    r = probes.probe("matplotlib.rc_context", _rc_context, setup=SETUP, params=linewidth,
                     file="lines.linewidth: 33")
    assert r == {"inside": [33], "after": [22]}
    r = probes.probe("matplotlib.rc_context", _rc_context, setup=SETUP, params=linewidth,
                     file="lines.linewidth: 33", rc={"lines.linewidth": "44"})
    assert r == {"inside": [44], "after": [22]}
    # rc_file starts from the defaults: the other parameters set before go.
    r = probes.probe("matplotlib.rc_file", _rc_file, setup={**SETUP, "lines.color": "red"},
                     file="lines.linewidth: 33",
                     params=["lines.linewidth", "lines.color", "text.usetex"])
    assert r[0] == 33


def test_rcparams_reset_after_fail():
    r = probes.probe("matplotlib.rc_context", _rc_context, setup={"text.usetex": "False"},
                     params=["text.usetex"], rc={"text.usetex": "True", "test.blah": "True"})
    assert isinstance(r, KeyError)


def test_nargs_cycler():
    _validate_all("axes.prop_cycle", ["cycler(cycler('color', 'rgb'), 2, 3)"])


def test_Bug_2543():
    _validate_all("svg.fonttype", ["True"])


def _legend_color(which, rc):
    with mpl.rc_context(rc):
        ax = Figure().subplots()
        ax.plot(range(3), label='test')
        patch = ax.legend().legendPatch
        return mcolors.to_rgba(getattr(patch, f"get_{which}color")())


def test_legend_colors():
    for which, rc in [
            ("face", {"legend.facecolor": "r"}),
            ("face", {"legend.facecolor": "inherit", "axes.facecolor": "r"}),
            ("face", {"legend.facecolor": "g", "axes.facecolor": "r"}),
            ("edge", {"legend.edgecolor": "r"}),
            ("edge", {"legend.edgecolor": "inherit", "axes.edgecolor": "r"}),
            ("edge", {"legend.edgecolor": "g", "axes.facecolor": "r"})]:
        probes.probe("legend.Legend.legendPatch.get_color", _legend_color, which=which, rc=rc)


def _marker_color(which):
    def color(rc):
        with mpl.rc_context(rc):
            line = mpl.lines.Line2D([1, 2], [1, 2])
            return mcolors.to_rgba(getattr(line, f"get_marker{which}color")())

    return color


def test_mfc_rcparams():
    probes.probe("lines.Line2D.get_markerfacecolor", _marker_color("face"),
                 rc={"lines.markerfacecolor": "r"})


def test_mec_rcparams():
    probes.probe("lines.Line2D.get_markeredgecolor", _marker_color("edge"),
                 rc={"lines.markeredgecolor": "r"})


def _title_color(rc):
    with mpl.rc_context(rc):
        return mcolors.to_rgba(Figure().subplots().set_title("Title").get_color())


def test_axes_titlecolor_rcparams():
    probes.probe("text.Text.get_color", _title_color, rc={"axes.titlecolor": "r"})


def test_validator_valid_invalid():
    _validate_all("axes.grid", ["t", "y", "yes", "on", "true", "1", "f", "n", "no", "off",
                                "false", "0", "aardvark", "2", "-1"])
    _validate_all("font.serif", ["", "a,b", "aardvark", "aardvark, ", "aardvark, ,"])
    _validate_all("axes.formatter.limits", ["1, 2", "aardvark"])
    _validate_all("figure.figsize", ["1.5, 2.5", "aardvark", "(3.5, 42, 1)"])
    _validate_all("axes.prop_cycle", [
        'cycler("color", "rgb")',
        'cycler("color", "Dark2")',
        '(cycler("color", ["r", "g", "b"]) +\n                          cycler("mew", [2, 3, 5]))',
        "cycler(c='rgb', lw=[1, 2, 3])",
        "cycler('c', 'rgb') * cycler('linestyle', ['-', '--'])",
        "2 * cycler('color', 'rgb')",
        "2 * cycler('color', 'r' + 'gb')",
        "cycler(c='r' + 'gb', lw=[1, 2, 3])",
        "cycler('color', 'rgb') * 2",
        "concat(cycler('color', 'rgb'), cycler('color', 'cmk'))",
        "cycler('color', 'rgbcmk')[:3]",
        "cycler('color', 'rgb')[::-1]",
        'cycler("bleh, [])',
        "cycler('color', 'rgb') * * cycler('color', 'rgb')",
        'Cycler("linewidth", [1, 2, 3])',
        "cycler('c', [j.__class__(j) for j in ['r', 'b']])",
        "cycler('c', [j. __class__(j) for j in ['r', 'b']])",
        "cycler('c', [j.\t__class__(j) for j in ['r', 'b']])",
        "cycler('c', [j.\u000c__class__(j) for j in ['r', 'b']])",
        "cycler('c', [j.__class__(j).lower() for j in ['r', 'b']])",
        "cycler('color', [x for x in ['r', 'g', 'b']])",
        '1 + 2',
        'os.system("echo Gotcha")',
        'import os',
        'def badjuju(a): return a; badjuju(cycler("color", "rgb"))',
        'cycler("waka", [1, 2, 3])',
        'cycler(c=[1, 2, 3])',
        "cycler(lw=['a', 'b', 'c'])",
        # validate_hatch, through the cycler's hatch property.
        "cycler('hatch', ['--|'])",
        "cycler('hatch', ['\\\\oO'])",
        "cycler('hatch', ['/+*/.x'])",
        "cycler('hatch', [''])",
        "cycler('hatch', ['--_'])",
        "cycler('hatch', [8])",
        "cycler('hatch', ['X'])",
        # validate_colorlist, through the cycler's color property.
        "cycler('color', 'r,g,b')",
        "cycler('color', ['r', 'g', 'b'])",
        "cycler('color', 'r, ,')",
        "cycler('color', ['', 'g', 'blue'])",
        "cycler('color', [(1, 0, 0), (0, 1, 0)])",
        "cycler('color', 'fish')",
        # validate_markevery, through the cycler's markevery property.
        "cycler('markevery', [None])",
        "cycler('markevery', [1])",
        "cycler('markevery', [0.1])",
        "cycler('markevery', [(1, 1)])",
        "cycler('markevery', [(0.1, 0.1)])",
        "cycler('markevery', [[1, 2, 3]])",
        "cycler('markevery', [(1, 2, 3)])",
        "cycler('markevery', [[1, 2, 0.3]])",
        "cycler('markevery', [['a', 2, 3]])",
        "cycler('markevery', [[1, 2, 'a']])",
        "cycler('markevery', [(0.1, 0.2, 0.3)])",
        "cycler('markevery', [(0.1, 2, 3)])",
        "cycler('markevery', [(1, 0.2, 0.3)])",
        "cycler('markevery', [(1, 0.1)])",
        "cycler('markevery', [(0.1, 1)])",
        "cycler('markevery', ['abc'])",
        "cycler('markevery', [(1, 'a')])",
        "cycler('markevery', [(0.1, 'b')])",
        "cycler('markevery', [('a', 1)])",
        "cycler('markevery', [('a', 0.1)])",
        "cycler('markevery', ['a'])",
    ])
    _validate_all("lines.linestyle", [
        "-", "solid", "--", "dashed", "-.", "dashdot", ":", "dotted", "", " ", "None",
        "none", "DoTtEd", "1, 3", "[1.23, 456]", "[1, 2, 3, 4]", "(0, [1, 2])", "(-1, [1, 2])",
        "aardvark", "[1, 2, 3]", "1.23", '("a", [1, 2])', "(None, [1, 2])", "(1, [1, 2, 3])",
        "([1, 2], 1)"])


def test_validate_cycler_bad_color_string():
    _validate_all("axes.prop_cycle", ["cycler('color', 'foo')"])


def test_validate_fontweight():
    _validate_all("font.weight", ["bold", "BOLD", "100", "20.6"])


def test_validate_fontstretch():
    _validate_all("font.stretch", ["expanded", "EXPANDED", "100", "20.6"])


def test_rcparams_legend_loc():
    _validate_all("legend.loc", ["best", "1", "(0.9, .7)", "(-0.9, .7)"])


def test_rcparams_legend_loc_from_file():
    for value in ["best", "1", "(0.9, 0.7)", "(-0.9, 0.7)"]:
        probes.probe("matplotlib.rc_context", _rc_context, setup={}, params=["legend.loc"],
                     file=f"legend.loc: {value}")


def test_validate_sketch():
    _validate_all("path.sketch", ["1, 2, 3", "(1, 2, 3)"])


def test_validate_sketch_error():
    _validate_all("path.sketch", ["1", "1 2 3"])


def test_rcparams_path_sketch_from_file():
    for value in ["1, 2, 3", "(1,2,3)"]:
        r = probes.probe("matplotlib.rc_context", _rc_context, setup={}, params=["path.sketch"],
                         file=f"path.sketch: {value}")
        assert r["inside"] == [[1, 2, 3]]


def _rc(group, kwargs, key):
    with mpl.rc_context():
        mpl.rc(group, **kwargs)
        return _canonical(key, mpl.rcParams[key])


def test_rc_aliases():
    for group, option, alias, value in [
            ('lines', 'linewidth', 'lw', 3),
            ('lines', 'linestyle', 'ls', 'dashed'),
            ('lines', 'color', 'c', 'white'),
            ('axes', 'facecolor', 'fc', 'black'),
            ('figure', 'edgecolor', 'ec', 'magenta'),
            ('lines', 'markeredgewidth', 'mew', 1.5),
            ('patch', 'antialiased', 'aa', False),
            ('font', 'sans-serif', 'sans', ["Verdana"])]:
        probes.probe("matplotlib.rc", _rc, group=group, kwargs={alias: value},
                     key=f"{group}.{option}")


def _default(key):
    return _canonical(key, mpl.rcParamsDefault[key])


def test_defaults_as_code():
    for key in sorted(mpl.rcParamsDefault):
        if key not in SKIP:
            probes.probe("matplotlib.rcParamsDefault", _default, key=key)
