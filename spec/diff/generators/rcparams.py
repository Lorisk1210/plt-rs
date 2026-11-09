"""Programs setting rcParams and using style sheets, as users do.

* **Each rcParam** that ``matplotlib.rcParams`` has, but those only GUI
  windows and backends read (``rcparams_values.EXCLUDED``), set to two or
  three valid values other than its default (``rcparams_values.VALUES``):
  the parameters of a scene (``rcparams_scenes``) are set a few at a time,
  the n-th program of a group setting each one's n-th value, each read
  back, then the scene is built, drawn, saved and its artists read; its
  figure stays open, so that its picture is compared too, saved with the
  parameters set (``savefig.*`` as well).
* **Every style sheet** alone, and pairs of them, on one scene with all
  the artists, each parameter the sheets set read back.
* **The rc functions**: ``rc_context`` nested, as a decorator, given a
  file, and restored after an exception; ``rcdefaults``,
  ``rc_file_defaults``, ``rc_file`` and ``rc_params_from_file`` with
  files the program writes, ``matplotlib_fname`` finding them,
  ``matplotlib.rc`` with its aliases, the mapping methods of
  ``RcParams`` (``update``, ``copy``, ``find_all``, ...), the style
  functions, and an invalid value for each validator
  (``rcparams_values.INVALID``).

Scenes needing programs that are not on ``PATH`` (TeX, Ghostscript) are
left out (``rcparams_scenes.REQUIRES``).
"""

from __future__ import annotations

import shutil
from typing import Iterator

import matplotlib
import matplotlib.style

from .rcparams_scenes import REQUIRES, SCENES, SETUP
from .rcparams_values import EXCLUDED, INVALID, VALUES

#: The parameters a program of a scene sets at most.
CHUNK = 6
#: Every other style sheet a pair program combines with each.
PAIR_STEP = 7

#: The scene of the style programs: every kind of artist on four Axes.
STYLE_SCENE = [
    "fig, axs = plt.subplots(2, 2, figsize=(6, 4.5))",
    "ax1, ax2, ax3, ax4 = axs.flat",
    "for i in range(5):\n    ax1.plot(x, y + i / 2, marker='o' if i == 0 else None, label=f'l{i}')",
    "ax1.legend(title='lines')",
    "ax1.set_title('Lines')",
    "ax1.set_xlabel('x')",
    "sc = ax2.scatter(x, y, c=values, s=values * 10)",
    "fig.colorbar(sc, ax=ax2)",
    "bars = ax3.bar([1, 2, 3], [3, 1, 2], yerr=0.3, label='bars')",
    "ax3.errorbar(x / 2, y + 2, yerr=0.2, fmt='s-')",
    "ax3.hist(values / 3, alpha=0.5)",
    "im = ax4.imshow(z)",
    "cs = ax4.contour(z, colors='k')",
    "ax4.clabel(cs)",
    "ax4.text(1, 1, r'$\\alpha^2$')",
    "ax4.grid(True)",
    "fig.suptitle('Style')",
    "fig.canvas.draw()",
    "(ax1.get_facecolor(), fig.get_facecolor(), ax1.spines['left'].get_edgecolor())",
    "[(l.get_color(), l.get_linewidth(), l.get_marker(), l.get_markersize()) for l in ax1.lines]",
    "(ax1.xaxis.get_gridlines()[0].get_visible(), ax1.xaxis.get_gridlines()[0].get_color())",
    "(ax1.title.get_fontsize(), ax1.title.get_color(), ax1.xaxis.label.get_fontsize())",
    "(sc.get_edgecolor(), bars[0].get_facecolor(), bars[0].get_edgecolor(), im.get_cmap().name)",
    "[t.get_text() for t in ax1.get_xticklabels()]",
    "[a.get_position().bounds for a in fig.axes]",
]

