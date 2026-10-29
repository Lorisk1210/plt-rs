"""300 labels: rotated two-line texts, formulas, and texts with inline
math, under a title and axis labels that hold math too."""

import matplotlib.pyplot as plt

from _common import save


def draw():
    fig, ax = plt.subplots()
    ax.set_title(r"Mixed text: $\alpha_i + \beta^2 = \gamma$")
    ax.set_xlabel(r"time $t$ (s)")
    ax.set_ylabel(r"response $\sigma(t)$")
    for k in range(300):
        x, y = (k % 15) / 15.0, (k // 15) / 20.0
        if k % 3 == 0:
            ax.text(x, y, f"label {k}\nsecond line", fontsize=7, rotation=k % 90)
        elif k % 3 == 1:
            ax.text(x, y, rf"$x_{{{k}}}^{{2}} + \frac{{\pi}}{{{k + 1}}}$", fontsize=8)
        else:
            ax.text(x, y, rf"value {k}: $\sqrt{{{k}}} \leq \Omega$", fontsize=7,
                    ha="center", va="center")
    return fig


if __name__ == "__main__":
    save(draw())
