# Distribution of daily returns with a normal fit and the 5% value-at-risk line.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(10)
r = rng.standard_t(4, 1000) * 0.01
var5 = np.percentile(r, 5)
fig, ax = plt.subplots(figsize=(6, 4))
ax.hist(r, bins=60, density=True, color="0.7")
x = np.linspace(r.min(), r.max(), 300)
mu, sd = r.mean(), r.std()
ax.plot(x, np.exp(-(x - mu) ** 2 / (2 * sd ** 2)) / (sd * np.sqrt(2 * np.pi)), "k--", label="normal fit")
ax.axvline(var5, color="red", label=f"VaR 5% = {var5:.2%}")
ax.legend()
ax.xaxis.set_major_formatter(plt.FuncFormatter(lambda v, _: f"{v:.0%}"))
print(round(float(var5), 6))
fig.savefig("returns.png")
