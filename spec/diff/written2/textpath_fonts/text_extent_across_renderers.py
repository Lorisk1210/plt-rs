# get_text_width_height_descent for one string on the Agg, SVG, PDF and PS renderers and on TextToPath, plain and mathtext.
import io
import matplotlib
matplotlib.use("agg")
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import RendererAgg
from matplotlib.backends.backend_svg import RendererSVG
from matplotlib.backends.backend_pdf import RendererPdf, PdfFile
from matplotlib.backends.backend_ps import RendererPS
from matplotlib.font_manager import FontProperties
from matplotlib.textpath import TextToPath

fp = FontProperties(family="DejaVu Sans", size=12)
strings = [("Hello", False), ("gjpqy", False), (r"$x^2+\sqrt{y}$", True), ("", False), ("two\nlines", False)]
renderers = {
    "agg": RendererAgg(400, 300, 72),
    "svg": RendererSVG(400, 300, io.StringIO()),
    "pdf": RendererPdf(PdfFile(io.BytesIO()), 72, 300, 400),
    "ps": RendererPS(400, 300, io.StringIO()),
}
for name, r in renderers.items():
    row = []
    for s, ismath in strings:
        w, h, d = r.get_text_width_height_descent(s, fp, ismath=ismath)
        row.append((round(w, 2), round(h, 2), round(d, 2)))
    print(name, row)
ttp = TextToPath()
print("path", [tuple(round(v, 2) for v in ttp.get_text_width_height_descent(s, fp, ismath=m)) for s, m in strings])
print(renderers["agg"].points_to_pixels(12), renderers["svg"].points_to_pixels(12), renderers["pdf"].points_to_pixels(12))
print(renderers["agg"].get_canvas_width_height(), renderers["svg"].get_canvas_width_height(),
      renderers["ps"].get_canvas_width_height())
print([r.flipy() for r in renderers.values()], [r.option_image_nocomposite() for r in renderers.values()])
fig = Figure(figsize=(4, 3), dpi=72)
t = fig.text(0.5, 0.5, "Hello", fontproperties=fp)
print([round(v, 2) for v in t.get_window_extent(renderers["agg"]).bounds])
print(type(renderers["svg"]._text2path).__name__, renderers["agg"].dpi)
