"""A pie of six shares with labels and percentages, starting at the top."""

import matplotlib.pyplot as plt

from _common import save

shares = [31.0, 24.0, 17.0, 12.0, 9.0, 7.0]
labels = ["rent", "food", "transport", "savings", "leisure", "other"]


def draw():
    fig, ax = plt.subplots()
    ax.pie(shares, labels=labels, autopct="%1.1f%%", startangle=90.0)
    ax.set_title("Where the money goes")
    return fig


if __name__ == "__main__":
    save(draw())
