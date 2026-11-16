# Legend anchored in figure coordinates through bbox_transform=fig.transFigure.
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 2, figsize=(8, 3.5))
axs[0].plot([1, 2, 3], label="one")
axs[1].plot([3, 2, 1], label="two", color="C1")
leg = axs[1].legend(*[sum(lists, []) for lists in zip(axs[0].get_legend_handles_labels(), axs[1].get_legend_handles_labels())],
                    loc="upper right", bbox_to_anchor=(0.98, 0.98), bbox_transform=fig.transFigure)
fig.subplots_adjust(top=0.8)
print([t.get_text() for t in leg.get_texts()])
fig.savefig("legend_fig_coords.png")
fig.savefig("legend_bbox_transform_figure_small.png", dpi=40)
print(fig.get_size_inches().tolist(), len(fig.axes))
