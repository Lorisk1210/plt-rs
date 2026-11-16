# Different numpy dtypes as data: int8, uint16, float16, float32, bool, and big ints near overflow.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(10)
arrays = {"int8": np.arange(10, dtype=np.int8) * 12, "uint16": np.arange(10, dtype=np.uint16) * 7000,
          "float16": np.linspace(0, 1, 10, dtype=np.float16), "float32": np.linspace(0, 1, 10, dtype=np.float32) ** 2,
          "bool": np.arange(10) % 3 == 0, "int64 big": np.arange(10, dtype=np.int64) * 10 ** 15}
fig, axs = plt.subplots(2, 3, figsize=(10, 5))
for ax, (name, arr) in zip(axs.flat, arrays.items()):
    ax.plot(x, arr, "o-")
    ax.set_title(name)
    print(name, ax.lines[0].get_ydata().dtype, ax.get_ylim()[1] > 0)
fig.tight_layout()
fig.savefig("dtypes.png")
