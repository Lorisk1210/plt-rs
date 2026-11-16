# Mutate axes after plotting: scales, limits, aspect, facecolor, position, title, ticks; read each back after draw.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(np.logspace(0, 3, 20), np.logspace(0, 2, 20), "o-")
ax.set_xscale("log")
ax.set_yscale("log")
ax.set_xlim(1, 2000)
ax.set_aspect("auto")
ax.set_facecolor("#eef")
ax.set_position([0.15, 0.15, 0.7, 0.7])
ax.set_title("mutated", loc="left")
ax.set_xticks([1, 10, 100, 1000], ["one", "ten", "hundred", "thousand"])
fig.canvas.draw()
print(ax.get_xscale(), ax.get_yscale(), ax.get_xlim(), [round(v, 3) for v in ax.get_ylim()])
print(ax.get_facecolor(), [round(v, 3) for v in ax.get_position().bounds], ax.get_title(loc="left"))
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("axes_mutations.png", dpi=50)
