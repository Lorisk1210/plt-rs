# Inputs of every old form: plain lists, tuples, numpy matrices, masked arrays, integer arrays and range objects, through common calls.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(8, 5))
inputs = {"list": [1, 3, 2, 4], "tuple": (1, 3, 2, 4), "range": range(1, 5), "int array": np.array([1, 3, 2, 4]),
          "masked": np.ma.masked_array([1, 3, 2, 4], [0, 1, 0, 0]), "matrix": np.asmatrix([[1, 3, 2, 4]])}
for ax, (name, data) in zip(axs.flat, inputs.items()):
    try:
        ln, = ax.plot(data if name != "matrix" else np.asarray(data).ravel())
        bars = ax.bar(range(4), np.asarray(data).ravel() if name == "matrix" else data)
        sc = ax.scatter(range(4), np.asarray(data).ravel() if name == "matrix" else data)
        h = ax.hist(np.asarray(data).ravel() if name == "matrix" else data, bins=3)
        print(name, ln.get_ydata().dtype.kind, type(ln.get_ydata(orig=True)).__name__, [b.get_height() for b in bars],
              np.ma.isMaskedArray(sc.get_offsets()), h[0].tolist())
    except Exception as e:
        print(name, type(e).__name__, str(e)[:60])
try:
    plt.figure().add_subplot().plot(np.asmatrix([[1, 2], [3, 4]]))
    print("matrix plotted")
except Exception as e:
    print("matrix", type(e).__name__)
fig.savefig("inputs.png")
print([round(v, 2) for v in axs[1, 1].get_ylim()])
ax = axs[0, 0]
ax.fill_between(range(4), [0, 0, 0, 0], (1, 2, 3, 4))
ax.errorbar([0, 1], (1, 2), yerr=[0.1, 0.2])
print(len(ax.collections), len(ax.containers))
