"""pyplot's functions: the state they keep and what they return.

``state`` calls pyplot's state functions in an order a session would;
``wrappers`` calls the plotting wrappers that need no arguments and the
colormap functions; ``queries`` reads every function pyplot has that takes
no argument and does not wait for a user.
"""

from __future__ import annotations

import inspect
from typing import Iterator

from . import common

#: pyplot functions a program never calls without arguments: they wait for
#: a user, show a window, or change the session for the programs after.
NOT_CALLED = {
    "ginput", "waitforbuttonpress", "show", "pause", "subplot_tool", "install_repl_displayhook",
    "uninstall_repl_displayhook", "xkcd", "rc_context", "ion", "ioff", "new_figure_manager",
    "get_current_fig_manager", "connect", "disconnect", "switch_backend", "savefig",
}

STATE = [
    "plt.get_fignums()",
    "plt.figure()",
    "plt.figure(5)",
    "plt.figure('named')",
    "plt.get_fignums()",
    "plt.get_figlabels()",
    "plt.gcf().number",
    "plt.figure(5) is plt.gcf()",
    "plt.fignum_exists(5)",
    "plt.close(5)",
    "plt.fignum_exists(5)",
    "plt.close('named')",
    "plt.get_fignums()",
    "plt.subplot(2, 2, 1)",
    "plt.subplot(222)",
    "plt.subplot(2, 2, 1) is plt.gcf().axes[0]",
    "plt.subplot(2, 2, (3, 4))",
    "plt.gca().get_subplotspec().get_geometry()",
    "plt.sca(plt.gcf().axes[0])",
    "plt.gca() is plt.gcf().axes[0]",
    "plt.axes([0.1, 0.1, 0.2, 0.2])",
    "plt.delaxes()",
    "len(plt.gcf().axes)",
    "plt.subplots(2, 3, sharex=True)",
    "plt.subplots(1, 2, squeeze=False)[1].shape",
    "plt.subplot_mosaic('AB;CC')",
    "plt.subplot2grid((3, 3), (0, 0), colspan=2)",
    "plt.plot([1, 2, 3])",
    "plt.xlim()",
    "plt.xlim(0, 5)",
    "plt.ylim(bottom=-1)",
    "plt.xticks()",
    "plt.xticks([0, 2, 4], ['a', 'b', 'c'])",
    "plt.yticks([])",
    "plt.xlabel('x')",
    "plt.ylabel('y', fontsize=14)",
    "plt.title('title')",
    "plt.suptitle('sup')",
    "plt.figtext(0.1, 0.1, 'figtext')",
    "plt.text(1, 1, 'text')",
    "plt.annotate('ann', (1, 1))",
    "plt.legend(['line'])",
    "plt.grid(True)",
    "plt.axis()",
    "plt.axis('off')",
    "plt.axis('equal')",
    "plt.axis([0, 1, 0, 1])",
    "plt.box(False)",
    "plt.margins(0.1)",
    "plt.minorticks_on()",
    "plt.tick_params(labelsize=8)",
    "plt.locator_params(nbins=4)",
    "plt.autoscale(enable=True)",
    "plt.imshow(z)",
    "plt.gci()",
    "plt.colorbar()",
    "plt.clim(0, 1)",
    "plt.set_cmap('gray')",
    "plt.gci().get_cmap().name",
    "plt.viridis()",
    "plt.gci().get_cmap().name",
    "plt.findobj(match=plt.Text)",
    "plt.subplots_adjust(left=0.2, wspace=0.5)",
    "plt.tight_layout()",
    "plt.twinx()",
    "plt.twiny()",
    "plt.polar(x, values)",
    "plt.thetagrids([0, 90, 180])",
    "plt.rgrids([1, 2, 3])",
    "plt.cla()",
    "plt.clf()",
    "plt.gcf().axes",
    "plt.close('all')",
    "plt.get_fignums()",
    "plt.isinteractive()",
    "plt.get_backend()",
    "plt.get_plot_commands()",
    "plt.colormaps()[:5]",
    "plt.color_sequences()[:3]",
    "plt.get_cmap('viridis').name",
    "plt.rcParams['lines.linewidth']",
    "plt.rcParams['axes.prop_cycle']",
    "plt.rcParams['font.family']",
    "plt.rcParams['figure.figsize']",
    "plt.rcParams['lines.dashed_pattern']",
    "plt.rcParams['savefig.dpi']",
    "plt.rcParams['axes.formatter.limits']",
    "plt.rcParams['image.cmap']",
    "plt.rcParams['text.color']",
    "plt.rcParams['date.converter']",
    "plt.rcParams['legend.loc']",
    "plt.rcParams['figure.subplot.left']",
]


def _queries() -> list[str]:
    import matplotlib.pyplot as plt

    statements = []
    for name in sorted(vars(plt)):
        function = getattr(plt, name)
        if name.startswith("_") or name in NOT_CALLED or not inspect.isfunction(function):
            continue
        if function.__module__ != "matplotlib.pyplot":
            continue
        try:
            signature = inspect.signature(function)
        except (TypeError, ValueError):
            continue
        required = [p for p in signature.parameters.values()
                    if p.default is p.empty and p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD)]
        if not required:
            statements += [f"plt.{name}()", "plt.close('all')"]
    return statements


def programs() -> Iterator[tuple[str, list[str]]]:
    """pyplot's state and query programs."""
    base = common.HEADER + common.DATA
    yield "state", base + STATE
    yield "queries", base + _queries()
