"""The scenes of the ``rcparams`` programs: for each group of rcParams
(``rcparams_values.VALUES``), the statements building a figure those
parameters change, drawing it, and reading back what they set (the
artists' values, the text of tick labels, the facts of the files saved),
the figure left open for its picture. Each scene runs after the
parameters are set, and after `SETUP`.
"""

from __future__ import annotations

from . import common

#: The statements every program starts with.
SETUP = common.HEADER + common.DATA + [
    "import io, os, pathlib, re",
    "from matplotlib import patheffects",
]

#: The statements reading the lines of the ``lines`` scene.
_LINE_PROBES = [
    "[(l.get_linestyle(), l.get_linewidth(), l.get_color(), l.get_marker(), l.get_markersize()) for l in ax.lines]",
    "[(l.get_markeredgewidth(), l.get_markeredgecolor(), l.get_markerfacecolor(), l.get_fillstyle()) "
    "for l in ax.lines]",
    "[(l.get_dash_capstyle(), l.get_dash_joinstyle(), l.get_solid_capstyle(), l.get_solid_joinstyle()) "
    "for l in ax.lines]",
    "[(l.get_antialiased(), l.get_sketch_params(), l.get_snap(), l.get_path_effects()) for l in ax.lines]",
]

#: How a scene saves a vector file to read its facts: the format, into a
#: buffer named after it.
_SAVED = "{name} = io.BytesIO()\nfig.savefig({name}, format='{name}')"

