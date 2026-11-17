# set_offset with a callable (width, height, xdescent, ydescent, renderer) placing a box relative to the axes corner; changes on resize.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import TextArea, VPacker

fig, ax = plt.subplots(figsize=(4, 3))
box = VPacker(children=[TextArea("corner"), TextArea("box")], align="right", pad=3, sep=1)
calls = []


def place(width, height, xdescent, ydescent, renderer):
    calls.append((round(width, 1), round(height, 1)))
    b = ax.bbox
    return b.x1 - width - 5 + xdescent, b.y0 + 5 + ydescent


box.set_offset(place)
box.set_figure(fig)
fig.artists.append(box)
fig.canvas.draw()
r = fig.canvas.get_renderer()
print(calls[-1], [round(v, 1) for v in box.get_window_extent(r).bounds])
fig.set_size_inches(6, 4)
fig.canvas.draw()
print(calls[-1], [round(v, 1) for v in box.get_window_extent(r).bounds])
bb = box.get_bbox(r)
print([round(v, 1) for v in box.get_offset(bb, r)])
box.set_offset((10, 10))
fig.canvas.draw()
print([round(v, 1) for v in box.get_window_extent(r).bounds])
print(len(calls) > 2, box.get_visible_children() == box.get_children())
box.set_width(80)
box.set_height(40)
fig.canvas.draw()
print([round(v, 1) for v in box.get_bbox(r).bounds])
fig.savefig("callable_offset.png")
