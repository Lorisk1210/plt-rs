# Every ha x va x multialignment combination for a two-line text: the extent's offset from the anchor point.
import itertools
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(3, 3), dpi=100)
r = fig.canvas.get_renderer()
rows = []
for ha, va, ma in itertools.product(("left", "center", "right"), ("top", "center", "baseline", "bottom", "center_baseline"),
                                    ("left", "center", "right")):
    t = fig.text(0.5, 0.5, "short\na longer line", ha=ha, va=va, ma=ma, fontsize=10)
    fig.canvas.draw()
    b = t.get_window_extent(r)
    rows.append((ha, va, ma, round(b.x0 - 150, 2), round(b.y0 - 150, 2), round(b.width, 2), round(b.height, 2)))
    t.remove()
for row in rows[::3]:
    print(row)
print(len({(r[3], r[4]) for r in rows}))
t = fig.text(0.5, 0.5, "x", ha="left", va="baseline")
fig.canvas.draw()
print(t.get_horizontalalignment(), t.get_verticalalignment(), t._multialignment)
try:
    t.set_ha("middle")
except ValueError as e:
    print("ValueError", str(e)[:60])
t.set_multialignment("center")
t.set_text("a\nbbb")
fig.canvas.draw()
print([round(v, 2) for v in t.get_window_extent(r).bounds], t._multialignment)
