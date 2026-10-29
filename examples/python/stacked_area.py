"""Stacked area chart: where a country's electricity came from, year by
year, the top edge giving the total."""

import numpy as np

import pltrs.pyplot as plt

years = np.arange(2000, 2025)
t = years - 2000
sources = {
    "coal": 210 - 7.5 * t + 0.08 * t**2,
    "gas": 90 + 3.2 * t - 0.06 * t**2,
    "nuclear": 160 - 1.5 * t,
    "hydro": 60 + 4 * np.sin(t / 2.2),
    "wind": 0.35 * t**2,
    "solar": 0.012 * t**3,
}

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
ax.stackplot(years, list(sources.values()), labels=list(sources), alpha=0.9)
ax.set_title("Electricity generation by source")
ax.set_ylabel("TWh")
ax.set_xlim(2000, 2024)
ax.legend(loc="center left", bbox_to_anchor=(1, 0.5), reverse=True)

fig.savefig("stacked_area.png")
