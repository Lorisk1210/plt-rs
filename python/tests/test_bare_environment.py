"""
pltrs in an environment holding only what it requires: a contour set
refuses the chunk sizes contourpy refuses whether or not contourpy (a
requirement of matplotlib's that pltrs replaces) is installed, with the
errors recorded from matplotlib 3.11.2 and contourpy 1.4.0; and the rules
by which ``check_dist.py --install`` compares pltrs alone with matplotlib
alone (``bare_imports.py``).
"""

import pathlib
import sys
import warnings

import numpy as np
import pytest

import pltrs.pyplot as plt

TOOLS = pathlib.Path(__file__).resolve().parents[1] / "tools"
sys.path.insert(0, str(TOOLS))

import bare_imports  # noqa: E402
import build_facts  # noqa: E402
import check_dist  # noqa: E402

Z = np.arange(12.).reshape(3, 4)

#: The parameters of contourpy 1.4.0's generator constructors pybind11
#: lists, up to the keywords of each.
ARRAYS = ("x: typing.Annotated[numpy.typing.ArrayLike, numpy.float64], "
          "y: typing.Annotated[numpy.typing.ArrayLike, numpy.float64], "
          "z: typing.Annotated[numpy.typing.ArrayLike, numpy.float64], "
          "mask: typing.Annotated[numpy.typing.ArrayLike, numpy.bool], *, ")
SIZES = ("x_chunk_size: typing.SupportsInt | typing.SupportsIndex = 0, "
         "y_chunk_size: typing.SupportsInt | typing.SupportsIndex = 0")
GRID = ("array([[0., 1., 2., 3.],\n       [0., 1., 2., 3.],\n       [0., 1., 2., 3.]]), "
        "array([[0., 0., 0., 0.],\n       [1., 1., 1., 1.],\n       [2., 2., 2., 2.]]), "
        "array([[ 0.,  1.,  2.,  3.],\n       [ 4.,  5.,  6.,  7.],\n       [ 8.,  9., 10., 11.]])")
REFUSED = ("__init__(): incompatible constructor arguments. The following argument types are "
           "supported:\n    1. contourpy._contourpy.")


@pytest.fixture(autouse=True)
def closed():
    """Each test's figures closed after it."""
    yield
    plt.close("all")


@pytest.fixture(params=["contourpy blocked", "contourpy as installed"])
def contourpy_or_not(request, monkeypatch):
    """Each test twice: with contourpy unimportable, and as installed."""
    if request.param == "contourpy blocked":
        monkeypatch.setitem(sys.modules, "contourpy", None)


