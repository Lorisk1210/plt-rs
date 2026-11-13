"""
matplotlib's public instance attributes that pltrs (Rust) holds, assigned
as programs assign them (task TYPES2): each reads back what was assigned,
and the later reads matplotlib makes of it see it (the next draw, the
patch transform, the next setter or ``set_data``, a pickle), as recorded
from matplotlib 3.11.2 (``target/types1/assign/record.py``); and the type
information of pltrs' toolkits, untyped as matplotlib's are.
"""

import pathlib
import pickle

import numpy as np

import pltrs
import pltrs.pyplot as plt
from pltrs import cbook, patches

PACKAGE = pathlib.Path(pltrs.__file__).parent

#: What matplotlib 3.11.2 gives for `scene`.
EXPECTED = {
    "wedge_assigned": ["Wedge(center=(0.5, 0.5), r=0.45, theta1=10, theta2=200, width=None)", True],
    "wedge_after_setter": [0.05, 0.3461, 0.8728, 0.6039],
    "stairs_assigned": [0.0, 0.0, 3.0, 3.0],
    "stairs_after_set_data": [0.0, 0.0, 3.0, 3.0],
    "arrow_assigned": [[[0, 0], [1, 0], [1, 1]], [0.082, 0.082, 0.677, 0.677]],
    "arrow_after_set_data": (8, 2),
    "polygon": ["RegularPolygon((0.5, 0.5), 8, radius=0.1, orientation=0)",
                [[0.1, 0.0, 0.5], [0.0, 0.1, 0.5], [0.0, 0.0, 1.0]], 6],
    "arc": [90, [0.0, 1.0]],
    "origin": ["lower", [-0.5, 2.5, 1.5, -0.5]],
    "axison": [0, True],
    "contour": [-5.0, [1, 2], [-5.0]],
}


def scene():
    """Assign the attributes, then read them as matplotlib's code does."""
    out = {}
    fig, ax = plt.subplots()
    wedge = patches.Wedge((0.5, 0.5), 0.3, 10, 200)
    ax.add_patch(wedge)
    before = np.round(wedge.get_path().get_extents().bounds, 4).tolist()
    wedge.r = 0.45
    # The path stays until a setter clears it, which reads every field.
    out["wedge_assigned"] = [
        str(wedge), np.round(wedge.get_path().get_extents().bounds, 4).tolist() == before]
    wedge.set_theta1(20)
    out["wedge_after_setter"] = np.round(wedge.get_path().get_extents().bounds, 4).tolist()
    stairs = ax.stairs([1, 3, 2], [0, 1, 2, 3])
    stairs.orientation = "horizontal"
    out["stairs_assigned"] = np.round(stairs.get_path().get_extents().bounds, 4).tolist()
    stairs.set_data([1, 3, 2])
    out["stairs_after_set_data"] = np.round(stairs.get_path().get_extents().bounds, 4).tolist()
    arrow = ax.arrow(0.1, 0.1, 0.5, 0.5, width=0.05)
    arrow.verts = [[0, 0], [1, 0], [1, 1]]
    out["arrow_assigned"] = [arrow.verts,
                             np.round(arrow.get_path().get_extents().bounds, 3).tolist()]
    arrow.set_data(dx=0.2)
    out["arrow_after_set_data"] = np.shape(arrow.verts)
    polygon = patches.RegularPolygon((0.5, 0.5), 5, radius=0.3)
    ax.add_patch(polygon)
    polygon.radius = 0.1
    polygon.numvertices = 8
    out["polygon"] = [str(polygon), np.round(polygon.get_patch_transform().get_matrix(), 4).tolist(),
                      len(polygon.get_path().vertices)]
    arc = patches.Arc((0.5, 0.5), 0.6, 0.6, theta1=10, theta2=200)
    ax.add_patch(arc)
    arc.theta2 = 90
    fig.canvas.draw()
    out["arc"] = [arc.theta2, np.round(arc.get_path().vertices[-1], 4).tolist()]
    image = ax.imshow(np.arange(6.).reshape(2, 3))
    image.origin = "lower"
    out["origin"] = [image.origin, list(image.get_extent())]
    ax.axison = 0
    fig.canvas.draw()
    out["axison"] = [ax.axison, ax.xaxis.get_visible()]
    contours = ax.contour(np.arange(16.).reshape(4, 4))
    contours.zmin, contours.layers = -5.0, [1, 2]
    copy = pickle.loads(pickle.dumps(fig))
    out["contour"] = [contours.zmin, contours.layers,
                      [c.zmin for c in copy.axes[0].collections if hasattr(c, "zmin")]]
    plt.close(fig)
    return out


def test_assigned_attributes_are_read_where_matplotlib_reads_them():
    found = scene()
    for key, expected in EXPECTED.items():
        assert found[key] == expected, key


def drawn(fig):
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def test_assignments_read_at_draw_change_the_next_draw():
    # matplotlib's draws read these: the polygon's transform, the arc's
    # angles, the image's origin, whether the Axes' decorations are drawn.
    for make, name, value in [
            (lambda ax: ax.add_patch(patches.RegularPolygon((0.5, 0.5), 5, radius=0.3)),
             "xy", (0.3, 0.3)),
            (lambda ax: ax.add_patch(patches.Arc((0.5, 0.5), 0.6, 0.4, theta1=10, lw=3)),
             "theta1", 90),
            (lambda ax: ax.imshow(np.add.outer(np.arange(4.), np.arange(4.) ** 2)),
             "origin", "lower"),
            (lambda ax: ax, "axison", False)]:
        fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
        artist = make(ax)
        before = drawn(fig)
        setattr(artist, name, value)
        assert getattr(artist, name) == value
        assert not np.array_equal(before, drawn(fig)), name
        plt.close(fig)


def test_assignments_read_only_when_made_leave_the_draw():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    wedge = ax.add_patch(patches.Wedge((0.5, 0.5), 0.3, 10, 200))
    contours = ax.contour(np.arange(16.).reshape(4, 4))
    triangles = ax.tricontour([0, 1, 0, 1], [0, 0, 1, 1], [0, 1, 2, 3])
    before = drawn(fig)
    wedge.center, wedge.theta2, wedge.width = (0.2, 0.2), 300, 0.1
    contours.zmax = 50.0
    triangles.zmin = -1.0
    assert np.array_equal(before, drawn(fig))
    assert (wedge.center, wedge.theta2, wedge.width) == ((0.2, 0.2), 300, 0.1)
    assert (contours.zmax, triangles.zmin) == (50.0, -1.0)
    plt.close(fig)


def test_assigned_callbacks_hear_the_limit_changes():
    fig, ax = plt.subplots()
    seen = []
    ax.callbacks.connect("xlim_changed", lambda axes: seen.append("old"))
    ax.callbacks = cbook.CallbackRegistry()
    ax.callbacks.connect("xlim_changed", lambda axes: seen.append("new"))
    ax.set_xlim(0, 5)
    assert seen == ["new"]  # matplotlib 3.11.2
    plt.close(fig)


def test_toolkits_are_untyped_for_checkers():
    # A stub of a module-level __getattr__ alone beside each module: every
    # name Any, as matplotlib's mpl_toolkits (no py.typed) are.
    toolkits = PACKAGE / "mpl_toolkits"
    modules = sorted(toolkits.rglob("*.py"))
    assert len(modules) > 30
    for module in modules:
        stub = module.with_suffix(".pyi").read_text(encoding="utf-8")
        assert "def __getattr__(name: str) -> Any: ..." in stub, module
