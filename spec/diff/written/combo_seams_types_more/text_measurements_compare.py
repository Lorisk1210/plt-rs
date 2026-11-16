# Measure several strings' widths in pixels (normal, bold, mathtext, multiline) and size boxes accordingly.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

fig, ax = plt.subplots(figsize=(7, 4), dpi=100)
ax.set_xlim(0, 7); ax.set_ylim(0, 4)
specs = [("plain text", {}), ("bold text", {"weight": "bold"}), (r"$\alpha^2+\beta^2$", {}), ("two\nlines", {}), ("small", {"size": 7})]
fig.canvas.draw()
r = fig.canvas.get_renderer()
widths = []
for k, (s, kw) in enumerate(specs):
    t = ax.text(0.5, 3.4 - k * 0.7, s, **kw)
    bb = t.get_window_extent(r).transformed(ax.transData.inverted())
    ax.add_patch(Rectangle((bb.x0, bb.y0), bb.width, bb.height, fill=False, ec="r", lw=0.5))
    widths.append(round(t.get_window_extent(r).width, 1))
print(widths)
fig.savefig("measured.png", dpi=50)
