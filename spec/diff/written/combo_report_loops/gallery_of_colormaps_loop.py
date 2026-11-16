# Save one figure per colormap family with gradient strips and the colormap names as labels.
import numpy as np
import matplotlib.pyplot as plt

families = {"sequential": ["viridis", "plasma", "Blues", "Greens"], "diverging": ["RdBu", "coolwarm", "PiYG"],
            "qualitative": ["tab10", "Set2", "Pastel1"]}
gradient = np.linspace(0, 1, 256)[None, :]
for family, names in families.items():
    fig, axs = plt.subplots(len(names), 1, figsize=(6, 0.5 * len(names) + 0.6))
    fig.suptitle(family)
    for ax, name in zip(axs, names):
        ax.imshow(gradient, aspect="auto", cmap=name)
        ax.text(-0.01, 0.5, name, va="center", ha="right", fontsize=8, transform=ax.transAxes)
        ax.set_axis_off()
    fig.subplots_adjust(left=0.25, right=0.98, top=0.85)
    fig.savefig(f"cmaps_{family}.png", dpi=50)
    print(family, [plt.get_cmap(n).N for n in names])
    plt.close(fig)
