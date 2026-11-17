# FigureCanvasPS used directly without pyplot: print_ps / print_eps to files and streams, dpi handling and the text stream case.
import io
import re
import matplotlib
from matplotlib.figure import Figure
from matplotlib.backends.backend_ps import FigureCanvasPS

fig = Figure(figsize=(3, 2), dpi=120)
canvas = FigureCanvasPS(fig)
ax = fig.add_subplot()
ax.plot([0, 1, 2], [1, 0, 1], "o-")
ax.set_title("no pyplot")
print(type(fig.canvas).__name__, canvas.get_default_filetype(), sorted(canvas.get_supported_filetypes()))
canvas.print_ps("direct.ps")
canvas.print_eps("direct.eps")
text = io.StringIO()
fig.savefig(text, format="ps")
binary = io.BytesIO()
fig.savefig(binary, format="ps")
with open("direct.ps", "rb") as f:
    data = f.read()
print(data[:11], len(data) > 1000, text.getvalue()[:11])
strip = lambda b: re.sub(rb"%%CreationDate: [^\n]*\n", b"", b)
print(strip(data) == strip(binary.getvalue()), strip(text.getvalue().encode("latin-1")) == strip(binary.getvalue()))
with open("direct.eps", "rb") as f:
    eps = f.read()
print(eps.splitlines()[0], re.search(rb"%%BoundingBox: ([-\d ]+)", eps).group(1))
print(fig.dpi)
fig.savefig("dpi.ps", dpi=300)
print(fig.dpi, re.search(rb"%%BoundingBox: ([-\d ]+)", open("dpi.ps", "rb").read()).group(1))
fig.savefig("via_format.eps", format="eps")
print(open("via_format.eps", "rb").read(23))
try:
    canvas.print_ps("bad.ps", papertype="nonsense")
except ValueError as e:
    print("ValueError", str(e)[:50])
