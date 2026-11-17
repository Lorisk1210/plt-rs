# Call canvas print_* methods directly (print_png, print_svg, print_pdf, print_raw, print_rgba, print_jpg) with kwargs; sizes.
import io
from PIL import Image
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "print"
fig, ax = plt.subplots(figsize=(2, 1.5), dpi=50)
ax.plot([0, 1], [1, 0])
canvas = fig.canvas
for meth in ("print_png", "print_raw", "print_rgba", "print_jpg", "print_tif", "print_webp"):
    buf = io.BytesIO()
    try:
        getattr(canvas, meth)(buf)
        data = buf.getvalue()
        if meth in ("print_raw", "print_rgba"):
            print(meth, len(data), data[:4])
        else:
            with Image.open(io.BytesIO(data)) as decoded:
                print(meth, decoded.format, decoded.size, decoded.mode, data[:4])
    except Exception as e:
        print(meth, type(e).__name__, str(e)[:50])
buf = io.BytesIO()
canvas.print_png(buf, metadata={"Software": None, "Title": "direct"})
print(b"Software" in buf.getvalue(), b"direct" in buf.getvalue())
buf = io.BytesIO()
canvas.print_figure(buf, format="svg", metadata={"Date": None})
print(buf.getvalue()[:5], type(fig.canvas).__name__)
buf = io.BytesIO()
canvas.print_figure(buf, format="pdf", dpi=300, metadata={"CreationDate": None})
print(buf.getvalue()[:8], type(fig.canvas).__name__)
print(canvas.get_supported_filetypes_grouped()["Joint Photographic Experts Group"])
print(len(io.BytesIO().getvalue()), fig.dpi)
raw = io.BytesIO()
canvas.print_raw(raw)
w, h = canvas.get_width_height()
print(len(raw.getvalue()) == w * h * 4, (w, h))
