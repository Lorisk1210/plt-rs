# Copy styles between artists: update_from for lines and patches, then read properties back to verify.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle
from matplotlib.lines import Line2D

fig, ax = plt.subplots(figsize=(5, 4))
src = Rectangle((0.1, 0.1), 0.3, 0.3, fc="tomato", ec="navy", lw=3, ls="--", hatch="//", alpha=0.7)
ax.add_patch(src)
dst = Circle((0.7, 0.7), 0.15)
dst.update_from(src)
ax.add_patch(dst)
line_src, = ax.plot([0, 1], [1, 0], color="green", lw=4, ls=":", marker="o", ms=8, mfc="yellow")
line_dst = Line2D([0, 1], [0.5, 0.5])
line_dst.update_from(line_src)
ax.add_line(line_dst)
print(dst.get_facecolor(), dst.get_edgecolor(), dst.get_linewidth(), dst.get_linestyle(), dst.get_hatch(), dst.get_alpha())
print(line_dst.get_color(), line_dst.get_marker(), line_dst.get_markerfacecolor(), line_dst.get_linestyle())
fig.savefig("update_from.png", dpi=50)
