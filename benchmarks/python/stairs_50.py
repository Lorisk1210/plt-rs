"""The 50 values of a histogram drawn as stairs."""

import matplotlib.pyplot as plt

from _common import FastRng, save

values = FastRng(42).uniform(50, 1.0, 10.0)


def draw():
    fig, ax = plt.subplots()
    ax.stairs(values)
    ax.set_title("Counts per bin")
    return fig


if __name__ == "__main__":
    save(draw())
