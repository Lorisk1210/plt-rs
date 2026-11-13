"""
pltrs (Python) is typed as matplotlib is (task TYPES1): it ships
matplotlib's ``py.typed`` and type stubs beside the modules they describe
(``python/tools/matplotlib_stubs.py`` writes them, importing pltrs'
modules where matplotlib's import matplotlib's), so that a type checker
reads pltrs as it reads matplotlib; and the names its stubs describe are
what pltrs has at run time where matplotlib's are (``cargo xtask diff
--types``). The package checked is the one imported, so that an installed
wheel is checked too (``check_dist.py --install``).
"""

import ast
import copy
import importlib.util
import pathlib
import subprocess
import sys

import pytest

import pltrs
import pltrs.pyplot as plt

PACKAGE = pathlib.Path(pltrs.__file__).parent

#: The stubs matplotlib 3.11.2 installs whose modules pltrs does not have:
#: its C++ ``_qhull`` and the macOS backend's ``_macosx``.
NOT_IN_PLTRS = ("_qhull.pyi", "backends/_macosx.pyi")


def stubs():
    """The stubs of matplotlib's that pltrs ships, below the package (not
    the untyped ones of its toolkits, `test_types2.py`'s)."""
    return sorted(path.relative_to(PACKAGE) for path in PACKAGE.rglob("*.pyi")
                  if path.relative_to(PACKAGE).parts[0] != "mpl_toolkits")


def test_the_package_is_typed():
    assert (PACKAGE / "py.typed").is_file()
    # matplotlib's 85 installed stubs but the two above, and pyplot's,
    # which matplotlib types inline.
    assert len(stubs()) == 85 - len(NOT_IN_PLTRS) + 1
    for name in ("__init__.pyi", "pyplot.pyi", "figure.pyi", "axes/_axes.pyi", "cm.pyi"):
        assert pathlib.Path(name) in stubs()
    for name in NOT_IN_PLTRS:
        assert pathlib.Path(name) not in stubs()


@pytest.mark.parametrize("stub", stubs(), ids=str)
def test_every_stub_has_its_module_and_imports_pltrs(stub):
    module = PACKAGE / stub.with_suffix(".py")
    assert module.is_file() or (PACKAGE / stub.with_suffix("") / "__init__.py").is_file()
    tree = ast.parse((PACKAGE / stub).read_text(encoding="utf-8"))
    # matplotlib's imports (absolute, or relative where its stub has them),
    # of pltrs' modules.
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.level == 0:
            assert node.module.split(".")[0] != "matplotlib", stub
        if isinstance(node, ast.Import):
            assert not [a for a in node.names if a.name.split(".")[0] == "matplotlib"], stub


def test_stubs_are_not_imported_at_run_time():
    # A stub is never what Python loads.
    assert not [name for name, module in sys.modules.items()
                if name.startswith("pltrs") and str(getattr(module, "__file__", "")).endswith(".pyi")]


def test_names_are_matplotlibs_where_the_stubs_describe_them():
    # Recorded from matplotlib 3.11.2.
    assert pltrs.__all__[:3] == ["__bibtex__", "__version__", "__version_info__"]
    assert len(pltrs.__all__) == 30 and "color_sequences" in pltrs.__all__
    for module in ("axes", "axis", "contour", "layout_engine"):
        assert not hasattr(importlib.import_module(f"pltrs.{module}"), "__all__")
    from pltrs import collections, figure, patches, projections
    assert not hasattr(figure.SubFigure, "get_subplotspec")
    assert not hasattr(collections.Collection, "pickradius")
    assert not hasattr(patches.ArrowStyle.Simple, "arrow")
    assert patches.ArrowStyle.CurveA.arrow == "<-"
    assert not hasattr(projections.geo.AitoffAxes.AitoffTransform, "projection")
    assert "set_ec" not in vars(patches.Circle)
    fig, ax = plt.subplots()
    tick = ax.xaxis.get_major_ticks()[0]
    assert not hasattr(tick, "axis")
    assert not hasattr(type(ax.xaxis).__mro__[1], "get_ticks_position")
    assert ax.xaxis.get_ticks_position() == "bottom"
    plt.close(fig)


