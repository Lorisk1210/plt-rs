"""Five random walks of 10⁴ points, labelled, with a legend."""

import matplotlib.pyplot as plt

from _common import random_walk, save

walks = [random_walk(10_000, 30 + k) for k in range(5)]


def draw():
    fig, ax = plt.subplots()
    for k, (x, y) in enumerate(walks):
        ax.plot(x, y, label=f"sensor {k}")
    ax.set_title("Five signals of 10,000 points")
    ax.set_xlabel("sample")
    ax.set_ylabel("value")
    ax.legend()
    return fig


if __name__ == "__main__":
    save(draw())
