"""Histogram with a density curve: 1500 commute times, normalised to a
density, against the log-normal distribution they were drawn from."""

import numpy as np

import pltrs.pyplot as plt
from samples import Samples

mu, sigma = 3.3, 0.35  # of the logarithm of the time in minutes
minutes = np.exp(Samples(3).normal(1500, mu, sigma))

x = np.linspace(5, 80, 300)
density = np.exp(-((np.log(x) - mu) ** 2) / (2 * sigma**2)) / (x * sigma * np.sqrt(2 * np.pi))
median = np.exp(mu)

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
ax.hist(minutes, bins=40, range=(5, 80), density=True, alpha=0.6, edgecolor="white", label="survey")
ax.plot(x, density, linewidth=2.5, label="log-normal model")
ax.axvline(median, color="0.3", linestyle="--", linewidth=1)
ax.annotate(
    f"median {median:.0f} min",
    xy=(median, 0.03),
    xytext=(42, 0.036),
    color="0.3",
    arrowprops=dict(arrowstyle="->", color="0.3"),
)
ax.set_title("How long the commute takes")
ax.set_xlabel("minutes")
ax.set_ylabel("density")
ax.set_xlim(5, 80)
ax.legend()

fig.savefig("histogram_density.png")
