# va='baseline' vs 'bottom' vs 'center_baseline' with descenders and mathtext.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(7, 2.5))
ax.axhline(0.5, color="r", lw=0.5)
for i, va in enumerate(["baseline", "bottom", "center_baseline", "top", "center"]):
    ax.text(0.1 + i * 0.19, 0.5, f"gy {va}", va=va, fontsize=12)
ax.text(0.1, 0.15, r"$\int_0^1 x\,dx$ baseline", va="baseline")
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)
ax.set_yticks([])
fig.canvas.draw()
print([round(t.get_window_extent().y0, 1) for t in ax.texts])
fig.savefig("baselines.png")
