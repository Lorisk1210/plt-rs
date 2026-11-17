# imsave/imread round trips for float, uint8, RGBA and 16-bit-like data with cmaps, vmin/vmax, origin, dpi and metadata.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.image as mimage
from PIL import Image

gray = np.linspace(0, 1, 48).reshape(6, 8)
mimage.imsave("gray.png", gray, cmap="gray")
back = mimage.imread("gray.png")
print(back.shape, back.dtype, np.round(back[0, :3, 0], 4).tolist())
mimage.imsave("viridis_lower.png", gray, cmap="viridis", origin="lower", vmin=0.2, vmax=0.8)
b2 = mimage.imread("viridis_lower.png")
print(b2.shape, np.round(b2[-1, 0], 4).tolist(), np.round(b2[0, -1], 4).tolist())
rgba = np.zeros((4, 4, 4), np.uint8)
rgba[..., 0] = 255
rgba[..., 3] = np.arange(16).reshape(4, 4) * 16
mimage.imsave("rgba.png", rgba)
b3 = mimage.imread("rgba.png")
print(b3.dtype, np.round(b3[1, 1], 4).tolist())
mimage.imsave("meta.png", gray, metadata={"Software": "written2", "Title": "t"}, dpi=150)
with Image.open("meta.png") as im:
    print(im.info.get("Software"), im.info.get("Title"), tuple(round(v) for v in im.info.get("dpi", (0, 0))))
mimage.imsave("as.jpg", gray, cmap="gray", pil_kwargs={"quality": 95})
with Image.open("as.jpg") as im:
    print(im.format, im.mode, im.size)
mimage.imsave("as.tiff", gray)
print(mimage.imread("as.tiff").shape)
u8 = (gray * 255).astype(np.uint8)
mimage.imsave("u8.png", u8, cmap="gray", vmin=0, vmax=255)
print(np.round(mimage.imread("u8.png")[0, :4, 0] * 255).astype(int).tolist(), u8[0, :4].tolist())
pil = mimage.pil_to_array(Image.fromarray(u8))
print(pil.shape, pil.dtype)
try:
    mimage.imsave("bad.png", np.zeros((2, 2, 5)))
except ValueError as e:
    print("ValueError", str(e)[:60])
