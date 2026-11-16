# A helper fig_to_array used in ML notebooks to log plots as arrays, via savefig into BytesIO.
import io
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image


def fig_to_array(fig, dpi=60):
    buf = io.BytesIO()
    fig.savefig(buf, format="png", dpi=dpi)
    buf.seek(0)
    return np.array(Image.open(buf))


losses = 1 / np.arange(1, 30)
fig, ax = plt.subplots(figsize=(4, 3))
ax.plot(losses)
ax.set_title("loss")
arr = fig_to_array(fig)
print(arr.shape, arr.dtype, int(arr[..., 3].min()))
fig2, ax2 = plt.subplots(figsize=(4, 3))
ax2.imshow(arr)
ax2.axis("off")
fig2.savefig("logged.png")
