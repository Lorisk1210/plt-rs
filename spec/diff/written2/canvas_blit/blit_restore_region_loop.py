# Manual blitting: save the background, then for each step restore it, draw an animated artist and blit; checksum each frame.
import hashlib
import numpy as np
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg

fig = Figure(figsize=(4, 3), dpi=60)
canvas = FigureCanvasAgg(fig)
ax = fig.add_subplot()
x = np.linspace(0, 2 * np.pi, 100)
ax.plot(x, np.sin(x), color="0.7")
ax.set_ylim(-1.2, 1.2)
dot, = ax.plot([], [], "ro", animated=True)
canvas.draw()
background = canvas.copy_from_bbox(ax.bbox)
digests = []
for k in range(5):
    canvas.restore_region(background)
    dot.set_data([x[k * 20]], [np.sin(x[k * 20])])
    ax.draw_artist(dot)
    canvas.blit(ax.bbox)
    frame = np.asarray(canvas.buffer_rgba())
    red = (frame[..., 0] > 200) & (frame[..., 1] < 60) & (frame[..., 2] < 60)
    digests.append(int(red.sum()))
print(digests)
canvas.restore_region(background)
plain = np.asarray(canvas.buffer_rgba())
print(int(((plain[..., 0] > 200) & (plain[..., 1] < 60)).sum()))
fig.savefig("after_blit.png")
canvas.draw()
background2 = canvas.copy_from_bbox(ax.bbox)
print([int(v) for v in background2.get_extents()], dot.get_animated(), dot in ax.lines)
