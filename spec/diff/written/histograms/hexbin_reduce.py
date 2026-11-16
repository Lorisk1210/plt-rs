# Hexbin of positions coloured by the mean of a third value (reduce_C_function) with mincnt.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(16)
x = rng.uniform(-3, 3, 8000)
y = rng.uniform(-3, 3, 8000)
z = np.sin(x) * np.cos(y)
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4))
hb = a.hexbin(x, y, gridsize=25, cmap="Greens", mincnt=1)
fig.colorbar(hb, ax=a, label="count")
hb2 = b.hexbin(x, y, C=z, gridsize=25, reduce_C_function=np.mean, cmap="RdBu")
fig.colorbar(hb2, ax=b, label="mean z")
print(hb.get_array().shape, round(float(hb2.get_array().max()), 4))
fig.savefig("hexbin.png")
