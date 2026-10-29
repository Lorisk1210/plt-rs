"""Twin axes: a climate chart, monthly rainfall as bars on one y axis and
the mean temperature as a line on another, sharing the months."""

import pltrs.pyplot as plt

months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
rain = [67, 64, 75, 91, 118, 132, 128, 131, 98, 84, 81, 76]  # mm
temperature = [0.4, 1.6, 5.4, 9.1, 13.5, 16.9, 18.9, 18.3, 14.6, 10.0, 4.9, 1.6]  # °C

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
bars = ax.bar(months, rain, color="C0", alpha=0.75, label="rainfall")
ax.set_ylabel("rainfall (mm)", color="C0")
ax.tick_params(axis="y", labelcolor="C0")
ax.set_ylim(0, 160)

right = ax.twinx()
(line,) = right.plot(months, temperature, color="C1", marker="o", linewidth=2.5, label="temperature")
right.set_ylabel("temperature (°C)", color="C1")
right.tick_params(axis="y", labelcolor="C1")
right.set_ylim(-4, 24)
right.grid(False)

ax.set_title("Climate of a Central European city")
right.legend(handles=[bars, line], loc="upper left")

fig.savefig("twin_axes.png")
