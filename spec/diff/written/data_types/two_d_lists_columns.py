# plot() with a 2-D array: each column becomes a line; label with a list of labels.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 20)
Y = np.column_stack([x, x ** 2, x ** 3])
fig, ax = plt.subplots(figsize=(5, 3.5))
lines = ax.plot(x, Y, label=["linear", "square", "cube"])
ax.legend()
print(len(lines), [l.get_label() for l in lines])
lines2 = ax.plot(Y[:, :2] + 1)
print(len(lines2), lines2[0].get_xdata()[:3])
fig.savefig("columns.png")
