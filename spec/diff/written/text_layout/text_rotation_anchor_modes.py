# Rotated text around its anchor: rotation_mode 'default' vs 'anchor' at several angles.
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 2, figsize=(9, 4))
for ax, mode in zip(axs, ["default", "anchor"]):
    for i, angle in enumerate([0, 30, 60, 90, 135]):
        ax.plot(i, 0, "ko", ms=3)
        ax.text(i, 0, f"rot {angle}", rotation=angle, rotation_mode=mode, ha="left", va="bottom")
    ax.set_xlim(-0.5, 5)
    ax.set_ylim(-1, 1.5)
    ax.set_title(mode)
fig.canvas.draw()
print([[round(v, 1) for v in t.get_window_extent().bounds] for t in axs[1].texts[:2]])
fig.savefig("rotation_modes.png")
