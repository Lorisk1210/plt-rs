# Formatter given as a plain function or format string (set_major_formatter shortcuts).
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(10, 3))
for ax in axs:
    ax.plot([0, 1000, 2000], [0, 0.5, 1])
axs[0].xaxis.set_major_formatter(lambda x, pos: f"{x / 1000:.1f}k")
axs[1].xaxis.set_major_formatter("{x:.0f} m")
axs[2].yaxis.set_major_formatter("{x:.0%}")
fig.tight_layout()
fig.canvas.draw()
print([t.get_text() for t in axs[0].get_xticklabels()][:3], [t.get_text() for t in axs[2].get_yticklabels()][:3])
print(type(axs[1].xaxis.get_major_formatter()).__name__)
fig.savefig("shortcut_formatters.png")
