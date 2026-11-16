# Histogram of exam scores normalised as a density with the fitted normal curve on top.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(19)
scores = np.random.normal(68, 11, 600).clip(0, 100)
mu, sigma = scores.mean(), scores.std()
plt.figure(figsize=(6, 4))
counts, bins, patches = plt.hist(scores, bins=30, density=True, alpha=0.7, color="skyblue", edgecolor="k")
x = np.linspace(0, 100, 200)
plt.plot(x, np.exp(-(x - mu) ** 2 / (2 * sigma ** 2)) / (sigma * np.sqrt(2 * np.pi)), "r", lw=2)
plt.title(rf"$\mu={mu:.1f}$, $\sigma={sigma:.1f}$")
plt.xlabel("score")
print(len(patches), round(float((counts * np.diff(bins)).sum()), 6))
plt.savefig("scores.png")
