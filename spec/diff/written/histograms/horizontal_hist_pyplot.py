# Horizontal histograms back to back (population pyramid style) with pyplot only.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(2)
men = np.random.normal(40, 18, 2000).clip(0, 99)
women = np.random.normal(43, 19, 2000).clip(0, 99)
bins = np.arange(0, 101, 5)
m, _ = np.histogram(men, bins)
w, _ = np.histogram(women, bins)
plt.figure(figsize=(6, 5))
plt.barh(bins[:-1], -m, height=4.5, align="edge", color="steelblue", label="men")
plt.barh(bins[:-1], w, height=4.5, align="edge", color="lightcoral", label="women")
plt.xticks([-200, -100, 0, 100, 200], ["200", "100", "0", "100", "200"])
plt.ylabel("age")
plt.legend()
plt.title("Population pyramid")
print(m.sum(), w.sum())
plt.savefig("pyramid.png")
