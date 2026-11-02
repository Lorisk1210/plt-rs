"""Every getter of every kind of artist, before and after a draw, then
every setter with sample values, each followed by its getter.

A subject is a few statements leaving the artist in ``obj``. Its program
calls each getter (``common.getters``) and reads each public attribute,
draws the figure, calls the getters again, and then sets each property
``VALUES`` has samples for, reading it back after each sample. Properties
without samples are set to the value matplotlib's getter gave, when that
value has a literal.
"""

from __future__ import annotations

from typing import Any, Iterator

from . import common

AXES = ["fig, ax = plt.subplots()"]

#: (name, statements leaving the artist in ``obj``)
SUBJECTS: list[tuple[str, list[str]]] = [
    ("figure", ["fig = plt.figure()", "obj = fig"]),
    ("axes", AXES + ["ax.plot([1, 2, 3])", "obj = ax"]),
    ("axes_polar", ["fig = plt.figure()", "obj = fig.add_subplot(projection='polar')", "obj.plot(x, values)"]),
    ("axes3d", ["fig = plt.figure()", "obj = fig.add_subplot(projection='3d')", "obj.plot(x, y, values)"]),
    ("line", AXES + ["obj = ax.plot(x, y, 'o-', label='sin')[0]"]),
    ("line_loose", AXES + ["obj = lines.Line2D([0, 1, 2], [2, 0, 1])"]),
    ("line_added", AXES + ["obj = lines.Line2D([0, 1, 2], [2, 0, 1], color='r')", "ax.add_line(obj)"]),
    ("axline", AXES + ["obj = ax.axline((0, 0), (1, 2))"]),
    ("axhline", AXES + ["obj = ax.axhline(0.5)"]),
    ("text", AXES + ["obj = ax.text(0.5, 0.5, 'hello', ha='center')"]),
    ("text_box", AXES + ["obj = ax.text(0.2, 0.3, 'boxed', bbox=dict(boxstyle='round', fc='y'))"]),
    ("title", AXES + ["obj = ax.set_title('title')"]),
    ("annotation", AXES + ["obj = ax.annotate('note', xy=(0.2, 0.2), xytext=(0.6, 0.7), "
                           "arrowprops=dict(arrowstyle='->'))"]),
    ("annotation_fancy", AXES + ["obj = ax.annotate('fancy', xy=(0.5, 0.5), xytext=(0.1, 0.9), "
                                 "arrowprops=dict(facecolor='black', shrink=0.05))"]),
    ("rectangle", AXES + ["obj = patches.Rectangle((0.1, 0.2), 0.5, 0.3, fc='r')", "ax.add_patch(obj)"]),
    ("rectangle_loose", ["obj = patches.Rectangle((0.1, 0.2), 0.5, 0.3, angle=20)"]),
    ("circle", AXES + ["obj = patches.Circle((0.5, 0.5), 0.2)", "ax.add_patch(obj)"]),
    ("ellipse", AXES + ["obj = patches.Ellipse((0.5, 0.5), 0.4, 0.2, angle=30)", "ax.add_patch(obj)"]),
    ("polygon", AXES + ["obj = patches.Polygon([[0, 0], [1, 0], [0.5, 1]], closed=True)", "ax.add_patch(obj)"]),
    ("wedge", AXES + ["obj = patches.Wedge((0.5, 0.5), 0.3, 30, 120, width=0.1)", "ax.add_patch(obj)"]),
    ("arc", AXES + ["obj = patches.Arc((0.5, 0.5), 0.4, 0.3, theta1=10, theta2=200)", "ax.add_patch(obj)"]),
    ("regular_polygon", AXES + ["obj = patches.RegularPolygon((0.5, 0.5), 6, radius=0.2)", "ax.add_patch(obj)"]),
    ("circle_polygon", AXES + ["obj = patches.CirclePolygon((0.5, 0.5), 0.2)", "ax.add_patch(obj)"]),
    ("annulus", AXES + ["obj = patches.Annulus((0.5, 0.5), 0.3, 0.1)", "ax.add_patch(obj)"]),
    ("fancy_box", AXES + ["obj = patches.FancyBboxPatch((0.2, 0.2), 0.4, 0.3, boxstyle='round,pad=0.05')",
                          "ax.add_patch(obj)"]),
    ("fancy_arrow_patch", AXES + ["obj = patches.FancyArrowPatch((0.1, 0.1), (0.8, 0.6), arrowstyle='-|>', "
                                  "mutation_scale=20)", "ax.add_patch(obj)"]),
    ("fancy_arrow", AXES + ["obj = patches.FancyArrow(0.1, 0.1, 0.5, 0.3, width=0.05)", "ax.add_patch(obj)"]),
    ("arrow", AXES + ["obj = patches.Arrow(0.1, 0.1, 0.5, 0.3)", "ax.add_patch(obj)"]),
    ("path_patch", AXES + ["obj = patches.PathPatch(path.Path([(0, 0), (1, 1), (1, 0)], [1, 2, 2]))",
                           "ax.add_patch(obj)"]),
    ("connection_patch", AXES + ["obj = patches.ConnectionPatch((0.1, 0.1), (0.9, 0.9), 'data')",
                                 "ax.add_patch(obj)"]),
    ("step_patch", AXES + ["obj = ax.stairs([1, 3, 2], [0, 1, 2, 3])"]),
    ("shadow", AXES + ["p = patches.Circle((0.5, 0.5), 0.2)", "ax.add_patch(p)",
                       "obj = patches.Shadow(p, 0.02, -0.02)", "ax.add_patch(obj)"]),
    ("bar_patch", AXES + ["obj = ax.bar([1, 2, 3], [3, 1, 2])[1]"]),
    ("scatter", AXES + ["obj = ax.scatter(x, y, c=values, s=values * 10)"]),
    ("line_collection", AXES + ["obj = collections.LineCollection([[(0, 0), (1, 1)], [(0, 1), (1, 0)]], "
                                "linewidths=[1, 3])", "ax.add_collection(obj)"]),
    ("poly_collection", AXES + ["obj = ax.fill_between(x, y, 0)"]),
    ("fill_between_poly", AXES + ["obj = ax.fill_betweenx(x, y, 1)"]),
    ("quadmesh", AXES + ["obj = ax.pcolormesh(z)"]),
    ("pcolor", AXES + ["obj = ax.pcolor(z)"]),
    ("hexbin", AXES + ["obj = ax.hexbin(x, y, gridsize=4)"]),
    ("eventplot", AXES + ["obj = ax.eventplot([1, 2, 4])[0]"]),
    ("quiver", AXES + ["obj = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])"]),
    ("quiver_key", AXES + ["q = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])",
                           "obj = ax.quiverkey(q, 0.8, 0.9, 1, 'one')"]),
    ("barbs", AXES + ["obj = ax.barbs([0, 1], [0, 1], [10, -20], [15, 5])"]),
    ("image", AXES + ["obj = ax.imshow(z)"]),
    ("image_rgb", AXES + ["obj = ax.imshow(np.dstack([z / 2 + 0.5] * 3).clip(0, 1))"]),
    ("figimage", ["fig = plt.figure()", "obj = fig.figimage(z)"]),
    ("matshow", AXES + ["obj = ax.matshow(z)"]),
    ("spy", AXES + ["obj = ax.spy(z > 0)"]),
    ("contour", AXES + ["obj = ax.contour(z, levels=3)"]),
    ("contourf", AXES + ["obj = ax.contourf(z)"]),
    ("tricontour", AXES + ["obj = ax.tricontour([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5], [1, 2, 3, 4, 0])"]),
    ("triplot", AXES + ["obj = ax.triplot([0, 1, 0, 1], [0, 0, 1, 1])[0]"]),
    ("tripcolor", AXES + ["obj = ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4])"]),
    ("legend", AXES + ["ax.plot([1, 2], label='one')", "ax.bar([1], [2], label='two')", "obj = ax.legend()"]),
    ("fig_legend", AXES + ["ax.plot([1, 2], label='one')", "obj = fig.legend()"]),
    ("colorbar", AXES + ["im = ax.imshow(z)", "obj = fig.colorbar(im)"]),
    ("colorbar_axes", AXES + ["im = ax.imshow(z)", "obj = fig.colorbar(im).ax"]),
    ("table", AXES + ["obj = ax.table([['a', 'b'], ['c', 'd']], loc='center')"]),
    ("table_cell", AXES + ["obj = ax.table([['a', 'b'], ['c', 'd']], loc='center')[0, 1]"]),
    ("xaxis", AXES + ["ax.plot([1, 2, 3])", "obj = ax.xaxis"]),
    ("yaxis_log", AXES + ["ax.plot([1, 10, 100])", "ax.set_yscale('log')", "obj = ax.yaxis"]),
    ("major_tick", AXES + ["ax.plot([1, 2, 3])", "obj = ax.xaxis.get_major_ticks()[0]"]),
    ("tick_label", AXES + ["ax.plot([1, 2, 3])", "obj = ax.get_xticklabels()[0]"]),
    ("spine", AXES + ["obj = ax.spines['left']"]),
    ("errorbar", AXES + ["obj = ax.errorbar([1, 2, 3], [2, 1, 3], yerr=0.5, xerr=[0.1, 0.2, 0.3])"]),
    ("errorbar_caplines", AXES + ["obj = ax.errorbar([1, 2], [2, 1], yerr=0.5, capsize=3).lines[1][0]"]),
    ("stem", AXES + ["obj = ax.stem([1, 2, 3], [2, 1, 3])"]),
    ("bar_container", AXES + ["obj = ax.bar(['a', 'b'], [1, 2])"]),
    ("hist", AXES + ["obj = ax.hist(values, bins=3)"]),
    ("boxplot", AXES + ["obj = ax.boxplot([values, values * 2])"]),
    ("violin", AXES + ["obj = ax.violinplot([values, values * 2])"]),
    ("pie", AXES + ["obj = ax.pie([1, 2, 3], labels=['a', 'b', 'c'], autopct='%1.0f%%')"]),
    ("pie_wedge", AXES + ["obj = ax.pie([1, 2, 3])[0][1]"]),
    ("streamplot", AXES + ["u = np.ones((5, 6))", "v = z", "obj = ax.streamplot(np.arange(6), np.arange(5), u, v)"]),
    ("anchored_text", AXES + ["from matplotlib.offsetbox import AnchoredText",
                              "obj = AnchoredText('anchored', loc='upper left')", "ax.add_artist(obj)"]),
    ("annotation_bbox", AXES + ["from matplotlib.offsetbox import AnnotationBbox, TextArea",
                                "obj = AnnotationBbox(TextArea('box'), (0.5, 0.5))", "ax.add_artist(obj)"]),
    ("offset_image", AXES + ["from matplotlib.offsetbox import AnnotationBbox, OffsetImage",
                             "obj = OffsetImage(z, zoom=2)", "ax.add_artist(AnnotationBbox(obj, (0.5, 0.5)))"]),
    ("secondary_axis", AXES + ["obj = ax.secondary_xaxis('top', functions=(lambda v: 2 * v, lambda v: v / 2))"]),
    ("inset_axes", AXES + ["obj = ax.inset_axes([0.6, 0.6, 0.3, 0.3])"]),
    ("indicate_inset", AXES + ["ins = ax.inset_axes([0.6, 0.6, 0.3, 0.3])", "obj = ax.indicate_inset_zoom(ins)"]),
    ("twinx", AXES + ["ax.plot([1, 2, 3])", "obj = ax.twinx()"]),
    ("subfigure", ["fig = plt.figure()", "obj = fig.subfigures(1, 2)[0]", "obj.subplots()"]),
    ("suptitle", ["fig = plt.figure()", "obj = fig.suptitle('super')"]),
    ("supxlabel", ["fig = plt.figure()", "obj = fig.supxlabel('xs')"]),
    ("gridspec", ["fig = plt.figure()", "obj = fig.add_gridspec(2, 3, wspace=0.3)"]),
    ("subplotspec", ["fig = plt.figure()", "obj = fig.add_gridspec(2, 3)[0, 1:]"]),
    ("surface3d", ["fig = plt.figure()", "ax = fig.add_subplot(projection='3d')",
                   "X, Y = np.meshgrid(np.arange(6), np.arange(5))", "obj = ax.plot_surface(X, Y, z)"]),
    ("scatter3d", ["fig = plt.figure()", "ax = fig.add_subplot(projection='3d')", "obj = ax.scatter(x, y, values)"]),
    ("line3d", ["fig = plt.figure()", "ax = fig.add_subplot(projection='3d')", "obj = ax.plot(x, y, values)[0]"]),
    ("zaxis", ["fig = plt.figure()", "ax = fig.add_subplot(projection='3d')", "obj = ax.zaxis"]),
]

