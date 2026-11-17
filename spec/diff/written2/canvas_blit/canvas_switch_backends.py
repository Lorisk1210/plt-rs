# One Figure printed through several canvas classes in turn (Agg, SVG, PDF, PS), checking which canvas the figure keeps.
import io
import matplotlib
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.backends.backend_svg import FigureCanvasSVG
from matplotlib.backends.backend_pdf import FigureCanvasPdf
from matplotlib.backends.backend_ps import FigureCanvasPS

fig = Figure(figsize=(3, 2))
ax = fig.add_subplot()
ax.plot([1, 2, 3], [3, 1, 2])
matplotlib.rcParams["svg.hashsalt"] = "switch"
results = []
for cls, method, kw in [(FigureCanvasAgg, "print_png", {}), (FigureCanvasSVG, "print_svg", {"metadata": {"Date": None}}),
                        (FigureCanvasPdf, "print_pdf", {"metadata": {"CreationDate": None}}), (FigureCanvasPS, "print_ps", {})]:
    canvas = cls(fig)
    buf = io.BytesIO() if method != "print_svg" else io.StringIO()
    getattr(canvas, method)(buf, **kw)
    data = buf.getvalue()
    results.append((cls.__name__, type(fig.canvas).__name__, data[:5] if isinstance(data, bytes) else data[:5]))
for r in results:
    print(r)
canvas = FigureCanvasAgg(fig)
print(type(fig.canvas).__name__, fig.canvas is canvas, canvas.figure is fig)
print(sorted(canvas.get_supported_filetypes())[:6], canvas.get_default_filetype())
print(sorted(FigureCanvasSVG(fig).get_supported_filetypes()), FigureCanvasPdf.filetypes)
fig.savefig("switched.png")
print(type(fig.canvas).__name__)
buf = io.BytesIO()
fig.canvas.print_figure(buf, format="pdf", metadata={"CreationDate": None})
print(buf.getvalue()[:5], type(fig.canvas).__name__)
