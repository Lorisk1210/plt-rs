# Multi-line text heights for linespacing values and font sizes; per-line layout from Text._get_layout with descents.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(4, 4), dpi=100)
r = fig.canvas.get_renderer()
for size in (8, 12, 20):
    for spacing in (0.8, 1.2, 2.0):
        t = fig.text(0.1, 0.5, "Ag\nxy\nÉq", fontsize=size, linespacing=spacing)
        fig.canvas.draw()
        bbox, info, (corner, extent) = t._get_layout(r)
        print(size, spacing, round(bbox.height, 2), [round(i[2][1], 2) for i in info], round(corner[1], 2),
              [tuple(round(v, 2) for v in i[1]) for i in info][:1])
        t.remove()
t = fig.text(0.1, 0.5, "single")
fig.canvas.draw()
print(t.get_linespacing(), [round(v, 2) for v in t.get_window_extent(r).bounds])
t.set_linespacing(3)
fig.canvas.draw()
print([round(v, 2) for v in t.get_window_extent(r).bounds])
t.set_text("")
fig.canvas.draw()
print([round(v, 2) for v in t.get_window_extent(r).bounds])
t.set_text("trailing newline\n")
fig.canvas.draw()
print(round(t.get_window_extent(r).height, 2))
t.set_text("\nleading newline")
fig.canvas.draw()
print(round(t.get_window_extent(r).height, 2))
