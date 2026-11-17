# Figures built with matplotlib.figure.Figure (no canvas given), saved straight away; canvas and manager state inspected.
import io
from matplotlib.figure import Figure

fig = Figure(figsize=(3, 2), dpi=90)
print(type(fig.canvas).__name__, fig.canvas.manager)
ax = fig.add_subplot()
ax.scatter([1, 2, 3], [1, 4, 9], c=[0.1, 0.5, 0.9])
fig.savefig("bare.png")
buf = io.BytesIO()
fig.savefig(buf, format="png")
print(buf.getvalue()[:4], type(fig.canvas).__name__)
for fmt in ("svg", "pdf", "eps", "jpg", "webp", "tiff"):
    b = io.BytesIO()
    kwargs = {"svg": {"metadata": {"Date": None}}, "pdf": {"metadata": {"CreationDate": None}}}.get(fmt, {})
    fig.savefig(b, format=fmt, **kwargs)
    print(fmt, len(b.getvalue()) > 100, type(fig.canvas).__name__)
fig2 = Figure()
print(fig2.canvas.get_width_height(), fig2.dpi, fig2.get_size_inches().tolist())
print(fig.canvas.get_window_title() if hasattr(fig.canvas, "get_window_title") else "no title api")
fig.savefig("bare.pdf", metadata={"CreationDate": None})
print(fig.canvas.get_width_height(), fig.get_size_inches().tolist())
fig.set_canvas(fig.canvas)
print(fig.canvas.figure is fig)
for dpi in (30, 60):
    b = io.BytesIO(); fig.savefig(b, format="png", dpi=dpi)
    from PIL import Image
    print(dpi, Image.open(io.BytesIO(b.getvalue())).size)
fig2.savefig("empty.png")
print(type(fig2.canvas).__name__, fig2.stale)
