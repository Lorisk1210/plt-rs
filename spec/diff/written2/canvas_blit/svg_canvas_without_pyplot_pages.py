# Build several Figures without pyplot and write them into one PDF with PdfPages and one SVG each; count pages and ids.
import io
import re
from matplotlib.figure import Figure
from matplotlib.backends.backend_pdf import PdfPages

import matplotlib as mpl
mpl.rcParams["svg.hashsalt"] = "pages"
figs = []
for k in range(3):
    fig = Figure(figsize=(3, 2))
    ax = fig.add_subplot()
    ax.plot(range(k + 3), [i ** 0.5 for i in range(k + 3)], label=f"series {k}")
    ax.legend()
    figs.append(fig)
with PdfPages("pages.pdf", metadata={"CreationDate": None}) as pdf:
    for fig in figs:
        pdf.savefig(fig)
    count = pdf.get_pagecount()
print(count)
for k, fig in enumerate(figs):
    buf = io.StringIO()
    fig.savefig(buf, format="svg", metadata={"Date": None})
    svg = buf.getvalue()
    print(k, len(re.findall(r'id="line2d_\d+"', svg)), len(re.findall(r'id="legend_\d+"', svg)), 'id="figure_1"' in svg)
with open("pages.pdf", "rb") as f:
    data = f.read()
print(data.count(b"/Type /Page\n") + data.count(b"/Type /Page "), data[:8])
for fig in figs:
    fig.clear()
print([len(f.axes) for f in figs])
