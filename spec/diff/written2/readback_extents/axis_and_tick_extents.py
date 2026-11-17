# Extents of axis parts: ticklabel bboxes via _get_ticklabel_bboxes, offset text, axis label, and Axis.get_tightbbox per side.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
ax.plot([0, 1e6], [0, 1e-4])
ax.set_xlabel("big x")
ax.set_ylabel("small y")
ax.tick_params(axis="x", labelrotation=30, labeltop=True)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for axis in (ax.xaxis, ax.yaxis):
    b1, b2 = axis._get_ticklabel_bboxes(axis._update_ticks(), r)
    print(type(axis).__name__, len(b1), len(b2), [round(v, 1) for v in b1[0].bounds] if b1 else None,
          [round(v, 1) for v in b2[0].bounds] if b2 else None)
    print([round(v, 1) for v in axis.get_tightbbox(r).bounds], [round(v, 1) for v in axis.label.get_window_extent(r).bounds])
    print(repr(axis.get_offset_text().get_text()), [round(v, 1) for v in axis.get_offset_text().get_window_extent(r).bounds])
print(len(ax.xaxis.get_major_ticks()), [t.label2.get_visible() for t in ax.xaxis.get_major_ticks()][:2])
print([round(v, 1) for v in ax.xaxis.get_major_ticks()[1].tick1line.get_window_extent(r).bounds])
print([round(v, 1) for v in ax.yaxis.get_major_ticks()[0].label1.get_window_extent(r).bounds])
ax.xaxis.set_label_position("top")
fig.canvas.draw()
print([round(v, 1) for v in ax.xaxis.label.get_window_extent(r).bounds])
print([round(v, 1) for v in ax.spines["left"].get_window_extent(r).bounds])
print([round(v, 1) for v in ax.spines["bottom"].get_window_extent(r).bounds])
fig.savefig("axis_extents.png")
ax.tick_params(axis="y", pad=12, labelsize=14)
fig.canvas.draw()
print([round(v, 1) for v in ax.yaxis.get_tightbbox(r).bounds], ax.yaxis.get_major_ticks()[0].get_pad())
print([round(v, 1) for v in ax.get_tightbbox(r).bounds])
