# Save one figure in a loop over every supported format and a matrix of savefig kwargs; record each output's size class and header.
import io
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "loop"
fig, ax = plt.subplots(figsize=(3, 2))
ax.plot([0, 1, 2], [1, 3, 2])
ax.set_title("formats")
fmts = sorted(fig.canvas.get_supported_filetypes())
print(fmts)
for fmt in fmts:
    buf = io.BytesIO()
    kw = {}
    if fmt == "pdf":
        kw["metadata"] = {"CreationDate": None}
    if fmt in ("svg", "svgz"):
        kw["metadata"] = {"Date": None}
    try:
        fig.savefig(buf, format=fmt, **kw)
        data = buf.getvalue()
        print(fmt, data[:4], len(data) > 200)
    except Exception as e:
        print(fmt, type(e).__name__, str(e)[:50])
for kw in [dict(transparent=True), dict(facecolor="k"), dict(edgecolor="r", facecolor="w"), dict(dpi=30),
           dict(bbox_inches="tight", pad_inches=0.3), dict(pil_kwargs={"optimize": True})]:
    buf = io.BytesIO()
    fig.savefig(buf, format="png", **kw)
    buf.seek(0)
    img = plt.imread(buf)
    print(sorted(kw), img.shape, round(float(img[0, 0, 3]), 3), [round(float(v), 3) for v in img[0, 0, :3]])
print(fig.get_facecolor(), ax.patch.get_facecolor())
