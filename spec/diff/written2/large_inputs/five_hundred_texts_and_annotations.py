# 500 texts and 100 annotations with arrows in one axes; extents of a sample, tight bbox of all, and SVG text element count.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(8, 6), dpi=60)
ax.set_xlim(0, 50)
ax.set_ylim(0, 50)
texts = [ax.text((k * 7) % 50, (k * 13) % 50, f"t{k}", fontsize=6 + k % 5, rotation=(k * 17) % 90) for k in range(500)]
anns = [ax.annotate(f"a{k}", ((k * 3) % 50, (k * 11) % 50), ((k * 3 + 5) % 50, (k * 11 + 5) % 50),
                    arrowprops=dict(arrowstyle="->", lw=0.5), fontsize=6) for k in range(100)]
fig.canvas.draw()
r = fig.canvas.get_renderer()
print(len(ax.texts), [[round(v, 1) for v in t.get_window_extent(r).bounds] for t in texts[:3]])
print([[round(v, 1) for v in a.get_window_extent(r).bounds] for a in anns[:2]])
tb = ax.get_tightbbox(r)
print([round(v, 1) for v in tb.bounds])
with plt.rc_context({"svg.fonttype": "none", "svg.hashsalt": "texts"}):
    buf = io.BytesIO()
    fig.savefig(buf, format="svg", metadata={"Date": None})
print(buf.getvalue().count(b"<text"))
fig.savefig("texts.png")
for t in texts[::2]:
    t.remove()
fig.canvas.draw()
print(len(ax.texts))
for a in anns[:50]:
    a.set_visible(False)
fig.canvas.draw()
print(sum(a.get_visible() for a in anns), [round(v, 1) for v in ax.get_tightbbox(r).bounds])
