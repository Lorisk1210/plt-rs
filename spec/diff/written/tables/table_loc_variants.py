# The same small table at several loc values around the axes, one per subplot.
import matplotlib.pyplot as plt

locs = ["upper right", "lower left", "center", "top", "right", "bottom"]
fig, axs = plt.subplots(2, 3, figsize=(10, 5))
for ax, loc in zip(axs.flat, locs):
    ax.plot([0, 1], [0, 1])
    ax.table(cellText=[["a", "1"], ["b", "2"]], loc=loc, colWidths=[0.15, 0.15])
    ax.set_title(loc, fontsize=9)
fig.tight_layout()
fig.savefig("locs.png")
print([len(ax.tables) for ax in axs.flat])
fig.savefig("table_loc_variants_small.png", dpi=50)
print(fig.get_size_inches())
