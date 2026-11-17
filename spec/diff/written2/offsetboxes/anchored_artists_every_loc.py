# AnchoredText, AnchoredSizeBar and AnchoredDirectionArrows at every legend location code; window extents per location.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import AnchoredText
from mpl_toolkits.axes_grid1.anchored_artists import AnchoredSizeBar, AnchoredDirectionArrows

fig, ax = plt.subplots(figsize=(6, 5))
ax.set_xlim(0, 100)
ax.set_ylim(0, 50)
codes = AnchoredText.codes
print(sorted(codes.items(), key=lambda kv: kv[1]))
boxes = []
for name, code in sorted(codes.items(), key=lambda kv: kv[1]):
    if code == 0:
        continue
    at = AnchoredText(name, loc=code, prop=dict(size=7), frameon=True, borderpad=0.3)
    ax.add_artist(at)
    boxes.append((name, at))
bar = AnchoredSizeBar(ax.transData, 20, "20 m", loc="lower center", pad=0.5, sep=3, frameon=False, size_vertical=1)
ax.add_artist(bar)
arrows = AnchoredDirectionArrows(ax.transAxes, "E", "N", loc="center left", fontsize=8)
ax.add_artist(arrows)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for name, at in boxes:
    print(name, [round(v, 1) for v in at.get_window_extent(r).bounds])
print([round(v, 1) for v in bar.get_window_extent(r).bounds])
print([round(v, 1) for v in arrows.get_window_extent(r).bounds])
print(boxes[0][1].txt.get_text(), boxes[0][1].patch.get_boxstyle().pad)
fig.savefig("anchored.png")
