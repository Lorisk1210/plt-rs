# A script that ends in an uncaught error (bar heights shorter than the categories), as users run it.
import numpy as np
import matplotlib.pyplot as plt

labels = ["a", "b", "c", "d"]
fig, ax = plt.subplots(figsize=(4, 3))
ax.set_title("bars that will not draw")
ax.plot([0, 1], [0, 1])
ax.set_xlabel("category")
ax.set_ylabel("count")
fig.savefig("before_error.png")
ax.bar(labels, np.array([1, 2, 3]))
fig.savefig("never.png")
