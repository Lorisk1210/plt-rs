"""50 distinct formulas: sums, fractions, roots, sub- and superscripts."""

import matplotlib.pyplot as plt

from _common import save


def draw():
    fig, ax = plt.subplots()
    for k in range(50):
        formula = (rf"$\sum_{{n=1}}^{{{k}}} \frac{{x^{{n}}}}{{n!}} + "
                   rf"\sqrt{{\alpha_{{{k}}}}} - \int_0^{{{k}}} \beta\,dt$")
        ax.text((k % 5) / 5.0, (k // 5) / 10.0, formula)
    return fig


if __name__ == "__main__":
    save(draw())