@pytest.mark.parametrize("nchunk, error, message", [
    (-1, ValueError, "chunk_size cannot be negative"),
    (-2.5, ValueError, "chunk_size cannot be negative"),
    (np.int64(-3), ValueError, "chunk_size cannot be negative"),
    ((2, -1), ValueError, "chunk_size cannot be negative"),
    ("a", TypeError, "'<' not supported between instances of 'str' and 'int'"),
    ([1], TypeError, "'<' not supported between instances of 'list' and 'int'"),
])
def test_contour_refuses_chunk_sizes_as_contourpy(contourpy_or_not, nchunk, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{message}$"):
        ax.contour(Z, nchunk=nchunk)


@pytest.mark.parametrize("nchunk", [None, True, (1, 2), np.float32(2.5), 2 ** 63 - 1])
def test_contour_takes_the_chunk_sizes_contourpy_takes(contourpy_or_not, nchunk):
    fig, ax = plt.subplots()
    assert len(ax.contour(Z, nchunk=nchunk).levels)


@pytest.mark.parametrize("algorithm, nchunk, signature, keywords", [
    (None, 0.5, "Mpl2014ContourGenerator(" + ARRAYS + "corner_mask: bool, " + SIZES,
     "x_chunk_size=0.5, y_chunk_size=0.5, corner_mask=True"),
    ("mpl2005", 2.5, "Mpl2005ContourGenerator(" + ARRAYS + SIZES,
     "x_chunk_size=2.5, y_chunk_size=2.5"),
    ("serial", 1e9, "SerialContourGenerator(" + ARRAYS + "corner_mask: bool, "
     "line_type: contourpy._contourpy.LineType, fill_type: contourpy._contourpy.FillType, "
     "quad_as_tri: bool, z_interp: contourpy._contourpy.ZInterp, " + SIZES,
     "x_chunk_size=1000000000.0, y_chunk_size=1000000000.0, "
     "line_type=<LineType.SeparateCode: 102>, fill_type=<FillType.OuterCode: 201>, "
     "corner_mask=True, quad_as_tri=False, z_interp=<ZInterp.Linear: 1>"),
    ("threaded", float("nan"), "ThreadedContourGenerator(" + ARRAYS + "corner_mask: bool, "
     "line_type: contourpy._contourpy.LineType, fill_type: contourpy._contourpy.FillType, "
     "quad_as_tri: bool, z_interp: contourpy._contourpy.ZInterp, " + SIZES
     + ", thread_count: typing.SupportsInt | typing.SupportsIndex = 0",
     "x_chunk_size=nan, y_chunk_size=nan, line_type=<LineType.SeparateCode: 102>, "
     "fill_type=<FillType.OuterCode: 201>, corner_mask=True, quad_as_tri=False, "
     "z_interp=<ZInterp.Linear: 1>, thread_count=0"),
])
def test_contour_refuses_sizes_no_integer_as_contourpy_s_constructors(
        contourpy_or_not, algorithm, nchunk, signature, keywords):
    fig, ax = plt.subplots()
    with pytest.raises(TypeError) as caught:
        ax.contour(Z, nchunk=nchunk, algorithm=algorithm)
    assert str(caught.value) == (
        f"{REFUSED}{signature})\n\nInvoked with: {GRID}, None; kwargs: {keywords}")


def test_the_refusal_shows_the_values_under_the_mask(contourpy_or_not):
    z = np.arange(6.).reshape(2, 3)
    z[0, 1] = np.nan
    fig, ax = plt.subplots()
    with pytest.raises(TypeError) as caught:
        ax.contourf([0, 1, 3], [0, 2], np.ma.masked_greater(z, 4), nchunk=(1, 0.5))
    assert str(caught.value) == (
        f"{REFUSED}Mpl2014ContourGenerator({ARRAYS}corner_mask: bool, {SIZES})\n\n"
        "Invoked with: array([[0., 1., 3.],\n       [0., 1., 3.]]), "
        "array([[0., 0., 0.],\n       [2., 2., 2.]]), "
        "array([[ 0., nan,  2.],\n       [ 3.,  4.,  5.]]), "
        "array([[False,  True, False],\n       [False, False,  True]]); "
        "kwargs: x_chunk_size=0.5, y_chunk_size=1, corner_mask=True")


# -- numpy's matrix products ---------------------------------------------------

def _dot_warnings(fig, ax):
    """The warnings of composing *ax*'s data transform, its y view without
    width (a product of matrices holding an infinity and a zero)."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        ax.transData.get_affine()
    return [str(w.message) for w in caught]


def test_products_warn_as_those_of_the_numpy_pltrs_runs_with():
    from pltrs import _api
    from pltrs._native import warnings as native
    dot, matmul = _api._numpy_products_warn()
    assert {"dot": dot, "matmul": matmul} == build_facts.PRODUCTS_WARN
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.yaxis.set_view_interval(0, 0, ignore=True)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig.canvas.draw()
    assert _dot_warnings(fig, ax) == ["invalid value encountered in dot"] * 2 * dot
    try:
        native.set_products_warn(not dot, matmul)
        assert _dot_warnings(fig, ax) == ["invalid value encountered in dot"] * 2 * (not dot)
    finally:
        native.set_products_warn(dot, matmul)


# -- python-dateutil's zones ---------------------------------------------------

def test_dateutil_zones_rounding_their_offsets_are_asked_through_python(monkeypatch):
    # python-dateutil 2.7 rounds the offsets of its zone files to whole
    # minutes, which pltrs (Rust)'s model of its zones does not: such zones
    # are asked through Python, as matplotlib asks them.
    import datetime

    import dateutil.tz

    from pltrs import _date_locators, dates
    berlin = dateutil.tz.gettz("Europe/Berlin")
    if type(berlin).__module__ != "dateutil.tz.tz":
        pytest.skip("python-dateutil's zones are asked through Python here")
    when = datetime.datetime(1800, 1, 1, tzinfo=berlin)
    expected = ((when.astimezone(datetime.timezone.utc).replace(tzinfo=None)
                 - datetime.datetime(1970, 1, 1)) / datetime.timedelta(days=1))
    for keeps in (True, False):
        monkeypatch.setattr(_date_locators, "_dateutil_keeps_offsets", lambda keeps=keeps: keeps)
        native = _date_locators._native_tz(berlin)
        assert isinstance(native, str) == keeps
        assert dates.date2num(when) == expected


# -- check_dist.py's comparison of pltrs alone with matplotlib alone ----------

def _tables(modules=None, backends=None, imports=None, required=("numpy",)):
    return {"modules": modules or {}, "backends": backends or {}, "imports": imports or {},
            "required": list(required)}


MATPLOTLIB = _tables(
    modules={"matplotlib.pyplot": None,
             "matplotlib.backends.backend_qtagg": ["ImportError", "Failed to import any"],
             "matplotlib.backends.backend_webagg": ["RuntimeError", "requires Tornado."]},
    backends={"agg": None, "cairo": ["ImportError", "cairo backend requires pycairo"]},
    imports={"numpy": "matplotlib", "packaging": "matplotlib", "tornado": "matplotlib.x",
             "IPython": "matplotlib.y"},
    required=("numpy", "packaging"))


def test_pltrs_alone_as_matplotlib_alone_shows_nothing():
    ours = _tables(
        modules={"matplotlib.pyplot": None,
                 "matplotlib.backends.backend_qtagg": ["ImportError", "No binding"],
                 # Better than matplotlib: serving without Tornado.
                 "matplotlib.backends.backend_webagg": None,
                 # pltrs' own module, wanting an optional package.
                 "matplotlib.backends.backend_inline": ["ModuleNotFoundError", "", "IPython"]},
        backends={"agg": None, "cairo": ["ImportError", "pycairo or cairocffi"]},
        imports={"numpy": "pltrs", "tornado": "pltrs.x", "contourpy": "pltrs.contour"})
    assert check_dist.bare_problems(ours, MATPLOTLIB) == []


def test_pltrs_alone_shows_what_matplotlib_alone_does_not():
    ours = _tables(
        modules={"matplotlib.pyplot": ["ModuleNotFoundError", "No module named 'packaging'",
                                       "packaging"],
                 "matplotlib.backends.backend_qtagg": ["AttributeError", "x"],
                 "matplotlib.backends._own": ["KeyError", "y"],
                 "matplotlib.backends._optional": ["ModuleNotFoundError", "", "zope"]},
        backends={"agg": ["RuntimeError", "drawn badly"]},
        imports={"numpy": "pltrs", "packaging": "pltrs._executables", "zope": "pltrs.z"})
    assert check_dist.bare_problems(ours, MATPLOTLIB) == [
        "matplotlib.backends._optional: ModuleNotFoundError:  (under matplotlib: not "
        "matplotlib's)",
        "matplotlib.backends._own: KeyError: y (under matplotlib: not matplotlib's)",
        "matplotlib.backends.backend_qtagg: AttributeError: x (under matplotlib: ImportError)",
        "matplotlib.pyplot: ModuleNotFoundError: No module named 'packaging' "
        "(under matplotlib: fine)",
        "agg: RuntimeError: drawn badly (under matplotlib: fine)",
        "pltrs._executables imports packaging, which is no requirement of pltrs nor "
        "optional to matplotlib",
        "pltrs.z imports zope, which is no requirement of pltrs nor optional to matplotlib"]


def test_modules_of_another_system_may_fail_to_import():
    name, system = next(iter(check_dist.SYSTEM_MODULES.items()))
    ours = _tables(modules={name: ["AttributeError", "module 'ctypes' has no attribute"]})
    assert bool(check_dist.bare_problems(ours, MATPLOTLIB)) == (sys.platform == system)


def test_the_imports_of_a_package_include_those_inside_functions(tmp_path):
    package = tmp_path / "pkg"
    (package / "sub").mkdir(parents=True)
    (package / "system_only").mkdir()
    (package / "__init__.py").write_text(
        "import os, numpy.linalg\nfrom pltrs import pyplot\nfrom . import sub\n"
        "def later():\n    from packaging.version import parse\n    import zope.interface\n")
    (package / "sub" / "__init__.py").write_text("")
    (package / "sub" / "leaf.py").write_text("import tornado.web\n")
    (package / "system_only" / "_mod.py").write_text("")
    assert bare_imports.imported_packages(package / "__init__.py") == {
        "numpy", "packaging", "zope"}
    sys.path.insert(0, str(tmp_path))
    try:
        assert sorted(bare_imports.module_files("pkg")) == ["pkg", "pkg.sub", "pkg.sub.leaf"]
    finally:
        sys.path.remove(str(tmp_path))


@pytest.mark.parametrize("name, distribution, expected", [
    ("pltrs", "pltrs", "matplotlib"),
    ("pltrs.backends.backend_agg", "pltrs", "matplotlib.backends.backend_agg"),
    ("pltrs.mpl_toolkits.mplot3d.axes3d", "pltrs", "mpl_toolkits.mplot3d.axes3d"),
    ("pltrs.pylab", "pltrs", "pylab"),
    ("mpl_toolkits.axes_grid1", "matplotlib", "mpl_toolkits.axes_grid1"),
])
def test_modules_go_by_matplotlib_s_names(name, distribution, expected):
    assert bare_imports.matplotlib_name(name, distribution) == expected
