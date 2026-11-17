# A 12x12 grid of subplots with shared axes and tiny plots; tick label visibility, positions, and drawing with layout engines.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

for layout in (None, "constrained"):
    fig, axs = plt.subplots(12, 12, figsize=(10, 10), dpi=40, sharex=True, sharey=True, layout=layout)
    x = np.linspace(0, 1, 10)
    for k, ax in enumerate(axs.flat):
        ax.plot(x, x ** (1 + k / 100), lw=0.5)
    fig.canvas.draw()
    print(layout, len(fig.axes), [round(v, 4) for v in axs[0, 0].get_position().bounds],
          [round(v, 4) for v in axs[-1, -1].get_position().bounds])
    print(sum(any(t.get_visible() for t in ax.get_xticklabels()) for ax in axs.flat))
    fig.savefig(f"grid_{layout}.png")
    plt.close(fig)
fig, axs = plt.subplots(12, 12, figsize=(10, 10), dpi=40)
for ax in axs.flat:
    ax.set_axis_off()
axs[3, 4].set_axis_on()
fig.canvas.draw()
print(sum(ax.axison for ax in axs.flat), len(fig.findobj()) > 500)
fig.subplots_adjust(wspace=0, hspace=0)
fig.canvas.draw()
print([round(v, 4) for v in axs[0, 1].get_position().bounds], [round(v, 4) for v in axs[1, 0].get_position().bounds])
axs[0, 0].remove()
print(len(fig.axes))
fig.savefig("grid_off.png")
plt.close("all")
