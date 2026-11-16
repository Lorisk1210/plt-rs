# Linear regression with a 95% confidence band for the mean computed by hand with numpy.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
x = np.sort(rng.uniform(0, 10, 40))
y = 1.5 * x + 2 + rng.normal(0, 2, 40)
X = np.c_[np.ones_like(x), x]
beta, res, *_ = np.linalg.lstsq(X, y, rcond=None)
resid = y - X @ beta
s2 = resid @ resid / (len(x) - 2)
xs = np.linspace(0, 10, 100)
Xs = np.c_[np.ones_like(xs), xs]
se = np.sqrt(s2 * np.sum(Xs @ np.linalg.inv(X.T @ X) * Xs, axis=1))
fig, ax = plt.subplots(figsize=(6, 4))
ax.scatter(x, y, s=12)
ax.plot(xs, Xs @ beta, "k")
ax.fill_between(xs, Xs @ beta - 2.02 * se, Xs @ beta + 2.02 * se, alpha=0.25)
ax.set_title(f"y = {beta[1]:.2f}x + {beta[0]:.2f}")
print(np.round(beta, 4))
fig.savefig("regression.png")
