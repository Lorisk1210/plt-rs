# Positions of axes in nested subfigures and gridspecs (width/height ratios, wspace), read as figure fractions and pixels.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 5), dpi=50)
left, right = fig.subfigures(1, 2, width_ratios=[2, 1], wspace=0.05)
gs = left.add_gridspec(2, 3, height_ratios=[1, 2], width_ratios=[1, 2, 1], hspace=0.3, wspace=0.4)
axes = [left.add_subplot(gs[0, :]), left.add_subplot(gs[1, 0]), left.add_subplot(gs[1, 1:])]
inner = right.subfigures(2, 1)
axes.append(inner[0].add_subplot())
axes.append(inner[1].add_subplot(projection="polar"))
fig.canvas.draw()
for a in axes:
    print([round(v, 4) for v in a.get_position().bounds], [round(v, 1) for v in a.bbox.bounds])
print([round(v, 4) for v in left.bbox_relative.bounds], [round(v, 1) for v in right.bbox.bounds])
print([round(v, 1) for v in inner[1].bbox.bounds], inner[1].get_figure(root=True) is fig)
print(gs.get_geometry(), gs.get_width_ratios(), gs.get_height_ratios())
ss = axes[2].get_subplotspec()
print(ss.rowspan, ss.colspan, ss.is_first_col(), ss.is_last_row())
print([round(v, 4) for v in ss.get_position(left).bounds])
left.set_facecolor("0.9")
right.suptitle("right side")
fig.canvas.draw()
print([round(v, 1) for v in right._suptitle.get_window_extent().bounds])
fig.savefig("subfigs.png")
fig.subplots_adjust(left=0.2)
fig.canvas.draw()
print([round(v, 4) for v in axes[0].get_position().bounds], [round(v, 4) for v in axes[3].get_position().bounds])
print(len(fig.subfigs), len(right.subfigs), [len(sf.axes) for sf in (left, right, *inner)])
