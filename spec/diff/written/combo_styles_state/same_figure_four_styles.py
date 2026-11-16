# Build the same 2x2 figure (line, bars, scatter with colorbar, histogram) under four style sheets, reading back colours.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(90)
x = np.linspace(0, 10, 60)
data = rng.normal(size=300)
report = {}
for style in ["default", "ggplot", "bmh", "seaborn-v0_8-dark"]:
    with plt.style.context(style):
        fig, axs = plt.subplots(2, 2, figsize=(8, 6))
        lines = [axs[0, 0].plot(x, np.sin(x + k))[0] for k in range(3)]
        axs[0, 1].bar(list("abcd"), [3, 5, 2, 4])
        sc = axs[1, 0].scatter(rng.random(40), rng.random(40), c=rng.random(40))
        fig.colorbar(sc, ax=axs[1, 0])
        axs[1, 1].hist(data, bins=20)
        fig.suptitle(style)
        fig.tight_layout()
        fig.savefig(f"style_{style}.png", dpi=40)
        report[style] = ([l.get_color() for l in lines], axs[0, 0].get_facecolor())
        plt.close(fig)
for k, v in report.items():
    print(k, v)
