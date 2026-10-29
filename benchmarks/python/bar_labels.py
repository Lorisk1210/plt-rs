"""Bars on a categorical axis, each labelled with its value."""

import matplotlib.pyplot as plt

from _common import save

fruits = ["apple", "blueberry", "cherry", "orange", "peach", "plum"]
crates = [40, 100, 30, 55, 72, 18]


def draw():
    fig, ax = plt.subplots()
    bars = ax.bar(fruits, crates)
    ax.bar_label(bars)
    ax.set_ylabel("crates")
    ax.set_title("Fruit delivered this week")
    return fig


if __name__ == "__main__":
    save(draw())
