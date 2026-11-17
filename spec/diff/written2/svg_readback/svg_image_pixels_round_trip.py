# The image embedded in an SVG decoded back to pixels and compared with the data's colormapped RGBA at interpolation none.
import base64
import io
import xml.etree.ElementTree as ET
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from PIL import Image

plt.rcParams["svg.hashsalt"] = "img"
data = np.add.outer(np.arange(6), np.arange(8)) % 5
fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
im = ax.imshow(data, cmap="viridis", interpolation="none")
ax.set_axis_off()
for interp in ("none", "nearest", "bilinear"):
    im.set_interpolation(interp)
    buf = io.BytesIO()
    fig.savefig(buf, format="svg", metadata={"Date": None})
    root = ET.fromstring(buf.getvalue())
    el = next(root.iter("{http://www.w3.org/2000/svg}image"))
    png = Image.open(io.BytesIO(base64.b64decode(el.get("{http://www.w3.org/1999/xlink}href").split(",", 1)[1])))
    arr = np.asarray(png.convert("RGBA"))
    print(interp, png.size, el.get("width"), el.get("height"), el.get("transform") is not None, el.get("style"))
    if interp == "none":
        expected = (im.cmap(im.norm(data)) * 255).round().astype(np.uint8)
        print(arr.shape == expected.shape, int(np.abs(arr.astype(int) - expected.astype(int)).max()))
im.set_alpha(0.5)
buf = io.BytesIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
el = next(ET.fromstring(buf.getvalue()).iter("{http://www.w3.org/2000/svg}image"))
print(el.get("style"), el.get("opacity"))
