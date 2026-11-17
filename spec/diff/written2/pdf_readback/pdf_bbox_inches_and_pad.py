# savefig to PDF with bbox_inches tight/explicit Bbox, pad_inches and facecolor; MediaBox sizes in points for each.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Bbox

fig, ax = plt.subplots(figsize=(5, 4))
ax.plot([0, 1], [0, 1])
ax.set_title("A title")
ax.set_xlabel("x")
ax.text(1.2, 0.5, "outside", transform=ax.transAxes)


def mediabox(**kw):
    buf = io.BytesIO()
    fig.savefig(buf, format="pdf", metadata={"CreationDate": None}, **kw)
    m = re.search(rb"/MediaBox \[\s*([-0-9. ]+)\]", buf.getvalue())
    return [round(float(v), 2) for v in m.group(1).split()]


print(mediabox())
print(mediabox(bbox_inches="tight"))
print(mediabox(bbox_inches="tight", pad_inches=0))
print(mediabox(bbox_inches="tight", pad_inches=0.5))
print(mediabox(bbox_inches=Bbox.from_bounds(1, 1, 2, 1.5)))
print(mediabox(bbox_inches="tight", bbox_extra_artists=[]))
print(mediabox(dpi=300))
print(mediabox(bbox_inches="tight", pad_inches="layout"))
buf = io.BytesIO()
fig.savefig(buf, format="pdf", facecolor="lightgrey", metadata={"CreationDate": None})
print(b"0.827451 0.827451 0.827451 rg" in buf.getvalue() or b"0.827 0.827 0.827" in buf.getvalue())
print(fig.get_facecolor(), plt.rcParams["savefig.facecolor"])