#: Sample values per property, set one after the other.
VALUES: dict[str, list[str]] = {
    "alpha": ["0.5", "None"],
    "animated": ["True"],
    "antialiased": ["False"],
    "color": ["'r'", "(0.1, 0.2, 0.3)", "'#ff000080'", "'C2'"],
    "linewidth": ["2", "0.5"],
    "linestyle": ["'--'", "':'", "(0, (3, 1))", "'dashdot'", "'None'"],
    "marker": ["'o'", "'s'", "None", "'$x$'"],
    "markersize": ["10", "3.5"],
    "markeredgewidth": ["2"],
    "markeredgecolor": ["'k'"],
    "markerfacecolor": ["'none'", "'b'"],
    "markerfacecoloralt": ["'g'"],
    "fillstyle": ["'left'", "'none'"],
    "drawstyle": ["'steps-mid'"],
    "dash_capstyle": ["'round'"],
    "dash_joinstyle": ["'miter'"],
    "solid_capstyle": ["'projecting'"],
    "solid_joinstyle": ["'bevel'"],
    "label": ["'x'", "'_nolegend_'", "3"],
    "zorder": ["5", "2.5"],
    "visible": ["False", "True"],
    "gid": ["'g'"],
    "url": ["'http://x'"],
    "clip_on": ["False"],
    "rasterized": ["True"],
    "snap": ["True"],
    "sketch_params": ["1, 100, 2", "None"],
    "picker": ["5", "True"],
    "pickradius": ["3"],
    "markevery": ["2", "(1, 2)", "[0, 2]", "0.1"],
    "xdata": ["[3, 2, 1]"],
    "ydata": ["[1, 2, 3]"],
    "facecolor": ["'g'", "(1, 0, 0, 0.5)", "'none'"],
    "edgecolor": ["'r'", "'none'"],
    "hatch": ["'//'", "'x'"],
    "hatch_linewidth": ["2"],
    "fill": ["False"],
    "capstyle": ["'projecting'"],
    "joinstyle": ["'miter'"],
    "width": ["2"],
    "height": ["3"],
    "xy": ["(1, 2)"],
    "angle": ["30"],
    "radius": ["2"],
    "center": ["(0.3, 0.4)"],
    "theta1": ["15"],
    "theta2": ["150"],
    "text": ["'hello'", "3.5", "'$\\\\alpha$'"],
    "fontsize": ["14", "'large'"],
    "fontweight": ["'bold'", "700"],
    "fontstyle": ["'italic'"],
    "fontfamily": ["'serif'", "'monospace'"],
    "fontvariant": ["'small-caps'"],
    "fontstretch": ["'condensed'"],
    "rotation": ["45", "'vertical'", "400"],
    "rotation_mode": ["'anchor'"],
    "horizontalalignment": ["'right'"],
    "verticalalignment": ["'top'", "'baseline'"],
    "multialignment": ["'center'"],
    "linespacing": ["2"],
    "wrap": ["True"],
    "backgroundcolor": ["'y'"],
    "position": ["(0.3, 0.4)"],
    "x": ["0.2"],
    "y": ["0.3"],
    "title": ["'t'"],
    "xlim": ["(0, 10)", "(5, -5)"],
    "ylim": ["(-1, 1)"],
    "xlabel": ["'xl'"],
    "ylabel": ["'yl'"],
    "xscale": ["'log'", "'symlog'"],
    "yscale": ["'logit'"],
    "aspect": ["'equal'", "2"],
    "frame_on": ["False"],
    "axisbelow": ["True", "'line'"],
    "xticks": ["[0, 1, 2]"],
    "yticks": ["[0.5]"],
    "xticklabels": ["['a', 'b']"],
    "navigate": ["False"],
    "adjustable": ["'datalim'"],
    "anchor": ["'NE'"],
    "autoscale_on": ["False"],
    "xmargin": ["0.2"],
    "ymargin": ["0"],
    "box_aspect": ["1"],
    "cmap": ["'viridis'", "'RdBu_r'"],
    "clim": ["(0, 1)"],
    "sizes": ["[10, 20]"],
    "linewidths": ["[1, 2]"],
    "linestyles": ["'--'"],
    "facecolors": ["['r', 'g']"],
    "edgecolors": ["'k'"],
    "offsets": ["[[0, 0], [1, 1]]"],
    "interpolation": ["'nearest'", "'bicubic'"],
    "extent": ["(0, 1, 0, 1)"],
    "dpi": ["72"],
    "figwidth": ["4"],
    "figheight": ["3"],
    "size_inches": ["(4, 3)"],
    "frameon": ["False"],
    "layout_engine": ["'tight'", "'constrained'", "'none'"],
    "in_layout": ["False"],
    "clip_box": ["None"],
    "path_effects": ["[]"],
    "mouseover": ["True"],
    "pad": ["3"],
    "boxstyle": ["'round'", "'sawtooth,pad=0.2'"],
    "arrowstyle": ["'<->'", "'fancy'"],
    "connectionstyle": ["'arc3,rad=0.3'"],
    "mutation_scale": ["30"],
    "mutation_aspect": ["2"],
    "closed": ["False"],
    "bounds": ["(0, 0, 2, 3)"],
    "label_position": ["'top'"],
    "ticks_position": ["'both'"],
    "tick_params": [],
    "fontsize_scale": [],
    "usetex": [],
    "math_fontfamily": ["'cm'"],
    "antialiaseds": ["False"],
    "urls": ["['u']"],
    "array": ["np.arange(4.0)"],
    "norm": ["colors.LogNorm()"],
    "transform": ["transforms.IdentityTransform()"],
    "data": [],
    "orientation": ["'horizontal'"],
    "lineoffset": ["2"],
    "linelength": ["0.5"],
    "positions": ["[0.5, 1.5]"],
    "resample": ["True"],
    "filternorm": ["False"],
    "filterrad": ["2.0"],
    "zdata": [],
    "box": [],
}


