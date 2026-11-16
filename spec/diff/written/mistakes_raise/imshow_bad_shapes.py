# imshow with arrays of invalid shape or dtype: the TypeError text, then the reshaped fix.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
for arr in [np.arange(10), np.zeros((4, 4, 2)), np.zeros((3, 3), dtype=object), np.zeros((2, 2, 5))]:
    try:
        ax.imshow(arr)
        print(arr.shape, "ok")
    except TypeError as err:
        print(arr.shape, "TypeError:", err)
ax.imshow(np.arange(10).reshape(2, 5))
fig.savefig("reshaped.png")
