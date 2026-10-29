"""500 lines of 50 points, one ``plot`` call each, then three setter
calls on each line: colour, width and style."""

import matplotlib.pyplot as plt

from _common import save, waves

x, sines = waves(500, 50)
COLORS = ["tab:blue", "tab:orange", "tab:green", "tab:red", "tab:purple"]
STYLES = ["-", "--", ":", "-."]


def draw():
    fig, ax = plt.subplots()
    for k, y in enumerate(sines):
        (line,) = ax.plot(x, y)
        line.set_color(COLORS[k % 5])
        line.set_linewidth(0.5 + 0.5 * (k % 4))
        line.set_linestyle(STYLES[k % 4])
    return fig


if __name__ == "__main__":
    save(draw())
