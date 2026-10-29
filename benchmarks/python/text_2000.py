"""2000 small labels on a 50 × 40 lattice, one ``text`` call each."""

import matplotlib.pyplot as plt

from _common import save


def draw():
    fig, ax = plt.subplots()
    for i in range(2000):
        ax.text((i % 50) / 50.0, (i // 50) / 40.0, f"t{i}", fontsize=6)
    return fig


if __name__ == "__main__":
    save(draw())
