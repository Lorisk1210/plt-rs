"""Mathtext: normal distributions labelled with TeX-like formulas, typeset
without TeX, and the density written out in the axes."""

import numpy as np

import pltrs.pyplot as plt

x = np.linspace(-5, 5, 400)
fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
for mu, sigma in [(0, 0.6), (0, 1), (-1.5, 1.6)]:
    density = np.exp(-((x - mu) ** 2) / (2 * sigma**2)) / (sigma * np.sqrt(2 * np.pi))
    ax.plot(x, density, linewidth=2, label=rf"$\mu = {mu},\ \sigma = {sigma}$")
ax.text(
    1.2,
    0.48,
    r"$f(x) = \frac{1}{\sigma\sqrt{2\pi}}\,e^{-\frac{(x-\mu)^2}{2\sigma^2}}$",
    fontsize=15,
)
ax.set_title(r"The normal distribution $\mathcal{N}(\mu, \sigma^2)$")
ax.set_xlabel(r"$x$")
ax.set_ylabel(r"$f(x)$")
ax.set_ylim(0, 0.75)
ax.legend(loc="upper left")

fig.savefig("mathtext.png")
