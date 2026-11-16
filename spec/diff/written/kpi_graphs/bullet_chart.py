# Bullet chart: qualitative ranges as background bars, actual value bar and a target marker.
import matplotlib.pyplot as plt

metrics = [("Revenue", 270, 250, [150, 225, 300]), ("Profit", 22, 26, [20, 25, 30]), ("NPS", 45, 50, [30, 50, 70])]
fig, axs = plt.subplots(len(metrics), 1, figsize=(6, 3.5))
for ax, (name, actual, target, ranges) in zip(axs, metrics):
    for r, shade in zip(reversed(ranges), ["0.85", "0.7", "0.55"]):
        ax.barh(0, r, color=shade, height=0.8)
    ax.barh(0, actual, color="k", height=0.3)
    ax.plot([target, target], [-0.3, 0.3], color="red", lw=3)
    ax.set_yticks([0], [name])
    ax.set_xlim(0, ranges[-1])
fig.tight_layout()
print([m[1] >= m[2] for m in metrics])
fig.savefig("bullets.png")
