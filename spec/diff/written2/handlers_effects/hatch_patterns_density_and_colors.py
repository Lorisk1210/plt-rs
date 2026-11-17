# Hatches at densities and colours set every way (patch.hatch, hatchcolor, rcParams hatch.color/linewidth), saved to PNG/SVG/PDF.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

plt.rcParams.update({"hatch.color": "navy", "hatch.linewidth": 0.8, "svg.hashsalt": "hatches"})
fig, ax = plt.subplots(figsize=(6, 3))
hatches = ["/", "//", "///", "x", "xx", "\\|", "o", "O.", "*", "-+"]
patches = []
for k, h in enumerate(hatches):
    p = ax.add_patch(Rectangle((k, 0), 0.9, 1, fill=False, hatch=h))
    patches.append(p)
patches[3].set_hatchcolor("red")
patches[4].set_edgecolor("green")
patches[5].set_hatch_linewidth(2.5)
ax.set_xlim(0, len(hatches))
ax.set_ylim(0, 1)
print([p.get_hatch() for p in patches])
print([p.get_hatchcolor() for p in patches[2:6]], patches[5].get_hatch_linewidth(), patches[0].get_hatch_linewidth())
svg = io.StringIO()
fig.savefig(svg, format="svg", metadata={"Date": None})
print(len(re.findall(r'<pattern id="h', svg.getvalue())))
pdf = io.BytesIO()
fig.savefig(pdf, format="pdf", metadata={"CreationDate": None})
print(pdf.getvalue().count(b"/PatternType 1"))
fig.savefig("hatches.png")
patches[6].set_hatch("")
patches[7].set_hatch(None)
print(repr(patches[6].get_hatch()), repr(patches[7].get_hatch()))
