# Linear regression diagnostics: predicted vs actual with identity, residual histogram with normal, QQ, and leverage-like scatter.
import numpy as np
import matplotlib.pyplot as plt
from statistics import NormalDist

rng = np.random.default_rng(44)
X = np.c_[np.ones(150), rng.normal(size=(150, 2))]
beta = np.array([1.0, 2.0, -1.5])
yv = X @ beta + rng.standard_t(5, 150) * 0.5
b_hat, *_ = np.linalg.lstsq(X, yv, rcond=None)
pred = X @ b_hat
res = yv - pred
H = X @ np.linalg.inv(X.T @ X) @ X.T
lev = np.diag(H)
fig, axs = plt.subplots(2, 2, figsize=(9, 8))
axs[0, 0].scatter(pred, yv, s=8)
lim = [min(pred.min(), yv.min()), max(pred.max(), yv.max())]
axs[0, 0].plot(lim, lim, "r--")
axs[0, 1].hist(res, bins=20, density=True)
g = np.linspace(res.min(), res.max(), 100)
axs[0, 1].plot(g, np.exp(-g ** 2 / (2 * res.var())) / np.sqrt(2 * np.pi * res.var()), "k")
q = np.array([NormalDist().inv_cdf((i - 0.5) / 150) for i in range(1, 151)])
axs[1, 0].plot(q, np.sort(res / res.std()), ".")
axs[1, 0].plot([-3, 3], [-3, 3], "r--")
axs[1, 1].scatter(lev, res / res.std(), s=8, c=np.abs(res), cmap="Reds")
fig.tight_layout()
print(np.round(b_hat, 4).tolist(), round(float(lev.max()), 5))
fig.savefig("diagnostics.png", dpi=50)
