# Build a legend by hand from offset boxes (markers in DrawingAreas + TextAreas in a VPacker), compared with a real legend's size.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.offsetbox import AnchoredOffsetbox, DrawingArea, HPacker, TextArea, VPacker

fig, ax = plt.subplots(figsize=(5, 4))
series = [("alpha", "C0", "o"), ("beta", "C1", "s"), ("gamma", "C2", "^")]
for k, (name, color, marker) in enumerate(series):
    ax.plot(range(5), [v * (k + 1) for v in range(5)], color=color, marker=marker, label=name)
rows = []
for name, color, marker in series:
    da = DrawingArea(28, 10, 0, 0)
    da.add_artist(Line2D([0, 28], [5, 5], color=color))
    da.add_artist(Line2D([14], [5], color=color, marker=marker))
    rows.append(HPacker(children=[da, TextArea(name)], align="center", pad=0, sep=5))
manual = AnchoredOffsetbox(loc="upper left", child=VPacker(children=rows, align="left", pad=0, sep=3),
                           frameon=True, borderpad=0.5, pad=0.4)
manual.patch.set_boxstyle("round,pad=0,rounding_size=0.2")
ax.add_artist(manual)
leg = ax.legend(loc="lower right")
fig.canvas.draw()
r = fig.canvas.get_renderer()
mb, lb = manual.get_window_extent(r), leg.get_window_extent(r)
print([round(v, 1) for v in mb.bounds])
print([round(v, 1) for v in lb.bounds])
print(round(mb.width - lb.width, 1), round(mb.height - lb.height, 1))
print(type(leg._legend_box).__name__, [type(c).__name__ for c in leg._legend_box.get_children()])
print(manual.patch.get_boxstyle().rounding_size, leg.legendPatch.get_boxstyle().pad)
fig.savefig("manual_legend.png")
