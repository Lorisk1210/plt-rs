# Scatter with NaN colour values: hidden by default, shown with plotnonfinite and set_bad colour.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(10)
y = x ** 0.5
c = np.array([1, 2, np.nan, 4, 5, np.nan, 7, 8, 9, 10], dtype=float)
cmap = plt.get_cmap("viridis").with_extremes(bad="red")
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.scatter(x, y, c=c, cmap=cmap, s=60)
a.set_title("default")
b.scatter(x, y, c=c, cmap=cmap, s=60, plotnonfinite=True)
b.set_title("plotnonfinite=True")
print(len(a.collections[0].get_offsets()), len(b.collections[0].get_offsets()))
fig.savefig("nan_colors.png")
