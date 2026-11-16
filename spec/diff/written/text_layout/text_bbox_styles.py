# Every box style for text bboxes (round, round4, sawtooth, roundtooth, larrow, rarrow, darrow, circle).
import matplotlib.pyplot as plt

styles = ["square", "round", "round4", "sawtooth", "roundtooth", "larrow", "rarrow", "darrow", "circle", "ellipse"]
fig, ax = plt.subplots(figsize=(6, 6))
for i, st in enumerate(styles):
    ax.text(0.5, 0.95 - i * 0.095, st, ha="center", va="center", fontsize=10, transform=ax.transAxes,
            bbox=dict(boxstyle=st + ",pad=0.3", fc="lightyellow", ec="k"))
ax.axis("off")
fig.canvas.draw()
print(len(ax.texts))
fig.savefig("boxstyles.png")
fig.savefig("text_bbox_styles_small.png", dpi=50)
print(fig.get_size_inches())
