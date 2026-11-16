# Positional arguments where keywords are expected, and calls passing redundant keywords that warn.
import warnings
import numpy as np
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([1, 2, 3], [1, 2, 3], "r", color="b")
ax.plot([1, 2, 3], [3, 2, 1], "o-", linestyle=":")
try:
    ax.scatter([1, 2], [1, 2], c="r", color="b")
except ValueError as err:
    print("ValueError:", err)
ax.legend(["first", "second"])
x = np.arange(3)
ax.bar(x, [1, 2, 3], 0.4, 0)
fig.savefig("redundant.png")
print([l.get_color() for l in ax.lines])