#: program -> statements after `SETUP`: the rc functions.
API_PROGRAMS: dict[str, list[str]] = {
    "api_rc_context": [
        "plt.rcParams['lines.linewidth']",
        "with plt.rc_context({'lines.linewidth': 4, 'lines.color': 'r'}):\n"
        "    inner = plt.rcParams['lines.linewidth']\n"
        "    with plt.rc_context({'lines.linewidth': 7}):\n"
        "        innermost = plt.rcParams['lines.linewidth']\n"
        "        fig1, ax1 = plt.subplots(figsize=(2, 2))\n"
        "        line1, = ax1.plot(x, y)\n"
        "    after_inner = plt.rcParams['lines.linewidth']",
        "(inner, innermost, after_inner, plt.rcParams['lines.linewidth'], plt.rcParams['lines.color'])",
        "(line1.get_linewidth(), line1.get_color())",
        "try:\n    with plt.rc_context({'font.size': 30}):\n        raise ValueError('inside')\n"
        "except ValueError as err:\n    caught = str(err)",
        "(caught, plt.rcParams['font.size'])",
        "pathlib.Path('ctx.rc').write_text('lines.linewidth: 9\\nfont.size: 21\\n')",
        "with plt.rc_context(fname='ctx.rc', rc={'font.size': 5}):\n"
        "    sizes = (plt.rcParams['lines.linewidth'], plt.rcParams['font.size'])",
        "(sizes, plt.rcParams['lines.linewidth'], plt.rcParams['font.size'])",
        "@plt.rc_context({'lines.markersize': 13})\ndef marker_size():\n    return plt.rcParams['lines.markersize']",
        "(marker_size(), plt.rcParams['lines.markersize'])",
        "with plt.rc_context({'no.such.key': 1}):\n    pass",
        "with plt.rc_context({'lines.linewidth': 'thick'}):\n    pass",
        "plt.rcParams['lines.linewidth']",
        "with plt.rc_context({'axes.facecolor': 'y'}), plt.style.context('ggplot'):\n"
        "    nested = (plt.rcParams['axes.facecolor'], plt.rcParams['lines.linewidth'])",
        "(nested, plt.rcParams['axes.facecolor'])",
        "with plt.style.context('dark_background'):\n    with plt.rc_context({'lines.linewidth': 6}):\n"
        "        fig2, ax2 = plt.subplots(figsize=(2, 2))\n        line2, = ax2.plot(x, y)",
        "(line2.get_linewidth(), line2.get_color(), ax2.get_facecolor(), plt.rcParams['axes.facecolor'])",
        "context = plt.rc_context({'lines.linewidth': 2.5})",
        "context.__enter__()",
        "plt.rcParams['lines.linewidth']",
        "context.__exit__(None, None, None)",
        "plt.rcParams['lines.linewidth']",
    ],
    "api_rc_files": [
        "pathlib.Path('custom.rc').write_text(\"# comment\\nlines.linewidth : 5  # trailing\\n"
        "axes.facecolor: '#eeeeee'\\naxes.edgecolor: eeeeee\\nfont.family: serif\\nno.such.key: 1\\n"
        "lines.color: notacolor\\nbad line without colon\\nlines.linewidth: 6\\n"
        "axes.prop_cycle: cycler('color', ['r', 'b'])\\nlines.dashed_pattern: 2, 2\\n"
        "text.latex.preamble: \\\\usepackage{amsmath}\\nlegend.loc: \\\"upper left\\\"\\n\")",
        "matplotlib.rc_file('custom.rc')",
        "[plt.rcParams[key] for key in ('lines.linewidth', 'axes.facecolor', 'axes.edgecolor', 'font.family', "
        "'lines.color', 'axes.prop_cycle', 'lines.dashed_pattern', 'text.latex.preamble', 'legend.loc', "
        "'lines.markersize')]",
        "matplotlib.rcdefaults()",
        "matplotlib.rc_file('custom.rc', use_default_template=False)",
        "(plt.rcParams['lines.linewidth'], plt.rcParams['lines.markersize'])",
        "params = matplotlib.rc_params_from_file('custom.rc', fail_on_error=False, use_default_template=False)",
        "(type(params).__name__, sorted(params), params['lines.linewidth'], params['axes.facecolor'])",
        "matplotlib.rc_params_from_file('custom.rc', fail_on_error=True)",
        "len(matplotlib.rc_params_from_file('custom.rc'))",
        "matplotlib.rc_file('missing.rc')",
        "plt.rcParams['lines.linewidth'] = 11",
        "matplotlib.rc_file_defaults()",
        "plt.rcParams['lines.linewidth']",
        "plt.rcParams['font.size'] = 3",
        "matplotlib.rcdefaults()",
        "(plt.rcParams['font.size'], plt.rcParams['lines.linewidth'], plt.rcParams['axes.facecolor'])",
        "os.path.basename(matplotlib.matplotlib_fname())",
        "pathlib.Path('matplotlibrc').write_text('lines.linewidth: 3\\n')",
        "matplotlib.matplotlib_fname()",
        "matplotlib.rc_params()['lines.linewidth']",
        "os.remove('matplotlibrc')",
        "os.environ['MATPLOTLIBRC'] = os.path.abspath('custom.rc')",
        "matplotlib.matplotlib_fname()",
        "del os.environ['MATPLOTLIBRC']",
        "os.path.basename(matplotlib.matplotlib_fname())",
        "pathlib.Path('style.mplstyle').write_text('lines.linewidth: 8\\nbackend: pdf\\ninteractive: True\\n"
        "lines.marker: s\\n')",
        "matplotlib.rc_file('style.mplstyle')",
        "(plt.rcParams['lines.linewidth'], plt.rcParams['lines.marker'], plt.rcParams['interactive'])",
        "matplotlib.rcdefaults()",
    ],
    "api_rc_function": [
        "matplotlib.rc('lines', linewidth=2, color='r')",
        "(plt.rcParams['lines.linewidth'], plt.rcParams['lines.color'])",
        "matplotlib.rc('font', **{'family': 'serif', 'size': 12})",
        "(plt.rcParams['font.family'], plt.rcParams['font.size'])",
        "matplotlib.rc('lines', lw=3, c='g', ls='--', mew=2, ms=7, aa=False)",
        "[plt.rcParams['lines.' + key] for key in ('linewidth', 'color', 'linestyle', 'markeredgewidth', "
        "'markersize', 'antialiased')]",
        "matplotlib.rc('patch', fc='y', ec='k')",
        "(plt.rcParams['patch.facecolor'], plt.rcParams['patch.edgecolor'])",
        "matplotlib.rc(('xtick', 'ytick'), labelsize=7)",
        "(plt.rcParams['xtick.labelsize'], plt.rcParams['ytick.labelsize'])",
        "plt.rc('axes', grid=True, titlesize='large')",
        "(plt.rcParams['axes.grid'], plt.rcParams['axes.titlesize'])",
        "plt.rc('figure', figsize=(3, 2))",
        "plt.rcParams['figure.figsize']",
        "matplotlib.rc('nope', size=1)",
        "matplotlib.rc('lines', nope=1)",
        "matplotlib.rc('lines', linewidth='thick')",
        "fig, ax = plt.subplots()",
        "line, = ax.plot(x, y)",
        "(line.get_linewidth(), line.get_color(), line.get_linestyle(), line.get_antialiased(), fig.get_size_inches())",
        "plt.rcdefaults()",
        "plt.rcParams['lines.linewidth']",
    ],
    "api_mapping": [
        "plt.rcParams.update({'lines.linewidth': 3, 'font.size': 11})",
        "(plt.rcParams['lines.linewidth'], plt.rcParams['font.size'])",
        "plt.rcParams.update(**{'lines.linewidth': 2})",
        "plt.rcParams['lines.linewidth']",
        "plt.rcParams.update({'lines.markersize': 4, 'lines.linewidth': 'thick'})",
        "(plt.rcParams['lines.markersize'], plt.rcParams['lines.linewidth'])",
        "copied = plt.rcParams.copy()",
        "type(copied).__name__",
        "copied['lines.linewidth'] = 9",
        "(copied['lines.linewidth'], plt.rcParams['lines.linewidth'])",
        "copied['lines.linewidth'] = 'thick'",
        "found = plt.rcParams.find_all('^lines\\\\.dash')",
        "(type(found).__name__, sorted(found), dict(found))",
        "len(plt.rcParams.find_all('nomatch'))",
        "repr(plt.rcParams.find_all('lines.dashed'))",
        "str(plt.rcParams.find_all('lines.dashed'))",
        "(len(plt.rcParams), 'lines.linewidth' in plt.rcParams, 'nope' in plt.rcParams)",
        "list(plt.rcParams)[:12]",
        "sorted(set(plt.rcParams) ^ set(matplotlib.rcsetup._validators))",
        "(plt.rcParams.get('lines.linewidth'), plt.rcParams.get('nope', 'fallback'))",
        "plt.rcParams.setdefault('lines.linewidth', 9)",
        "plt.rcParams.setdefault('nope.key', 1)",
        "plt.rcParams.pop('lines.linewidth')",
        "plt.rcParams.popitem()",
        "del plt.rcParams['lines.linewidth']",
        "plt.rcParams['lines.linewidth']",
        "plt.rcParams['nope.key'] = 1",
        "plt.rcParams['nope.key']",
        "matplotlib.RcParams({'lines.linewidth': 3})['lines.linewidth']",
        "matplotlib.RcParams({'lines.linewidth': 'x'})",
        "dict(matplotlib.RcParams(lines_linewidth=3))",
        "(matplotlib.rcParamsDefault['lines.linewidth'], matplotlib.rcParamsOrig['lines.linewidth'])",
        "plt.rcParams.keys() == matplotlib.rcParamsDefault.keys()",
        "import copy, pickle",
        "dict(pickle.loads(pickle.dumps(plt.rcParams.find_all('^lines\\\\.dash'))))",
        "copy.deepcopy(plt.rcParams)['lines.linewidth']",
        "type(copy.copy(plt.rcParams)).__name__",
        "plt.rcParams == plt.rcParams.copy()",
        "[(key, plt.rcParams[key]) for key in sorted(plt.rcParams) if key.startswith(('axes.', 'lines.'))]",
        "[(key, plt.rcParams[key]) for key in sorted(plt.rcParams) if not key.startswith(('axes.', 'lines.', "
        "'backend'))]",
    ],
    "api_styles": [
        "sorted(plt.style.available)",
        "sorted(plt.style.library)",
        "(type(plt.style.library['ggplot']).__name__, sorted(plt.style.library['ggplot']))",
        "sorted(plt.style.core.STYLE_BLACKLIST)",
        "plt.style.use('ggplot')",
        "(plt.rcParams['axes.facecolor'], plt.rcParams['axes.prop_cycle'])",
        "plt.style.use('default')",
        "plt.rcParams['axes.facecolor']",
        "plt.style.use({'lines.linewidth': 3, 'interactive': True, 'backend': 'agg'})",
        "plt.rcParams['lines.linewidth']",
        "plt.style.use('nope')",
        "plt.style.use('seaborn')",
        "plt.style.use('nonexistent_package.style')",
        "pathlib.Path('mine.mplstyle').write_text('axes.facecolor: 0.8\\nlines.markersize: 11\\ntoolbar: None\\n')",
        "plt.style.use('./mine.mplstyle')",
        "(plt.rcParams['axes.facecolor'], plt.rcParams['lines.markersize'])",
        "plt.style.use(pathlib.Path('mine.mplstyle'))",
        "plt.style.use(['classic', {'lines.linewidth': 5}])",
        "(plt.rcParams['lines.linewidth'], plt.rcParams['axes.facecolor'], plt.rcParams['_internal.classic_mode'])",
        "with plt.style.context('ggplot', after_reset=True):\n    inside = plt.rcParams['lines.linewidth']",
        "(inside, plt.rcParams['lines.linewidth'])",
        "with plt.style.context(['bmh', {'lines.linewidth': 7}]):\n"
        "    bmh = (plt.rcParams['lines.linewidth'], plt.rcParams['axes.facecolor'])",
        "bmh",
        "plt.style.reload_library()",
        "plt.style.use('default')",
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y)",
    ],
    "api_validators": [
        *(statement for key, value in INVALID
          for statement in (f"plt.rcParams[{key!r}] = {value}", f"plt.rcParams[{key!r}]")),
        "from matplotlib import rcsetup",
        "rcsetup.cycler('color', 'rgb')",
        "rcsetup.cycler(c='rgb', lw=[1, 2, 3])",
        "rcsetup.cycler('nope', [1])",
        "rcsetup.cycler('color', 'rgb', 'extra')",
        "rcsetup.validate_hatch('xx')",
        "rcsetup.validate_hatch('?')",
        "rcsetup.validate_fontsize('medium')",
        "rcsetup.validate_fontsize(None)",
        "rcsetup.validate_color('C10')",
        "rcsetup.validate_colorlist('r, g, b')",
        "rcsetup.validate_int('3.5')",
        "rcsetup.validate_bool('Y')",
        "rcsetup.validate_backend('nobackend')",
        "rcsetup.validate_cycler(\"cycler('color', 'rgb').__class__\")",
        "rcsetup.validate_cycler(\"cycler('color', 'rgb') * cycler('lw', [1, 2])\")",
    ],
}


