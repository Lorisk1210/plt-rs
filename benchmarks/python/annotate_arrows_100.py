"""100 annotations, each pointing at its point with an arrow."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(50)
x, y = rng.uniform(100, 0.0, 10.0), rng.uniform(100, 0.0, 10.0)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y, "o", markersize=3)
    for i in range(100):
        ax.annotate(f"p{i}", (x[i], y[i]), xytext=(12.0, 8.0), textcoords="offset points",
                    arrowprops={"arrowstyle": "->"}, fontsize=6)
    return fig


if __name__ == "__main__":
    save(draw())
