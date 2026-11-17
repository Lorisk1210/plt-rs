# Colormap __call__ with floats, ints, arrays, bytes=True, alpha scalars/arrays, NaN and out-of-range; equality and hashing.
import numpy as np
import matplotlib

cmap = matplotlib.colormaps["coolwarm"]
print(np.round(cmap(0.5), 4).tolist(), np.round(cmap(128), 4).tolist(), cmap(0.5) == cmap(128))
print(cmap(np.array([0.0, 1.0]), bytes=True).tolist())
print(np.round(cmap([0.25, 0.75], alpha=0.3), 4).tolist())
print(np.round(cmap([0.25, 0.75], alpha=[0.1, 0.9]), 4).tolist())
print(np.round(cmap(np.nan), 4).tolist(), np.round(cmap(-5.0), 4).tolist(), np.round(cmap(5.0), 4).tolist())
print(np.round(cmap(-1), 4).tolist(), np.round(cmap(1000), 4).tolist())
print(cmap(np.ma.masked_array([0.1, 0.2], [0, 1])).round(4).tolist())
arr = cmap(np.linspace(0, 1, 6).reshape(2, 3))
print(arr.shape, arr.dtype)
print(cmap(np.array([0, 255], dtype=np.uint8), bytes=True).tolist())
print(cmap(np.float32(0.5)).__class__.__name__, cmap(0.5).__class__.__name__)
copy = cmap.copy()
print(copy == cmap, copy is cmap, copy.name)
copy.set_under("k")
print(copy == cmap)
try:
    cmap([0.1], alpha=[0.1, 0.2])
except ValueError as e:
    print("ValueError", str(e)[:60])
try:
    hash(cmap)
    print("hashable")
except TypeError as e:
    print("TypeError", str(e)[:40])
print(cmap.is_gray(), matplotlib.colormaps["gray"].is_gray(), cmap._i_under, cmap._i_over, cmap._i_bad)
