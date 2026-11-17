# Copy a chart into a new figure by hand (lines, patches, texts, limits, labels, ticks) as people do without pickle; compare PNGs.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def clone_axes(src, dst):
    for ln in src.get_lines():
        dst.plot(*ln.get_data(), color=ln.get_color(), lw=ln.get_linewidth(), ls=ln.get_linestyle(),
                 marker=ln.get_marker(), label=ln.get_label())
    for p in src.patches:
        dst.add_patch(plt.Rectangle(p.get_xy(), p.get_width(), p.get_height(), fc=p.get_facecolor(),
                                    ec=p.get_edgecolor(), lw=p.get_linewidth()))
    for t in src.texts:
        dst.text(*t.get_position(), t.get_text(), fontsize=t.get_fontsize(), color=t.get_color())
    dst.set_xlim(src.get_xlim())
    dst.set_ylim(src.get_ylim())
    dst.set_title(src.get_title())
    dst.set_xlabel(src.get_xlabel())
    dst.set_ylabel(src.get_ylabel())
    dst.set_xticks(src.get_xticks())
    dst.set_yticks(src.get_yticks())
    if src.get_legend():
        dst.legend(loc=src.get_legend()._loc)


fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
ax.plot(np.arange(5), np.arange(5) % 3, "o-", label="saw")
ax.bar([1, 3], [2, 1], color="C1")
ax.text(0.5, 2, "note", fontsize=12, color="g")
ax.set(title="original", xlabel="x", ylabel="y")
ax.legend(loc="upper left")
fig2, ax2 = plt.subplots(figsize=(4, 3), dpi=60)
clone_axes(ax, ax2)
a, b = io.BytesIO(), io.BytesIO()
fig.savefig(a, format="png")
fig2.savefig(b, format="png")
a.seek(0)
b.seek(0)
ia, ib = plt.imread(a), plt.imread(b)
print(ia.shape == ib.shape, round(float(np.abs(ia - ib).mean()), 5), int((np.abs(ia - ib).sum(axis=2) > 0.1).sum()))
print(ax2.get_legend()._loc, len(ax2.patches), len(ax.patches), ax2.get_xticks().tolist())
