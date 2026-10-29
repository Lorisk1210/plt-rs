"""``violinplot`` of 50 groups of 1000 values."""

import matplotlib.pyplot as plt

from _common import normal_groups, save

groups = normal_groups(50, 10)


def draw():
    fig, ax = plt.subplots()
    ax.violinplot(groups)
    return fig


if __name__ == "__main__":
    save(draw())
