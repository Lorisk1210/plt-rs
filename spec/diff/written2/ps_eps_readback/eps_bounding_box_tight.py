# EPS bounding boxes for figure sizes, bbox_inches tight, and content outside the axes; compare %%BoundingBox and HiResBoundingBox.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def bbox(fig, **kw):
    buf = io.BytesIO()
    fig.savefig(buf, format="eps", **kw)
    text = buf.getvalue().decode("latin-1")
    bb = re.search(r"%%BoundingBox: ([-\d ]+)", text).group(1)
    hi = re.search(r"%%HiResBoundingBox: ([-\d. ]+)", text)
    return bb, None if hi is None else [round(float(v), 2) for v in hi.group(1).split()], text.splitlines()[0]


for size in [(2, 2), (4, 3), (6.5, 2.25)]:
    fig, ax = plt.subplots(figsize=size)
    ax.plot([0, 1])
    print(size, bbox(fig))
    ax.set_title("a title that is fairly long for this figure")
    ax.text(1.3, 0.5, "beyond", transform=ax.transAxes)
    print(size, bbox(fig, bbox_inches="tight"))
    print(size, bbox(fig, bbox_inches="tight", pad_inches=0))
    plt.close(fig)
fig = plt.figure(figsize=(3, 3))
fig.text(0.5, 0.5, "only text", ha="center")
print(bbox(fig, bbox_inches="tight"))
print(bbox(fig, orientation="landscape"))
fig.savefig("only_text.eps")
with open("only_text.eps", "rb") as f:
    print(f.read(23))
