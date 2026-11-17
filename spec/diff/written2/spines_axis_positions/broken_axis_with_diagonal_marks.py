# A broken y-axis built from two axes with hidden spines and diagonal cut marks drawn with markers in axes coordinates.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

pts = np.array([0.15, 0.3, 0.45, 0.6, 0.75, 14.0, 0.2, 0.4, 0.55, 13.5])
fig, (top, bottom) = plt.subplots(2, 1, sharex=True, figsize=(4, 4), dpi=60)
fig.subplots_adjust(hspace=0.05)
top.plot(pts, "o-")
bottom.plot(pts, "o-")
top.set_ylim(12.5, 15)
bottom.set_ylim(0, 1)
top.spines.bottom.set_visible(False)
bottom.spines.top.set_visible(False)
top.xaxis.tick_top()
top.tick_params(labeltop=False)
bottom.xaxis.tick_bottom()
d = 0.5
kwargs = dict(marker=[(-1, -d), (1, d)], markersize=12, linestyle="none", color="k", mec="k", mew=1, clip_on=False)
m1, = top.plot([0, 1], [0, 0], transform=top.transAxes, **kwargs)
m2, = bottom.plot([0, 1], [1, 1], transform=bottom.transAxes, **kwargs)
fig.canvas.draw()
r = fig.canvas.get_renderer()
print([round(v, 3) for v in top.get_position().bounds], [round(v, 3) for v in bottom.get_position().bounds])
print(top.xaxis.get_ticks_position(), bottom.xaxis.get_ticks_position(), [t.get_text() for t in bottom.get_xticklabels()][:3])
print(m1.get_marker() is not None, len(m1.get_path().vertices), m1.get_clip_on())
print([round(v, 1) for v in m1.get_window_extent(r).bounds], [round(v, 1) for v in m2.get_window_extent(r).bounds])
print(top.spines.bottom.get_visible(), bottom.spines.top.get_visible(), top.get_shared_x_axes().joined(top, bottom))
print(len(top.lines), len(bottom.lines), [round(v, 2) for v in top.get_xlim()])
fig.savefig("broken_axis.png")