def test_packer_settings_are_properties():
    from pltrs.offsetbox import DrawingArea, PackerBase, TextArea, VPacker
    assert isinstance(vars(PackerBase)["sep"], property)
    packer = VPacker(children=[TextArea("a")], pad=2, sep=3)
    packer.sep = 5
    area = DrawingArea(10, 20)
    area.width = 30
    assert (packer.pad, packer.sep, area.width, area.height) == (2, 5, 30, 20)


def test_an_image_is_an_array_through_the_buffer_protocol():
    np = pytest.importorskip("numpy")
    from pltrs.ft2font import FT2Image
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        image = FT2Image(3, 2)
    image.draw_rect_filled(0, 0, 1, 1)
    assert not hasattr(FT2Image, "__array__")
    array = np.asarray(image)
    # Recorded from matplotlib 3.11.2.
    assert (array.shape, array.dtype, int(array.sum())) == ((2, 3), np.uint8, 1020)


def test_rcparams_copy_as_matplotlibs():
    copied = copy.copy(pltrs.rcParams)
    assert type(copied) is pltrs.RcParams and copied == pltrs.rcParams


#: Programs type-checked under pltrs, and what mypy says of each under
#: matplotlib 3.11.2 (recorded with the import replaced), but the
#: package's name.
CLEAN = """
import numpy as np
import pltrs.pyplot as plt
from pltrs.figure import Figure

fig, ax = plt.subplots(figsize=(4, 3), layout="constrained")
x = np.linspace(0, 1, 10)
(line,) = ax.plot(x, x ** 2, color="C1", label="square")
ax.legend()
ax.set_xlim(0, 1)
other: Figure = plt.figure()
norm: plt.Normalize = plt.Normalize(0, 1)
fig.savefig("out.png")
"""
WRONG = """
import numpy as np
import pltrs.pyplot as plt

fig, ax = plt.subplots()
reveal_type(fig)
reveal_type(ax)
(line,) = ax.plot(np.arange(3), [1, 2, 3])
line.set_color(3.5j)
ax.set_xlim("a", None, foo=1)
x: int = plt.gca()
reveal_type(plt.colormaps)
"""
WRONG_DIAGNOSTICS = """\
wrong.py:6: note: Revealed type is "pltrs.figure.Figure"
wrong.py:7: note: Revealed type is "pltrs.axes._axes.Axes"
wrong.py:9: error: Argument 1 to "set_color" of "Line2D" has incompatible type "complex"; \
expected "tuple[float, float, float] | str | str | tuple[float, float, float, float] | \
tuple[tuple[float, float, float] | str, float] | tuple[tuple[float, float, float, float], \
float]"  [arg-type]
wrong.py:10: error: Unexpected keyword argument "foo" for "set_xlim" of "_AxesBase"  [call-arg]
wrong.py:10: note: "set_xlim" defined in "pltrs.axes._base"
wrong.py:10: error: Argument 1 to "set_xlim" of "_AxesBase" has incompatible type "str"; \
expected "float | tuple[float, float] | None"  [arg-type]
wrong.py:11: error: Incompatible types in assignment (expression has type "Axes", variable \
has type "int")  [assignment]
wrong.py:12: note: Revealed type is "pltrs.cm.ColormapRegistry"
Found 4 errors in 1 file (checked 1 source file)
"""


@pytest.mark.parametrize("name, program, expected", [
    ("clean.py", CLEAN, "Success: no issues found in 1 source file\n"),
    ("wrong.py", WRONG, WRONG_DIAGNOSTICS),
])
def test_mypy_reads_pltrs_as_matplotlib(tmp_path, name, program, expected):
    if importlib.util.find_spec("mypy") is None:
        pytest.skip("mypy is not installed here (matplotlib's test requirements do not "
                    "have it); cargo xtask diff --types runs it in venvs of its own")
    (tmp_path / name).write_text(program)
    done = subprocess.run(
        [sys.executable, "-m", "mypy", "--cache-dir", str(tmp_path / "cache"), "--no-color-output", name],
        cwd=tmp_path, capture_output=True, text=True,
        env={"PATH": "", "PYTHONPATH": str(PACKAGE.parent), "MYPY_FORCE_COLOR": "0"})
    assert done.stdout.replace("\n\n", "\n") == expected, done.stdout + done.stderr
