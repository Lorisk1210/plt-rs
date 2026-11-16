# All-NaN data and empty arrays: what plot, imshow and hist do (warnings or not) with them.
import warnings
import numpy as np
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, axs = plt.subplots(1, 3, figsize=(9, 3))
with warnings.catch_warnings(record=True) as w:
    axs[0].plot([np.nan, np.nan], [np.nan, np.nan])
    axs[1].imshow(np.full((3, 3), np.nan))
    axs[2].hist([], bins=5)
    fig.canvas.draw()
print(sorted({type(x.message).__name__ for x in w}))
print([round(v, 3) for v in axs[0].get_xlim()], axs[2].get_ylim())
fig.savefig("empty.png")
