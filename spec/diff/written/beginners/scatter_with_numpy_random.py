# Scatter of random points from np.random with a fixed seed, sized and coloured.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(0)
x = np.random.rand(50)
y = np.random.rand(50)
sizes = 300 * np.random.rand(50)
colors = np.random.rand(50)
plt.scatter(x, y, s=sizes, c=colors, alpha=0.5)
plt.colorbar()
plt.title("Random bubbles")
plt.savefig("bubbles.png")
print(round(float(x.mean()), 6), round(float(sizes.max()), 4))
