# The same clipped scene saved as SVG, PDF and PS; count clip-path definitions and clipping operators in each document.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Circle, Rectangle

plt.rcParams["svg.hashsalt"] = "clips"
fig, ax = plt.subplots(figsize=(4, 3))
c = Circle((0.5, 0.5), 0.3, transform=ax.transAxes, fc="none")
ax.add_patch(c)
for k in range(5):
    ln, = ax.plot(np.linspace(0, 1, 30), np.linspace(0, 1, 30) ** (k + 1))
    if k % 2:
        ln.set_clip_path(c)
r = Rectangle((0.2, 0.2), 0.6, 0.6, fc="C3", alpha=0.3, clip_on=False)
ax.add_patch(r)
svg = io.BytesIO()
fig.savefig(svg, format="svg", metadata={"Date": None})
s = svg.getvalue().decode()
print(len(re.findall(r"<clipPath id=", s)), len(re.findall(r'clip-path="url\(#', s)))
pdf = io.BytesIO()
fig.savefig(pdf, format="pdf", metadata={"CreationDate": None})
print(pdf.getvalue().count(b" W n") > 0, len(pdf.getvalue()) > 1000)
ps = io.BytesIO()
fig.savefig(ps, format="ps")
txt = ps.getvalue().decode("latin-1")
print(txt.count("clip"), txt.startswith("%!PS-Adobe-3.0"))
eps = io.BytesIO()
fig.savefig(eps, format="eps")
print(eps.getvalue().decode("latin-1").splitlines()[0])
fig.savefig("clipped.png")
