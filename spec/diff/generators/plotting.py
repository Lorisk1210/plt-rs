"""Every plotting method of ``Axes`` with argument variants.

One program per method: each variant plots on a fresh axes, then the
program reads what matplotlib's tests read after plotting (the data and
view limits, the ticks and their labels, the artists of the axes), draws,
and reads the limits and ticks again. Every figure is compared as an image
at the end.
"""

from __future__ import annotations

from typing import Iterator

#: method -> argument lists (the text between the call's parentheses)
VARIANTS: dict[str, list[str]] = {
    "plot": ["[1, 2, 3]", "x, y", "x, y, 'r--'", "x, y, 'go', x, -y, 'b^'", "x, y, color='g', lw=3, marker='s'",
             "[], []", "x, np.where(y > 0, y, np.nan)", "y, drawstyle='steps'", "x, np.vstack([y, -y]).T",
             "['a', 'b', 'c'], [1, 3, 2]", "x, y, markevery=2, ms=12", "np.arange(5, dtype=np.int32)"],
    "scatter": ["x, y", "x, y, s=values * 20, c=values", "x, y, c='r', marker='^'", "x, y, c=values, cmap='plasma',"
                " vmin=0, vmax=5", "[], []", "x, y, s=50, facecolors='none', edgecolors='k'",
                "x, y, c=[(1, 0, 0)] * 7", "x, y, alpha=0.5, linewidths=2"],
    "bar": ["[1, 2, 3], [3, 1, 2]", "['a', 'b'], [1, 2], color=['r', 'g']", "[1, 2], [2, 3], width=0.4, "
            "bottom=1, align='edge'", "[1, 2, 3], [1, -1, 2], yerr=0.2, capsize=4", "[1, 2], [3, 4], label='bars',"
            " hatch='//', edgecolor='k'", "x, values, log=True"],
    "barh": ["[1, 2, 3], [3, 1, 2]", "['a', 'b'], [1, 2], left=[1, 0], height=0.5"],
    "hist": ["values", "values, bins=4, density=True", "values, bins=[0, 2, 4, 10], histtype='step'",
             "[values, values * 0.5], stacked=True", "values, orientation='horizontal', cumulative=True",
             "values, bins='auto', weights=np.ones(7) / 7", "[]", "values, range=(0, 5), log=True"],
    "hist2d": ["x, y, bins=3", "x, y, bins=[2, 3], density=True"],
    "errorbar": ["x, y, yerr=0.2", "x, y, xerr=0.1, yerr=[0.1] * 7, fmt='o', capsize=3",
                 "x, y, yerr=[np.full(7, 0.1), np.full(7, 0.3)], uplims=True", "x, y, errorevery=2, ecolor='r'"],
    "fill_between": ["x, y", "x, y, 0.5, where=y > 0, interpolate=True", "x, -1, y, step='mid', alpha=0.3"],
    "fill_betweenx": ["x, y", "x, y, 1, color='g'"],
    "fill": ["[0, 1, 1], [0, 0, 1]", "[0, 1, 1], [0, 0, 1], 'r', [2, 3, 3], [0, 0, 1], 'g'"],
    "stackplot": ["x, y + 1, values", "x, [values, values / 2], labels=['a', 'b'], baseline='sym'"],
    "stairs": ["[1, 3, 2]", "[1, 3, 2], [0, 1, 3, 4], fill=True", "[1, 2], baseline=None, orientation='horizontal'"],
    "step": ["x, y", "x, y, where='mid'", "x, y, 'r-', where='post'"],
    "stem": ["values", "x, values, linefmt='r-', markerfmt='D', basefmt='k:'", "x, values, orientation='horizontal',"
             " bottom=1"],
    "eventplot": ["[1, 2, 4]", "[[1, 2], [3, 4, 5]], orientation='vertical', colors=['r', 'b'], lineoffsets=[1, 3]"],
    "pie": ["[1, 2, 3]", "[1, 2, 3], labels=['a', 'b', 'c'], autopct='%1.1f%%', explode=[0, 0.1, 0]",
            "[0.2, 0.3], normalize=False", "[1, 1], wedgeprops=dict(width=0.3), startangle=90, counterclock=False"],
    "boxplot": ["values", "[values, values * 2], notch=True, vert=False", "[values], showmeans=True, "
                "meanline=True, patch_artist=True", "values, whis=(0, 100), labels=['v']"],
    "violinplot": ["values", "[values, values + 1], showmedians=True, showextrema=False",
                   "values, quantiles=[0.25, 0.75], orientation='horizontal'"],
    "imshow": ["z", "z, cmap='gray', origin='lower', extent=(0, 1, 0, 2)", "z, interpolation='bilinear', "
               "aspect='auto'", "z, vmin=0, vmax=1, alpha=0.5", "np.dstack([z / 4 + 0.5] * 3)",
               "(z * 50).astype(np.uint8)", "np.ma.masked_less(z, 0)", "z, norm='log'"],
    "matshow": ["z", "z, cmap='RdBu'"],
    "pcolormesh": ["z", "np.arange(7), np.arange(6), z", "z, shading='gouraud'", "z, edgecolors='k', lw=1"],
    "pcolor": ["z", "np.arange(7), np.arange(6), z, cmap='magma'"],
    "pcolorfast": ["z", "(0, 1), (0, 2), z"],
    "contour": ["z", "z, levels=[0, 0.5, 1]", "np.arange(6), np.arange(5), z, colors='k', linestyles='dashed'",
                "z, 4, cmap='jet'"],
    "contourf": ["z", "z, levels=4, extend='both'", "z, hatches=['//', None]"],
    "hexbin": ["x, y", "x, y, C=values, gridsize=3, reduce_C_function=np.max", "x, y, bins='log', mincnt=1"],
    "quiver": ["[0, 1], [0, 1], [1, -1], [1, 2]", "z, z", "[0, 1], [0, 1], [1, -1], [1, 2], angles='xy', "
               "scale_units='xy', scale=1", "[0], [0], [1], [1], color='r', width=0.02"],
    "barbs": ["[0, 1], [0, 1], [10, -20], [15, 5]", "[0, 1], [0, 1], [10, -20], [15, 5], length=8, "
              "barbcolor='r'"],
    "streamplot": ["np.arange(6), np.arange(5), np.ones((5, 6)), z", "np.arange(6), np.arange(5), "
                   "np.ones((5, 6)), z, density=0.5, color=z, linewidth=2"],
    "spy": ["z > 0", "z, precision=0.5, marker='s'"],
    "specgram": ["np.sin(np.arange(512) * 0.3), NFFT=64, Fs=2"],
    "psd": ["np.sin(np.arange(256) * 0.3), NFFT=64"],
    "csd": ["np.sin(np.arange(256) * 0.3), np.cos(np.arange(256) * 0.2), NFFT=64"],
    "cohere": ["np.sin(np.arange(256) * 0.3), np.cos(np.arange(256) * 0.2), NFFT=64"],
    "magnitude_spectrum": ["np.sin(np.arange(128) * 0.3)"],
    "angle_spectrum": ["np.sin(np.arange(128) * 0.3)"],
    "phase_spectrum": ["np.sin(np.arange(128) * 0.3)"],
    "acorr": ["values"],
    "xcorr": ["values, values[::-1], maxlags=3"],
    "loglog": ["x + 1, values"],
    "semilogx": ["x + 1, values"],
    "semilogy": ["x, values", "x, values, base=2"],
    "axhline": ["0.5", "y=0.2, xmin=0.1, xmax=0.4, color='r'"],
    "axvline": ["0.5", "x=0.3, ls='--'"],
    "axline": ["(0, 0), (1, 1)", "(0, 0), slope=2"],
    "axhspan": ["0.2, 0.4", "0.1, 0.3, xmin=0.2, alpha=0.5"],
    "axvspan": ["0.2, 0.4", "0.1, 0.3, color='g'"],
    "hlines": ["[1, 2], 0, 1", "[1, 2, 3], [0, 1, 0], 2, colors=['r', 'g', 'b']"],
    "vlines": ["[1, 2], 0, 1", "[1, 2], [0, 1], [1, 2], linestyles='dotted'"],
    "text": ["0.5, 0.5, 'hi'", "0.1, 0.2, 'rotated', rotation=30, fontsize=15, color='r'",
             "0.5, 0.5, '$x^2$', transform=ax.transAxes", "0.5, 0.5, 'line\\ntwo', ha='center', va='center'"],
    "annotate": ["'a', (0.5, 0.5)", "'b', xy=(0.2, 0.2), xytext=(0.8, 0.8), arrowprops=dict(arrowstyle='->')",
                 "'c', xy=(0.5, 0.5), xycoords='axes fraction', textcoords='offset points', xytext=(10, 10)"],
    "table": ["[['a', 'b'], ['c', 'd']]", "cellText=[['1', '2']], colLabels=['x', 'y'], loc='bottom'"],
    "arrow": ["0, 0, 0.5, 0.5", "0.1, 0.1, 0.3, 0.6, head_width=0.05, color='r'"],
    "broken_barh": ["[(1, 2), (4, 1)], (0, 1)", "[(1, 2)], (2, 1), facecolors='r'"],
    "triplot": ["[0, 1, 0, 1], [0, 0, 1, 1]"],
    "tripcolor": ["[0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4]"],
    "tricontour": ["[0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5], [1, 2, 3, 4, 0]"],
    "tricontourf": ["[0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5], [1, 2, 3, 4, 0], levels=3"],
    "legend": ["", "['one']", "loc='upper left', ncols=2, title='t'", "handles=[], labels=[]"],
    "grid": ["", "True, which='both', axis='x', color='r'"],
    "set_xlim": ["0, 10", "left=-1", "(3, 1)"],
    "set_ylim": ["-1, 1", "bottom=0"],
    "set_xscale": ["'log'", "'symlog', linthresh=0.5", "'logit'", "'asinh'", "'function', "
                   "functions=(np.sqrt, np.square)"],
    "set_yscale": ["'log', base=2"],
    "set_xticks": ["[0, 1, 2]", "[0, 1], labels=['a', 'b']", "[0.5], minor=True"],
    "set_aspect": ["'equal'", "2, adjustable='datalim'"],
    "margins": ["0.2", "x=0, y=0.3"],
    "tick_params": ["axis='x', labelsize=14, rotation=45", "direction='in', length=8, which='both'"],
    "ticklabel_format": ["style='sci', scilimits=(0, 0)", "useOffset=False"],
    "locator_params": ["nbins=3"],
    "autoscale": ["False", "axis='x', tight=True"],
    "invert_xaxis": [""],
    "set_title": ["'t'", "'left title', loc='left', pad=20", "'big', fontsize=20, fontweight='bold'"],
    "set_xlabel": ["'x'", "'x', labelpad=10, loc='right'"],
    "bar_label": ["ax.bar([1, 2], [3, 4])", "ax.bar([1, 2], [3, 4]), fmt='%.1f', label_type='center'"],
    "secondary_xaxis": ["'top'", "0.5"],
    "inset_axes": ["[0.5, 0.5, 0.4, 0.4]"],
    "twinx": [""],
    "twiny": [""],
    "ecdf": ["values", "values, complementary=True"],
}

#: What a program reads after each variant.
AFTER = [
    "ax.get_xlim()",
    "ax.get_ylim()",
    "ax.dataLim.bounds",
    "ax.get_xticks()",
    "ax.get_yticks()",
    "[t.get_text() for t in ax.get_xticklabels()]",
    "[type(c).__name__ for c in ax.get_children()]",
    "len(ax.lines), len(ax.patches), len(ax.collections), len(ax.images), len(ax.texts)",
]


def programs() -> Iterator[tuple[str, list[str]]]:
    """One program per plotting method."""
    from . import common

    for method, variants in VARIANTS.items():
        statements = list(common.HEADER + common.DATA)
        for arguments in variants:
            statements += [
                "fig, ax = plt.subplots()",
                f"result = ax.{method}({arguments})",
                *AFTER,
                "fig.canvas.draw()",
                *AFTER,
            ]
        yield method, statements
