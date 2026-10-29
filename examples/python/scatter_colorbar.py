"""Scatter plot with a colormap and a colorbar: the weight, fuel use and
power of 150 cars, the marker size giving the engine size."""

import pltrs.pyplot as plt
from samples import Samples

rng = Samples(7)
weight = rng.normal(150, 1.45, 0.3)  # tonnes
power = 40 + 85 * weight + rng.normal(150, 0, 12)  # kW
consumption = 2.2 + 3.1 * weight + 0.012 * power + rng.normal(150, 0, 0.45)  # l/100 km
engine = 0.4 + 0.9 * weight + rng.normal(150, 0, 0.15)  # litres

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
points = ax.scatter(
    weight,
    consumption,
    s=60 * engine**2,
    c=power,
    cmap="viridis",
    alpha=0.85,
    edgecolors="white",
    linewidths=0.6,
)
fig.colorbar(points, ax=ax, label="power (kW)")
ax.set_title("Heavier cars use more fuel")
ax.set_xlabel("weight (t)")
ax.set_ylabel("fuel use (l/100 km)")

fig.savefig("scatter_colorbar.png")
