# numpy masked arrays in scatter, bar heights and fill_between, and how masked points are skipped.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(8)
y = np.ma.masked_array([1, 3, 2, 5, 4, 6, 5, 7], mask=[0, 0, 1, 0, 0, 1, 0, 0])
fig, axs = plt.subplots(1, 3, figsize=(10, 3))
axs[0].scatter(x, y)
axs[0].set_title("scatter")
axs[1].bar(x, np.ma.filled(y, 0))
axs[1].set_title("bar filled")
axs[2].fill_between(x, 0, y, alpha=0.4)
axs[2].plot(x, y, "k.-")
axs[2].set_title("fill_between")
print(len(axs[0].collections[0].get_offsets()), int(y.count()))
fig.savefig("masked.png")
