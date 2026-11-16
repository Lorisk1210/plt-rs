# Data with NaNs and outliers: users filter NaN, set range, and see the outlier bin counts.
import numpy as np
import matplotlib.pyplot as plt

values = np.array([1.2, 2.5, np.nan, 3.1, 2.2, 50.0, 2.9, np.nan, 3.3, 1.8, 2.7, -40.0, 2.4])
clean = values[~np.isnan(values)]
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.2))
n1, e1, _ = a.hist(clean, bins=10)
a.set_title("all finite")
n2, e2, _ = b.hist(clean, bins=10, range=(0, 5))
b.set_title("range (0, 5)")
print(n1.sum(), n2.sum(), np.round(e2[:3], 2))
fig.tight_layout()
fig.savefig("nan_hist.png")