def _literal_round_trip(obj: Any, name: str) -> list[str]:
    getter = getattr(obj, f"get_{name}", None)
    if getter is None or not callable(getter):
        return []
    try:
        value = getter()
    except Exception:  # noqa: BLE001 - a getter matplotlib cannot answer gives no sample
        return []
    source = common.literal(value)
    return [source] if source is not None else []


def subject_programs(name: str, setup: list[str]) -> Iterator[tuple[str, list[str]]]:
    """The getters program and the setters program of one subject."""
    statements = common.HEADER + common.DATA + setup
    try:
        obj = common.namespace(statements)["obj"]
    except Exception as err:  # noqa: BLE001 - a broken subject is the table's error
        raise RuntimeError(f"subject {name} does not run under matplotlib: {err}") from err
    probes = list(common.probe_object("obj", obj))
    draw = ["plt.gcf().canvas.draw()"]
    yield f"{name}_getters", statements + probes + draw + probes
    sets: list[str] = []
    for prop in common.setters(obj):
        samples = VALUES.get(prop)
        if samples is None:
            samples = _literal_round_trip(obj, prop)
        has_getter = callable(getattr(obj, f"get_{prop}", None))
        for sample in samples:
            sets.append(f"obj.set_{prop}({sample})")
            if has_getter:
                sets.append(f"obj.get_{prop}()")
    yield f"{name}_setters", statements + sets + draw + probes


def programs() -> Iterator[tuple[str, list[str]]]:
    """The getters and setters programs of every subject."""
    for name, setup in SUBJECTS:
        yield from subject_programs(name, setup)
