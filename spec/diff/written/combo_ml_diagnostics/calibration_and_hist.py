# Calibration plot with binned reliability, perfect-calibration line, and a histogram of predicted probabilities below.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(46)
p = rng.beta(2, 2, 2000)
y = rng.random(2000) < p ** 1.3
bins = np.linspace(0, 1, 11)
idx = np.digitize(p, bins) - 1
centers = (bins[:-1] + bins[1:]) / 2
frac = np.array([y[idx == i].mean() for i in range(10)])
fig, (a, b) = plt.subplots(2, 1, figsize=(5.5, 7), sharex=True, gridspec_kw={"height_ratios": [3, 1]})
a.plot([0, 1], [0, 1], "k--", label="perfect")
a.plot(centers, frac, "s-", label="model")
a.set_ylabel("fraction positive")
a.legend()
a.set_aspect("equal")
b.hist(p, bins=bins, color="C0", edgecolor="w")
b.set_xlabel("predicted probability")
print(np.round(frac, 3).tolist())
fig.savefig("calibration.png", dpi=50)
