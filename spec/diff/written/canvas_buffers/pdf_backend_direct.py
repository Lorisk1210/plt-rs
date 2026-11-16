# Write PDF and SVG with their canvases directly (FigureCanvasPdf/SVG) and check file headers.
from matplotlib.figure import Figure
from matplotlib.backends.backend_pdf import FigureCanvasPdf
from matplotlib.backends.backend_svg import FigureCanvasSVG

fig = Figure(figsize=(3, 2))
ax = fig.add_subplot()
ax.plot([1, 3, 2])
FigureCanvasPdf(fig).print_pdf("direct.pdf", metadata={"CreationDate": None})
FigureCanvasSVG(fig).print_svg("direct.svg", metadata={"Date": None})
with open("direct.pdf", "rb") as f:
    print(f.read(8))
with open("direct.svg") as f:
    print(f.readline().strip())
print(type(fig.canvas).__name__)
