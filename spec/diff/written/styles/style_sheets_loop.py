# Render the same chart in several built-in style sheets with style.context, one file each.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 100)
for name in ["ggplot", "seaborn-v0_8-whitegrid", "bmh", "fivethirtyeight", "dark_background", "grayscale"]:
    with plt.style.context(name):
        fig, ax = plt.subplots(figsize=(4, 3))
        for k in range(3):
            ax.plot(x, np.sin(x + k), label=f"phase {k}")
        ax.set_title(name)
        ax.legend(fontsize=7)
        fig.savefig(f"style_{name}.png")
        plt.close(fig)
print(len(plt.style.available) > 20, "ggplot" in plt.style.available)
