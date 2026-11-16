# numpy timedelta64 arrays as x: converted to floats of a chosen unit before plotting.
import numpy as np
import matplotlib.pyplot as plt

elapsed = np.array([0, 90, 300, 720, 1500, 3000], dtype="timedelta64[s]")
size = np.array([1, 1.4, 2.2, 3.5, 5.0, 7.1])
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(elapsed / np.timedelta64(1, "m"), size, "o-")
ax.set_xlabel("elapsed [min]")
ax.set_ylabel("colony size [mm]")
try:
    ax.plot(elapsed, size)
    print("timedelta64 plotted", ax.lines[-1].get_xdata().dtype)
except Exception as err:
    print(type(err).__name__, str(err)[:80])
fig.savefig("elapsed.png")
