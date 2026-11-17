# imshow of int8/uint16/int64/float16/bool/complex data with norms; the dtype stored, clim chosen and rendered corner colour.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

base = np.arange(16).reshape(4, 4)
arrays = {"int8": base.astype(np.int8) - 8, "uint16": base.astype(np.uint16) * 4000, "int64": base.astype(np.int64) * 10**12,
          "float16": base.astype(np.float16) / 3, "bool": base % 2 == 0, "float32": base.astype(np.float32) * 1e30,
          "longdouble": base.astype(np.longdouble) / 7}
fig, ax = plt.subplots(figsize=(2, 2), dpi=30)
for name, arr in arrays.items():
    for stage in ("data", "rgba"):
        im = ax.imshow(arr, cmap="gray", interpolation="bilinear", interpolation_stage=stage)
        fig.canvas.draw()
        out, *_ = im.make_image(fig.canvas.get_renderer(), unsampled=True)
        print(name, stage, im.get_array().dtype, [round(float(v), 4) for v in im.get_clim()], out[0, 0, :3].tolist())
        im.remove()
try:
    ax.imshow(np.ones((2, 2), complex))
    fig.canvas.draw()
    print("complex accepted")
except TypeError as e:
    print("TypeError", str(e)[:60])
try:
    ax.imshow(np.array([["a", "b"]]))
    fig.canvas.draw()
except TypeError as e:
    print("TypeError", str(e)[:60])
im = ax.imshow(np.full((3, 3), 5.0))
fig.canvas.draw()
print(im.get_clim(), im.norm.vmin, im.norm.vmax)
