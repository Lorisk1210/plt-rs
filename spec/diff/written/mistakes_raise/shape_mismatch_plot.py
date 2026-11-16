# x and y of different lengths: the ValueError users google, then the fix by trimming.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(10)
y = np.arange(9) ** 2
fig, ax = plt.subplots(figsize=(4, 3))
try:
    ax.plot(x, y)
except ValueError as err:
    print("ValueError:", err)
n = min(len(x), len(y))
ax.plot(x[:n], y[:n])
try:
    ax.scatter(x, y)
except ValueError as err:
    print("ValueError:", err)
fig.savefig("trimmed.png")