#: scene -> its statements.
SCENES: dict[str, list[str]] = {
    "lines": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "line, = ax.plot(x, y, label='solid')",
        "dashed, = ax.plot(x, y - 0.5, '--')",
        "dotted, = ax.plot(x, y + 0.5, ':')",
        "dashdot, = ax.plot(x, -y, '-.')",
        "marked, = ax.plot(x, y / 2, marker='o')",
        "dense = np.linspace(0, 1, 3000)",
        "wiggle, = ax.plot(dense * 6, np.sin(dense * 300) * 0.3)",
        "fig.canvas.draw()",
        *_LINE_PROBES,
        "marked.get_markerfacecoloralt()",
        "wiggle.get_path().vertices.shape",
    ],
    "patches": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "rect = ax.add_patch(patches.Rectangle((0.1, 0.1), 0.3, 0.4, hatch='//'))",
        "circle = ax.add_patch(patches.Circle((0.7, 0.6), 0.2))",
        "poly = ax.add_patch(patches.Polygon([[0.5, 0.1], [0.9, 0.2], [0.7, 0.4]], hatch='xx', fill=False))",
        "bars = ax.bar([0.2, 0.5, 0.8], [0.2, 0.3, 0.1], width=0.1, bottom=0.6, hatch='..')",
        "filled = ax.fill_between(x / 7, y / 5 + 0.1, alpha=0.5)",
        "wedge = ax.add_patch(patches.Wedge((0.3, 0.8), 0.1, 30, 270))",
        "fig.canvas.draw()",
        "[(p.get_facecolor(), p.get_edgecolor(), p.get_linewidth(), p.get_antialiased(), p.get_hatch()) "
        "for p in ax.patches]",
        "[(p.get_hatchcolor(), p.get_hatch_linewidth(), p.get_fill()) for p in ax.patches]",
        "(filled.get_facecolor(), filled.get_edgecolor(), filled.get_linewidth())",
    ],
    "markers": [
        "fig, (ax, ax2, ax3) = plt.subplots(1, 3, figsize=(6, 2.5))",
        "sc = ax.scatter(x, y, c=values, s=values * 10)",
        "plain = ax.scatter(x, y + 1)",
        "eb = ax2.errorbar(x, y, yerr=0.2, xerr=0.1, fmt='o')",
        "counts, edges, hist_patches = ax3.hist(values * 1.1)",
        "fig.canvas.draw()",
        "(sc.get_edgecolor(), plain.get_edgecolor(), plain.get_facecolor(), sc.get_linewidths())",
        "plain.get_paths()[0]",
        "[(c.get_marker(), c.get_markersize(), c.get_markeredgewidth()) for c in eb.lines[1]]",
        "[c.get_linewidth() for c in eb.lines[2]]",
        "(counts, edges, len(hist_patches))",
    ],
    "boxplot": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "data = [np.concatenate([values, [20.0, -8.0]]), values * 2, y + 3]",
        "bp = ax.boxplot(data)",
        "fig.canvas.draw()",
        "{key: [type(artist).__name__ for artist in found] for key, found in bp.items()}",
        "[b.get_path().vertices for b in bp['boxes']]",
        "[(b.get_linewidth(), b.get_linestyle()) for b in bp['boxes']]",
        "[(l.get_color(), l.get_linewidth(), l.get_linestyle(), l.get_xydata()) "
        "for key in ('whiskers', 'caps', 'medians', 'means') for l in bp[key]]",
        "[(l.get_marker(), l.get_markersize(), l.get_markerfacecolor(), l.get_markeredgecolor(), "
        "l.get_markeredgewidth(), l.get_linestyle(), l.get_xydata()) for key in ('fliers', 'means') for l in bp[key]]",
        "(ax.get_xlim(), ax.get_ylim())",
    ],
    "images": [
        "fig, (ax, ax2, ax3) = plt.subplots(1, 3, figsize=(6, 2.5))",
        "im = ax.imshow(z)",
        "im2 = ax.imshow(z[::-1], extent=[0, 2, 0, 2], alpha=0.5)",
        "mesh = ax2.pcolormesh(z)",
        "quads = ax3.pcolor(z)",
        "fig.canvas.draw()",
        "(im.get_cmap().name, im.cmap.N, im.get_interpolation(), im.get_interpolation_stage(), im.get_resample())",
        "(im.origin, ax.get_aspect(), im.get_extent(), ax.get_xlim(), ax.get_ylim())",
        "(mesh.get_cmap().name, mesh.get_snap(), mesh.get_coordinates().shape, len(quads.get_paths()))",
        _SAVED.format(name="svg"),
        "svg.getvalue().count(b'<image')",
    ],
    "contours": [
        "fig, (ax, ax2) = plt.subplots(1, 2, figsize=(5, 2.5))",
        "zz = np.outer(np.linspace(-1, 1, 9), np.linspace(-1, 1, 11)) + np.sin(np.linspace(0, 3, 11))",
        "zm = np.ma.masked_where(zz > 1.2, zz)",
        "cs = ax.contour(zm, levels=[-1, -0.5, 0, 0.5, 1])",
        "cf = ax2.contourf(zm, levels=5)",
        "fig.canvas.draw()",
        "(cs.get_linestyles(), cs.get_linewidths(), cs.levels, cf.levels)",
        "[len(p.vertices) for p in cs.get_paths()]",
        "[len(p.vertices) for p in cf.get_paths()]",
    ],
    "text": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "title = ax.set_title('Title')",
        "label = ax.set_xlabel('x label')",
        "plain = ax.text(0.1, 0.85, 'Plain text, -1')",
        "families = [ax.text(0.1, 0.7 - 0.12 * i, f'{family} AVgf', family=family) "
        "for i, family in enumerate(['serif', 'sans-serif', 'monospace', 'cursive', 'fantasy'])]",
        "formula = ax.text(0.6, 0.5, r'$x^2 + \\alpha$')",
        "missing = ax.text(0.6, 0.2, 'glyph \\u6f22')",
        "fig.canvas.draw()",
        "[(t.get_fontsize(), t.get_fontfamily(), t.get_fontstyle(), t.get_fontvariant(), t.get_fontweight(), "
        "t.get_stretch(), t.get_color(), t.get_antialiased()) for t in [plain, title, label, formula]]",
        "[t.get_fontname() for t in [plain, *families]]",
        "[t.get_window_extent().bounds for t in [plain, title, label, *families, formula, missing]]",
        "(plain.get_language(), formula.get_parse_math())",
    ],
    "mathtext": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "expressions = [r'$\\alpha x^2 \\sum_i^n \\frac{a}{b}$', "
        "r'$\\mathrm{rm}\\ \\mathit{it}\\ \\mathbf{bf}\\ \\mathbfit{bi}$', "
        "r'$\\mathsf{sf}\\ \\mathtt{tt}\\ \\mathcal{CAL}$', r'$ab\\ 12\\ \\sqrt{x}$', r'$\\aleph \\wp \\beth$']",
        "texts = [ax.text(0.05, 0.85 - 0.18 * i, s) for i, s in enumerate(expressions)]",
        "custom = ax.text(0.6, 0.05, r'$\\mathrm{R} \\mathit{I} \\mathbf{B} x$', math_fontfamily='custom')",
        "fig.canvas.draw()",
        "[t.get_window_extent().bounds for t in [*texts, custom]]",
        "[t.get_math_fontfamily() for t in [*texts, custom]]",
    ],
    "legend": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y, 'o-', label='line')",
        "ax.plot(x, -y, label='second')",
        "ax.scatter(x, y / 2, label='points')",
        "ax.bar([1, 2], [0.5, 0.3], label='bars')",
        "leg = ax.legend(title='Legend', ncols=2)",
        "fig.canvas.draw()",
        "frame = leg.get_frame()",
        "(frame.get_facecolor(), frame.get_edgecolor(), frame.get_alpha(), frame.get_linewidth(), "
        "type(frame.get_boxstyle()).__name__, leg.shadow, leg.get_frame_on())",
        "[(t.get_color(), t.get_fontsize()) for t in leg.get_texts()]",
        "leg.get_title().get_fontsize()",
        "(leg.numpoints, leg.scatterpoints, leg.markerscale, leg.borderpad, leg.labelspacing, leg.handlelength, "
        "leg.handleheight, leg.handletextpad, leg.borderaxespad, leg.columnspacing)",
        "leg.get_window_extent().bounds",
        "[type(h).__name__ for h in leg.legend_handles]",
        "(leg.legend_handles[0].get_xdata(), leg.legend_handles[2].get_sizes(), "
        "leg.legend_handles[2].get_offsets())",
    ],
    "axes": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "for i in range(4):\n    ax.plot(x, y + i / 4, label=str(i))",
        "title = ax.set_title('Title')",
        "ax.set_xlabel('x label')",
        "ax.set_ylabel('y label')",
        "fig.canvas.draw()",
        "(ax.get_facecolor(), ax.spines['left'].get_edgecolor(), ax.spines['left'].get_linewidth())",
        "[spine.get_visible() for spine in ax.spines.values()]",
        "(ax.get_axisbelow(), ax.get_xmargin(), ax.get_ymargin(), ax.get_xlim(), ax.get_ylim())",
        "(title.get_fontsize(), title.get_fontweight(), title.get_color(), title.get_position(), "
        "title.get_horizontalalignment())",
        "[ax.get_title(loc) for loc in ('left', 'center', 'right')]",
        "(ax.xaxis.label.get_fontsize(), ax.xaxis.label.get_fontweight(), ax.xaxis.label.get_color(), "
        "ax.xaxis.labelpad, ax.xaxis.label.get_position(), ax.xaxis.label.get_horizontalalignment())",
        "(ax.yaxis.label.get_position(), ax.yaxis.label.get_horizontalalignment(), "
        "ax.yaxis.label.get_verticalalignment())",
        "[(l.get_color(), l.get_linestyle(), l.get_linewidth()) for l in ax.lines]",
        "[t.get_text() for t in ax.get_xticklabels()]",
        "type(ax.xaxis.get_major_locator()).__name__",
    ],
    "formatter": [
        "fig, (ax, ax2, ax3) = plt.subplots(1, 3, figsize=(7, 2.5))",
        "ax.plot([1e-7, 3e-7], [-1e6 - 1, -1e6 + 4])",
        "ax2.plot([0, 1], [12345.5, 12348.5])",
        "ax3.semilogy([1, 2], [1e-3, 1e3])",
        "fig.canvas.draw()",
        "[[t.get_text() for t in labels] for labels in (ax.get_xticklabels(), ax.get_yticklabels(), "
        "ax2.get_yticklabels(), ax3.get_yticklabels())]",
        "[a.get_offset_text().get_text() for a in (ax.xaxis, ax.yaxis, ax2.yaxis)]",
        "formatter = ax.yaxis.get_major_formatter()",
        "(formatter.get_useOffset(), formatter.get_useMathText(), formatter.get_useLocale(), formatter.offset)",
        "(formatter.format_data(-12345.678), formatter.format_data_short(-0.5))",
        "[ticker.LogFormatterSciNotation()(v) for v in (1e-3, 0.01, 1, 100, 1e4)]",
    ],
    "ticks": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y)",
        "ax.set_xlabel('x label')",
        "fig.canvas.draw()",
        "[axis.get_tick_params(which=which) for axis in (ax.xaxis, ax.yaxis) for which in ('major', 'minor')]",
        "[(t.tick1line.get_visible(), t.tick2line.get_visible(), t.label1.get_visible(), t.label2.get_visible()) "
        "for axis in (ax.xaxis, ax.yaxis) for t in axis.get_major_ticks()[:1] + axis.get_minor_ticks()[:1]]",
        "[(t.tick1line.get_markersize(), t.tick1line.get_markeredgewidth(), t.tick1line.get_color(), "
        "t.tick1line.get_marker(), t.get_pad()) for axis in (ax.xaxis, ax.yaxis) "
        "for t in axis.get_major_ticks()[:1] + axis.get_minor_ticks()[:1]]",
        "[(t.label1.get_color(), t.label1.get_fontsize(), t.label1.get_horizontalalignment(), "
        "t.label1.get_verticalalignment()) for axis in (ax.xaxis, ax.yaxis) for t in axis.get_major_ticks()[:1]]",
        "(len(ax.xaxis.get_minorticklocs()), len(ax.yaxis.get_minorticklocs()), "
        "type(ax.xaxis.get_minor_locator()).__name__)",
        "(ax.xaxis.get_ticks_position(), ax.yaxis.get_ticks_position(), ax.xaxis.get_label_position())",
    ],
    "grid": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y)",
        "ax.minorticks_on()",
        "fig.canvas.draw()",
        "def gridlines():\n    return [(t.gridline.get_visible(), t.gridline.get_color(), "
        "t.gridline.get_linestyle(), t.gridline.get_linewidth(), t.gridline.get_alpha()) "
        "for axis in (ax.xaxis, ax.yaxis) for t in axis.get_major_ticks()[:1] + axis.get_minor_ticks()[:1]]",
        "gridlines()",
        "ax.grid(True, which='both')",
        "fig.canvas.draw()",
        "gridlines()",
    ],
    "figure": [
        "fig = plt.figure()",
        "axs = fig.subplots(2, 2)",
        "title = fig.suptitle('Suptitle')",
        "xlabel = fig.supxlabel('supx')",
        "ylabel = fig.supylabel('supy')",
        "fig.canvas.draw()",
        "(fig.get_size_inches(), fig.get_dpi(), fig.get_facecolor(), fig.get_edgecolor(), fig.get_frameon())",
        "vars(fig.subplotpars)",
        "[a.get_position().bounds for a in axs.flat]",
        "(title.get_fontsize(), title.get_fontweight(), xlabel.get_fontsize(), xlabel.get_fontweight(), "
        "ylabel.get_fontsize())",
        "(fig.bbox.bounds, fig.canvas.get_width_height())",
    ],
    "layout": [
        "fig, axs = plt.subplots(2, 2)",
        "for i, a in enumerate(axs.flat):\n    a.set_title(f'Axes {i}')\n    a.set_ylabel('long y label')\n"
        "    a.plot(x, y * 10 ** i)",
        "fig.canvas.draw()",
        "(type(fig.get_layout_engine()).__name__, fig.get_tight_layout(), fig.get_constrained_layout())",
        "fig.get_layout_engine().get() if fig.get_layout_engine() is not None else None",
        "[a.get_position().bounds for a in axs.flat]",
    ],
    "dates": [
        "from matplotlib import dates as mdates",
        "fig, axs = plt.subplots(7, 1, figsize=(5, 9))",
        "start = np.datetime64('2021-03-04T05:06:07')",
        "spans = [np.timedelta64(3650, 'D'), np.timedelta64(300, 'D'), np.timedelta64(20, 'D'), "
        "np.timedelta64(30, 'h'), np.timedelta64(50, 'm'), np.timedelta64(50, 's'), np.timedelta64(500, 'ms')]",
        "for a, span in zip(axs, spans):\n    a.plot([start, start + span], [0, 1])",
        "fig.canvas.draw()",
        "[[t.get_text() for t in a.get_xticklabels()] for a in axs]",
        "[a.xaxis.get_offset_text().get_text() for a in axs]",
        "[type(a.xaxis.get_major_formatter()).__name__ for a in axs]",
        "[a.get_xlim() for a in axs]",
        "(mdates.get_epoch(), mdates.date2num(np.datetime64('2024-01-01')))",
        "(mdates.num2date(19000.25).isoformat(), mdates.DateFormatter('%H:%M')(19000.25))",
    ],
    "polar": [
        "fig = plt.figure(figsize=(3, 3))",
        "ax = fig.add_subplot(projection='polar')",
        "ax.plot(x, values)",
        "fig.canvas.draw()",
        "[[t.gridline.get_visible() for t in axis.get_major_ticks()] for axis in (ax.xaxis, ax.yaxis)]",
    ],
    "three_d": [
        "from matplotlib.backend_bases import MouseEvent",
        "fig = plt.figure(figsize=(4, 3))",
        "ax = fig.add_subplot(projection='3d')",
        "surface = ax.plot_surface(*np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-1, 1, 5)), z)",
        "points = ax.scatter(x / 3, y, values / 9, c=values)",
        "fig.canvas.draw()",
        "[axis.pane.get_facecolor() for axis in (ax.xaxis, ax.yaxis, ax.zaxis)]",
        "(ax.get_xlim(), ax.get_ylim(), ax.get_zlim())",
        "points.get_facecolor()[:3]",
        "cx, cy = ax.bbox.x0 + ax.bbox.width / 2, ax.bbox.y0 + ax.bbox.height / 2",
        "fig.canvas.callbacks.process('button_press_event', "
        "MouseEvent('button_press_event', fig.canvas, cx, cy, button=1))",
        "fig.canvas.callbacks.process('motion_notify_event', "
        "MouseEvent('motion_notify_event', fig.canvas, cx + 30, cy + 20, button=1))",
        "fig.canvas.callbacks.process('button_release_event', "
        "MouseEvent('button_release_event', fig.canvas, cx + 30, cy + 20, button=1))",
        "(ax.elev, ax.azim, ax.roll)",
    ],
    "savefig": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y)",
        "ax.text(1, 0, 'text')",
        "fig.savefig('saved')",
        "sorted(os.listdir())",
        "png = io.BytesIO()",
        "fig.savefig(png, format='png')",
        "png.seek(0)",
        "(plt.imread(png).shape, plt.imread(io.BytesIO(png.getvalue()))[0, 0])",
        _SAVED.format(name="eps"),
        "[line.decode() for line in eps.getvalue().splitlines() "
        "if line.startswith((b'%%BoundingBox', b'%%Orientation', b'%%DocumentPaperSizes'))]",
        _SAVED.format(name="svg"),
        "re.findall(rb'<svg[^>]*>', svg.getvalue())[0].decode()",
    ],
    "pdf": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y, color='C1')",
        "ax.set_title('Title text')",
        "ax.text(1, 0, 'Text AV')",
        _SAVED.format(name="pdf"),
        "(b'/FlateDecode' in pdf.getvalue(), b'/FontFile2' in pdf.getvalue(), b'/Type3' in pdf.getvalue())",
        "sorted({m.decode() for m in re.findall(rb'/BaseFont /([\\w+-]+)', pdf.getvalue())})",
        "sorted({m.decode() for m in re.findall(rb'/Subtype /(\\w+)', pdf.getvalue())})",
        "(pdf.getvalue().count(b' rg'), pdf.getvalue().count(b' RG'))",
    ],
    "ps": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y)",
        "ax.set_title('Title text')",
        _SAVED.format(name="ps"),
        "[line.decode() for line in ps.getvalue().splitlines() "
        "if line.startswith((b'%%BoundingBox', b'%%DocumentPaperSizes', b'%%Orientation', b'%!PS'))]",
        "(b'/FontType 42' in ps.getvalue(), b'/FontType 3' in ps.getvalue(), b'Helvetica' in ps.getvalue())",
        _SAVED.format(name="eps"),
        "[line.decode() for line in eps.getvalue().splitlines() if line.startswith((b'%%BoundingBox', b'%!PS'))]",
    ],
    "svg": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y)",
        "ax.set_title('Title text')",
        "ax.imshow(z, extent=[0, 6, -1, 1])",
        "fig.savefig('figure.svg')",
        "sorted(os.listdir())",
        "(pathlib.Path('figure.svg').read_text().count('<text'), pathlib.Path('figure.svg').read_text().count('<use'))",
        "re.findall(r'<svg[^>]*>', pathlib.Path('figure.svg').read_text())[0]",
        "re.findall(r' id=\"([^\"]+)\"', pathlib.Path('figure.svg').read_text())[:8]",
        "re.findall(r'<image[^>]*xlink:href=\"([^\"]{0,30})', pathlib.Path('figure.svg').read_text())",
    ],
    "pgf": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y)",
        "ax.set_title('Title text')",
        "fig.savefig('figure.pgf')",
        "pgf = pathlib.Path('figure.pgf').read_text()",
        "(pgf.count('\\\\pgftext'), '\\\\usepackage{amsmath}' in pgf, 'fontspec' in pgf, '\\\\setmainfont' in pgf)",
    ],
    "usetex": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y)",
        "title = ax.set_title(r'$\\alpha + \\beta$ TeX')",
        "label = ax.set_xlabel('x label')",
        "fig.canvas.draw()",
        "(title.get_usetex(), label.get_usetex())",
        "[t.get_window_extent().bounds for t in (title, label)]",
        "[t.get_text() for t in ax.get_xticklabels()]",
    ],
    "animation": [
        "from matplotlib import animation",
        "fig, ax = plt.subplots(figsize=(2, 1.5))",
        "line, = ax.plot(x, y)",
        "anim = animation.FuncAnimation(fig, lambda i: line.set_ydata(np.sin(x + i)), frames=2)",
        "(animation.writers.is_available('ffmpeg'), animation.writers.is_available('imagemagick'))",
        "(animation.FFMpegWriter.bin_path(), animation.ImageMagickWriter.bin_path())",
        "(animation.FFMpegWriter().codec, animation.FFMpegWriter().bitrate, animation.HTMLWriter().frame_format)",
        "anim.save('anim.gif')",
        "anim.save('anim.html')",
        "sorted(os.listdir())",
        "sorted(os.listdir('anim_frames')) if os.path.isdir('anim_frames') else None",
        "type(anim._repr_html_()).__name__",
    ],
    "keymap": [
        "from matplotlib.backend_bases import KeyEvent, NavigationToolbar2, key_press_handler",
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, values)",
        "fig.canvas.draw()",
        "toolbar = NavigationToolbar2(fig.canvas)",
        "toolbar.push_current()",
        "ax.set_xlim(1, 3)",
        "toolbar.push_current()",
        "def press(key):\n    key_press_handler(KeyEvent('key_press_event', fig.canvas, key, ax.bbox.x0 + 10, "
        "ax.bbox.y0 + 10))\n    return (ax.xaxis.get_major_ticks()[0].gridline.get_visible(), "
        "ax.yaxis.get_major_ticks()[0].gridline.get_visible(), ax.get_xscale(), ax.get_yscale(), ax.get_xlim(), "
        "str(toolbar.mode), plt.fignum_exists(fig.number))",
        *(f"press({key!r})" for key in (
            "g", "x", "G", "X", "z", "l", "k", "L", "a", "b", "h", "u", "left", "n", "right", "m", "p", "w", "o", "e",
            "s", "j", "f", "t", "y", "q")),
        "fig2, ax2 = plt.subplots()",
        "press('W')",
        "press('Q')",
        "plt.get_fignums()",
    ],
    "pyplot": [
        "fig = plt.figure(figsize=(2, 2))",
        "others = [plt.figure(figsize=(1, 1)) for _ in range(3)]",
        "plt.isinteractive()",
        "plt.plot(x, y)",
        "plt.get_fignums()",
        "matplotlib.is_interactive()",
    ],
}

# The PostScript distillers run on the ``ps`` scene.
SCENES["distiller"] = SCENES["ps"]

#: The programs a scene needs on ``PATH``: it is only generated where they
#: are.
REQUIRES: dict[str, tuple[str, ...]] = {
    "usetex": ("latex", "dvipng"),
    "pgf": ("xelatex", "pdflatex", "lualatex"),
    "distiller": ("gs", "ps2pdf", "pdftops"),
}
