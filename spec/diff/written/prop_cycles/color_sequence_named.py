# Use a named colour sequence (tab10, Set2, petroff10) for bars and lines from mpl.color_sequences.
import matplotlib as mpl
import matplotlib.pyplot as plt

seqs = ["tab10", "Set2", "Dark2", "petroff10"]
fig, axs = plt.subplots(len(seqs), 1, figsize=(6, 5))
for ax, name in zip(axs, seqs):
    colors = mpl.color_sequences[name]
    ax.bar(range(len(colors)), [1] * len(colors), color=colors, width=1)
    ax.set_ylabel(name, rotation=0, ha="right", va="center")
    ax.set_xticks([])
    ax.set_yticks([])
    print(name, len(colors))
fig.tight_layout()
fig.savefig("sequences.png")
