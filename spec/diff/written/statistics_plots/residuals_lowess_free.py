# Residual diagnostics: residuals vs fitted with a moving-average trend and a scale-location panel.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
x = np.linspace(1, 10, 120)
y = 0.5 * x ** 2 + rng.normal(0, x, 120)
coef = np.polyfit(x, y, 1)
fitted = np.polyval(coef, x)
resid = y - fitted
order = np.argsort(fitted)
trend = np.convolve(resid[order], np.ones(15) / 15, mode="valid")
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.scatter(fitted, resid, s=8)
a.plot(fitted[order][7:-7], trend, "r")
a.axhline(0, color="k", lw=0.5)
a.set_title("residuals vs fitted")
b.scatter(fitted, np.sqrt(np.abs(resid / resid.std())), s=8)
b.set_title("scale-location")
print(np.round(coef, 4))
fig.savefig("residuals.png")
