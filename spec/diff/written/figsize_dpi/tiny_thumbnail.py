# Sparkline-sized thumbnails (1.5 x 0.4 inches) without axes for a table of stock trends.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(11)
for ticker in ["AAA", "BBB", "CCC"]:
    walk = np.cumsum(rng.normal(0, 1, 60))
    fig = plt.figure(figsize=(1.5, 0.4), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.plot(walk, lw=1, color="green" if walk[-1] > walk[0] else "red")
    ax.plot(len(walk) - 1, walk[-1], "o", ms=2, color="k")
    ax.axis("off")
    ax.margins(0.05, 0.15)
    fig.savefig(f"spark_{ticker}.png")
    print(ticker, fig.canvas.get_width_height())
