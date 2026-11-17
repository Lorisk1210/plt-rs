# restore_region with a sub-box and offsets (moving a patch of pixels), as some blit libraries do; inspect the result.
import numpy as np
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg

fig = Figure(figsize=(3, 3), dpi=50)
canvas = FigureCanvasAgg(fig)
ax = fig.add_axes((0, 0, 1, 1))
ax.imshow(np.add.outer(np.arange(30), np.arange(30)), cmap="gray", interpolation="nearest")
ax.set_axis_off()
canvas.draw()
before = np.asarray(canvas.buffer_rgba()).copy()
region = canvas.copy_from_bbox(fig.bbox)
canvas.restore_region(region, bbox=(10, 10, 60, 60), xy=(70, 70))
after = np.asarray(canvas.buffer_rgba()).copy()
h = after.shape[0]
changed = np.argwhere((after != before).any(axis=2))
print(after.shape, len(changed))
if len(changed):
    print(changed.min(axis=0).tolist(), changed.max(axis=0).tolist())
print(before[h - 1 - 10, 10].tolist(), after[h - 1 - 70, 70].tolist())
fig.savefig("moved_region.png")
canvas.draw()
again = np.asarray(canvas.buffer_rgba())
print(np.array_equal(again, before))
region2 = canvas.copy_from_bbox(ax.bbox)
canvas.restore_region(region2)
print(np.array_equal(np.asarray(canvas.buffer_rgba()), before))
whole = fig.bbox.bounds
print([round(v, 3) for v in whole], [int(v) for v in region2.get_extents()])