def _available(scene: str) -> bool:
    """Whether the programs ``scene`` needs are on ``PATH``."""
    return all(shutil.which(program) for program in REQUIRES.get(scene, ()))


def unassigned() -> list[str]:
    """matplotlib's rcParams no scene sets and `EXCLUDED` does not list."""
    assigned = {key for params in VALUES.values() for key in params} | set(EXCLUDED)
    return sorted(set(matplotlib.rcParams) - assigned)


def _setting(key: str, value: str) -> list[str]:
    return [f"plt.rcParams[{key!r}] = {value}", f"plt.rcParams[{key!r}]"]


def _scene_programs() -> Iterator[tuple[str, list[str]]]:
    """The programs setting the parameters of each scene."""
    for scene, params in VALUES.items():
        if not _available(scene):
            continue
        keys = list(params)
        chunks = [keys[start:start + CHUNK] for start in range(0, len(keys), CHUNK)]
        for number, chunk in enumerate(chunks):
            for variant in range(max(len(params[key]) for key in chunk)):
                settings = [statement for key in chunk if variant < len(params[key])
                            for statement in _setting(key, params[key][variant])]
                part = f"_{number}" if len(chunks) > 1 else ""
                yield f"{scene}{part}_v{variant}", SETUP + settings + SCENES[scene]


