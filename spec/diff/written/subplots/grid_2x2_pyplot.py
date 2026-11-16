# Four diagnostic plots of a regression in a 2x2 grid using only pyplot's subplot().
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(0)
x = np.linspace(0, 10, 80)
y = 2.5 * x + 1 + np.random.normal(0, 2, x.size)
coef = np.polyfit(x, y, 1)
fit = np.polyval(coef, x)
resid = y - fit

plt.figure(figsize=(8, 6))
plt.subplot(2, 2, 1)
plt.scatter(x, y, s=10)
plt.plot(x, fit, "r")
plt.title("Data and fit")
plt.subplot(2, 2, 2)
plt.scatter(fit, resid, s=10)
plt.axhline(0, color="k", lw=0.8)
plt.title("Residuals vs fitted")
plt.subplot(223)
plt.hist(resid, bins=15)
plt.title("Residual histogram")
plt.subplot(224)
plt.plot(np.sort(resid), np.linspace(0, 1, resid.size), drawstyle="steps-post")
plt.title("Empirical CDF")
plt.tight_layout()
print("slope %.3f intercept %.3f" % tuple(coef))
plt.savefig("diagnostics.png")
