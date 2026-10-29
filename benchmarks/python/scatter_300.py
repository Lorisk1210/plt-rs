"""300 clusters of 10 points, one ``scatter`` call each."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(26)
clusters = [(rng.uniform(10, k % 20, k % 20 + 1.0), rng.uniform(10, k // 20, k // 20 + 1.0))
            for k in range(300)]


def draw():
    fig, ax = plt.subplots()
    for x, y in clusters:
        ax.scatter(x, y)
    return fig


if __name__ == "__main__":
    save(draw())
