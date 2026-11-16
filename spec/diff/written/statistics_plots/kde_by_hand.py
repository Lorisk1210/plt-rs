# Kernel density estimate written with numpy (Gaussian kernel, Silverman bandwidth) over a histogram.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
data = np.concatenate([rng.normal(-2, 0.6, 150), rng.normal(1.5, 1.0, 250)])
bw = 1.06 * data.std() * data.size ** (-1 / 5)
grid = np.linspace(-5, 5, 400)
kde = np.exp(-0.5 * ((grid[:, None] - data[None, :]) / bw) ** 2).sum(axis=1) / (data.size * bw * np.sqrt(2 * np.pi))
fig, ax = plt.subplots(figsize=(6, 4))
ax.hist(data, bins=40, density=True, alpha=0.4)
ax.plot(grid, kde, lw=2)
ax.plot(data, np.full_like(data, -0.01), "|", color="k", alpha=0.3)
print(round(float(bw), 5), round(float(np.trapezoid(kde, grid)), 4))
fig.savefig("kde.png")
