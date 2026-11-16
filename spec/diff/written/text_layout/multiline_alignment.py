# Multi-line text with ha/va/multialignment combinations and linespacing, marked at anchor points.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(7, 4))
ax.set_xlim(0, 3)
ax.set_ylim(0, 2)
text = "first line\nsecond, longer line\nthird"
for i, ha in enumerate(["left", "center", "right"]):
    for j, va in enumerate(["top", "center"]):
        x, y = 0.5 + i, 0.5 + j
        ax.plot(x, y, "r+")
        ax.text(x, y, text, ha=ha, va=va, multialignment=ha, linespacing=1.6, fontsize=8,
                bbox=dict(fc="none", ec="0.7"))
ax.set_title("alignment grid")
fig.canvas.draw()
print([round(t.get_window_extent().width, 1) for t in ax.texts][:3])
fig.savefig("alignment.png")
