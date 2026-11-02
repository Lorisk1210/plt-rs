"""Eight horizontal bars, labelled with their values, top to bottom."""

import matplotlib.pyplot as plt

from _common import FastRng, save

cities = ["Oslo", "Lima", "Pune", "Kyiv", "Turin", "Quito", "Perth", "Graz"]
days = FastRng(40).uniform(8, 10.0, 90.0).round()


def draw():
    fig, ax = plt.subplots()
    bars = ax.barh(cities, days)
    ax.bar_label(bars)
    ax.invert_yaxis()
    ax.set_xlabel("rainy days per quarter")
    ax.set_title("Rain by city")
    return fig


if __name__ == "__main__":
    save(draw())
