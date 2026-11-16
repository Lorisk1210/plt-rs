# Loop over conditions: each saves a 2x2 figure (curve, residuals, histogram, parameter text) and collects fits.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(12)
conditions = {"cold": 0.8, "room": 1.0, "warm": 1.3, "hot": 1.7}
fits = {}
for name, rate in conditions.items():
    t = np.linspace(0, 10, 50)
    y = 5 * np.exp(-rate * t / 4) + rng.normal(0, 0.1, 50)
    coef = np.polyfit(t, np.log(np.clip(y, 0.05, None)), 1)
    fit = np.exp(np.polyval(coef, t))
    fig, axs = plt.subplots(2, 2, figsize=(7, 5))
    axs[0, 0].plot(t, y, "o", ms=3)
    axs[0, 0].plot(t, fit, "r")
    axs[0, 1].plot(t, y - fit, ".")
    axs[0, 1].axhline(0, color="k", lw=0.5)
    axs[1, 0].hist(y - fit, bins=12)
    axs[1, 1].axis("off")
    axs[1, 1].text(0.1, 0.5, f"k = {-coef[0] * 4:.3f}\nA = {np.exp(coef[1]):.3f}", fontsize=12, family="monospace")
    fig.suptitle(name)
    fig.tight_layout()
    fig.savefig(f"fit_{name}.png", dpi=40)
    fits[name] = round(-coef[0] * 4, 3)
    plt.close(fig)
print(fits)
