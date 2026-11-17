# imshow of RGB/RGBA arrays in float and uint8, out-of-range values, masked RGB, and how alpha= combines with an alpha channel.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

cases = {
    "float rgb": np.dstack([np.linspace(0, 1, 9).reshape(3, 3)] * 3),
    "uint8 rgb": (np.dstack([np.linspace(0, 1, 9).reshape(3, 3)] * 3) * 255).astype(np.uint8),
    "float rgba": np.dstack([np.ones((3, 3)), np.zeros((3, 3)), np.zeros((3, 3)), np.linspace(0, 1, 9).reshape(3, 3)]),
    "out of range": np.dstack([np.linspace(-1, 2, 9).reshape(3, 3)] * 3),
    "uint16 rgb": (np.dstack([np.linspace(0, 1, 9).reshape(3, 3)] * 3) * 300).astype(np.uint16),
}
fig, axs = plt.subplots(1, len(cases) + 1, figsize=(9, 2), dpi=40)
for ax, (name, data) in zip(axs, cases.items()):
    try:
        im = ax.imshow(data, interpolation="nearest")
        fig.canvas.draw()
        arr = im.get_array()
        print(name, arr.dtype, arr.shape, im.get_alpha(), np.round(im.to_rgba(arr)[1, 1] if arr.ndim == 3 else 0, 3).tolist())
    except Exception as e:
        print(name, type(e).__name__, str(e)[:60])
im = axs[-1].imshow(cases["float rgba"], alpha=0.5)
fig.canvas.draw()
print(im.get_alpha(), np.round(im.make_image(fig.canvas.get_renderer(), unsampled=True)[0][1, 1], 3).tolist())
im.set_alpha(np.full((3, 3), 0.25))
fig.canvas.draw()
print(np.asarray(im.get_alpha()).shape)
buf = io.BytesIO()
fig.savefig(buf, format="png")
print(len(buf.getvalue()) > 500)