def _style_programs() -> Iterator[tuple[str, list[str]]]:
    """Every style sheet alone, then pairs, on `STYLE_SCENE`."""
    names = sorted(matplotlib.style.library)
    for name in names:
        yield f"style_{name}", SETUP + [
            f"plt.style.use({name!r})",
            f"[(key, plt.rcParams[key]) for key in sorted(plt.style.library[{name!r}])]",
        ] + STYLE_SCENE
    for index in range(0, len(names), 2):
        pair = [names[index], names[(index + PAIR_STEP) % len(names)]]
        yield f"styles_{pair[0]}_{pair[1]}", SETUP + [
            f"plt.style.use({pair!r})",
            f"[(key, plt.rcParams[key]) for key in sorted(set(plt.style.library[{pair[0]!r}]) | "
            f"set(plt.style.library[{pair[1]!r}]))]",
        ] + STYLE_SCENE


def programs() -> Iterator[tuple[str, list[str]]]:
    """Every program: the scenes', the style sheets' and the rc functions'."""
    missing = unassigned()
    if missing:
        raise RuntimeError(f"rcParams neither set by a scene nor excluded: {missing}")
    yield from _scene_programs()
    yield from _style_programs()
    for name, statements in API_PROGRAMS.items():
        yield name, SETUP + statements
