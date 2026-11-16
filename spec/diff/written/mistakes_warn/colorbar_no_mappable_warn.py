# plt.colorbar() with no mappable raises; with a mappable but no ax= in an add_axes figure it works.
import warnings
import numpy as np
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([1, 2, 3])
try:
    plt.colorbar()
except RuntimeError as err:
    print("RuntimeError:", str(err)[:90])
im = ax.imshow(np.eye(3), extent=(1, 3, 1, 3))
cb = plt.colorbar(im)
print(type(cb).__name__)
fig.savefig("colorbar_err.png")
