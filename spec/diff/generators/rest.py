"""The public callables and parameters no table-driven program reaches,
each called where it has what it needs: the fonts of a DVI file and the
PDF writer's font dictionaries, the buffers of FreeType's objects, the
SVG writer's stream, the testing helpers, an animation saved with
Pillow, the printers' Pillow options, a style file, the projection
registry and the deprecated parameters of the polar transforms.
"""

from __future__ import annotations

from typing import Iterator

from . import common

#: The statements every program starts with.
SETUP = common.HEADER + common.DATA + [
    "import io, os, pathlib, subprocess, sys",
    "from matplotlib import font_manager",
    "fig, ax = plt.subplots()",
    "line, = ax.plot(x, y, label='l')",
]

#: program name -> statements after `SETUP`
PROGRAMS: dict[str, list[str]] = {
    "dvi_fonts": [
        "from matplotlib import dviread, texmanager",
        "from matplotlib.backends import backend_pdf",
        "dvi_path = texmanager.TexManager.make_dvi('ab', 12)",
        "dvi = dviread.Dvi(dvi_path, 72)",
        "page = next(iter(dvi))",
        "font = page.text[0].font",
        "font.texname",
        "font.widths",
        "font.size",
        "dviread.DviFont(font.size, font._metrics, font.texname, None).texname",
        "dviread.DviFont(scale=2.0, metrics=font._metrics, texname=b'cmr10', vf=None).size",
        "xetex = dviread.DviFont.from_xetex(10.0, os.fsencode(font_manager.findfont('DejaVu Sans')), 0, {})",
        "xetex.texname[-12:]",
        "tfm = dviread.Tfm(dviread.find_tex_file('cmr10.tfm'))",
        "sorted(tfm.width.items())[:3]",
        "sorted(tfm.height.items())[:3]",
        "sorted(tfm.depth.items())[:3]",
        "fonts = dviread.PsfontsMap(dviread.find_tex_file('pdftex.map'))",
        "fonts.__getitem__(b'cmr10').psname",
        "fonts.__getitem__(b'no-such-font')",
        "vf = dviread.Vf(dviread.find_tex_file('ptmr8t.vf'))",
        "vf.__getitem__(65).width",
        "vf.__getitem__(-1)",
        "metrics = dviread.TexMetrics(tex_width=1, tex_height=2, tex_depth=3)",
        "metrics.__setattr__('tex_width', 5)",
        "metrics.__delattr__('tex_depth')",
        "pdf = backend_pdf.PdfFile(io.BytesIO())",
        "pdf.newPage(4, 3)",
        "pdf.dviFontName(font)",
        "pdf.dviFontName(font)",
        "from matplotlib import _type1font",
        "t1 = _type1font.Type1Font(dviread.find_tex_file('cmr10.pfb'))",
        "pdf.createType1Descriptor(t1)",
        "pdf.hatchPatterns",
        "pdf.finalize()",
        "dvi.close()",
    ],
    "font_buffers": [
        "from matplotlib import ft2font",
        "face = ft2font.FT2Font(font_manager.findfont('DejaVu Sans'))",
        "face.set_size(12, 72)",
        "face.set_text('Ab', 0.0)",
        "face.draw_glyphs_to_bitmap()",
        "view = memoryview(face)",
        "view.shape",
        "face.__release_buffer__(view)",
        "image = ft2font.FT2Image(8, 4)",
        "pixels = memoryview(image)",
        "pixels.shape",
        "image.__release_buffer__(pixels)",
    ],
    "svg_writer": [
        "from matplotlib.backends import backend_svg",
        "out = io.StringIO()",
        "writer = backend_svg.XMLWriter(out)",
        "writer.start('svg', width='4')",
        "writer.data('text')",
        "writer.flush()",
        "out.getvalue()",
        "writer.end('svg')",
        "writer.flush()",
        "backend_svg._XMLWriter.flush(writer)",
        "out.getvalue()",
    ],
    "testing_helpers": [
        "from matplotlib import testing",
        "from matplotlib.testing import compare, decorators",
        "done = testing.subprocess_run_for_testing([sys.executable, '-c', 'print(1)'], env={**os.environ}, "
        "stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, text=True)",
        "done.stdout",
        "done = testing.subprocess_run_for_testing([sys.executable, '-c', 'print(2)'], capture_output=True, "
        "check=False, text=True)",
        "done.stdout",
        "testing.subprocess_run_for_testing([sys.executable, '-c', 'raise SystemExit(3)'], check=True)",
        "done = testing.subprocess_run_helper(matplotlib.get_cachedir, timeout=60, extra_env={'MPLBACKEND': 'agg'})",
        "done.returncode",
        "first, second = pathlib.Path('first.png'), pathlib.Path('second.png')",
        "fig.savefig(first)",
        "ax.plot([0, 6], [1, -1])",
        "fig.savefig(second)",
        "compare.compare_images(str(first), str(first), tol=0, in_decorator=True)",
        "type(compare.compare_images(str(first), str(second), tol=0, in_decorator=True))",
        "sorted(compare.compare_images(str(first), str(second), tol=0, in_decorator=True))",
        "callable(decorators.image_comparison(['same'], extensions=['png'], freetype_version=('2.6', '2.14'), "
        "remove_text=True, savefig_kwarg={'dpi': 50}))",
        "testing.ipython_in_subprocess('agg', {(8, 24): 'agg'})",
    ],
    "animation_saved": [
        "from matplotlib import animation",
        "anim = animation.FuncAnimation(fig, lambda i: line.set_ydata(np.sin(x + i)), frames=3)",
        "steps = []",
        "anim.save('moving.gif', writer='pillow', fps=2, codec=None, bitrate=-1, extra_args=None, "
        "extra_anim=[], progress_callback=lambda i, n: steps.append((i, n)))",
        "steps",
        "pathlib.Path('moving.gif').stat().st_size > 0",
        "anim.save('moving.gif', writer='pillow', extra_anim=[anim])",
        "anim.to_html5_video(embed_limit=0.001)",
    ],
    "printers": [
        "fig.canvas.print_jpeg(io.BytesIO(), pil_kwargs={'quality': 50})",
        "fig.canvas.print_tiff(io.BytesIO(), pil_kwargs={'compression': 'tiff_lzw'})",
        "fig.canvas.print_tif(io.BytesIO(), pil_kwargs={})",
        "fig.canvas.print_webp(io.BytesIO(), pil_kwargs={'lossless': True})",
        "fig.canvas.print_gif(io.BytesIO(), pil_kwargs={'optimize': True})",
        "fig.canvas.print_jpeg(io.BytesIO(), pil_kwargs={'quality': 'best'})",
        "from matplotlib.backends import backend_pdf, backend_svg",
        "backend_pdf.FigureCanvasPdf(fig).print_pdf(io.BytesIO(), bbox_inches_restore=None)",
        "backend_svg.FigureCanvasSVG(fig).print_svg(io.StringIO(), bbox_inches_restore=None)",
        "fig.canvas.print_figure(io.BytesIO(), format='pdf', backend='pdf')",
        "fig.canvas.print_figure(io.BytesIO(), format='png', backend='nope')",
    ],
    "style_file": [
        "style = pathlib.Path('lines.mplstyle')",
        "style.write_text('lines.linewidth: 4\\naxes.facecolor: y\\n')",
        "matplotlib.rc_file(style, use_default_template=False)",
        "plt.rcParams['lines.linewidth']",
        "plt.rcParams['lines.markersize']",
        "matplotlib.rc_file(style, use_default_template=True)",
        "plt.rcParams['lines.markersize']",
        "plt.rcParams.__setitem__('lines.linewidth', 2)",
        "plt.rcParams.__getitem__('lines.linewidth')",
        "plt.rcParams.__setitem__('lines.linewidth', 'thick')",
        "plt.rcParams.__getitem__('no.such.key')",
        "matplotlib.rcdefaults()",
    ],
    "projection_registry": [
        "from matplotlib import projections",
        "from matplotlib.projections import polar",
        "projections.projection_registry.get_projection_class('polar', _error_cls=KeyError)",
        "projections.projection_registry.get_projection_class('nope', _error_cls=KeyError)",
        "projections.projection_registry.get_projection_class('nope')",
        "polar.InvertedPolarTransform(apply_theta_transforms=False).transform([[0.5, 0.5]])",
        "polar.PolarAxes.InvertedPolarTransform(apply_theta_transforms=True)",
        "polar.PolarTransform(apply_theta_transforms=False).transform([[0.5, 1.0]])",
    ],
}


def programs() -> Iterator[tuple[str, list[str]]]:
    """The programs, each after `SETUP`."""
    for name, statements in PROGRAMS.items():
        yield name, SETUP + statements
