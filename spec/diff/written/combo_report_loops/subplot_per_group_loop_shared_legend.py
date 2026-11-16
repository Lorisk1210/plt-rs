# Loop over groups drawing into a grid, each panel with lines and a band; one shared legend from the first panel.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(17)
groups = ["north", "south", "east", "west", "centre", "islands"]
fig, axs = plt.subplots(2, 3, figsize=(11, 6), sharex=True, sharey=True)
x = np.arange(24)
for ax, g in zip(axs.flat, groups):
    runs = np.cumsum(rng.normal(0.2, 1, (20, 24)), axis=1)
    mean, sd = runs.mean(axis=0), runs.std(axis=0)
    ax.plot(x, mean, label="mean")
    ax.fill_between(x, mean - sd, mean + sd, alpha=0.3, label="±1 sd")
    ax.plot(x, np.median(runs, axis=0), ls="--", label="median")
    ax.set_title(g)
handles, labels = axs[0, 0].get_legend_handles_labels()
fig.legend(handles, labels, loc="lower center", ncols=3)
fig.subplots_adjust(bottom=0.15)
print(labels, [type(h).__name__ for h in handles])
fig.savefig("groups.png", dpi=50)
