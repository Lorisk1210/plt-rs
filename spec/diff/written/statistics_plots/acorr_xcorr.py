# Autocorrelation and cross-correlation of two signals with Axes.acorr and Axes.xcorr.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(9)
t = np.arange(200)
a = np.sin(t / 6) + rng.normal(0, 0.5, 200)
b = np.roll(a, 7) + rng.normal(0, 0.5, 200)
fig, (l, r) = plt.subplots(1, 2, figsize=(9, 3.5))
lags, c, *_ = l.acorr(a, maxlags=40, usevlines=True, normed=True)
l.set_title("autocorrelation")
lags2, c2, *_ = r.xcorr(a, b, maxlags=20, usevlines=False, marker="o", linestyle="-")
r.set_title("cross-correlation")
print(int(lags2[np.argmax(c2)]), round(float(c[40]), 4))
fig.savefig("correlations.png")
