# Rasterized artists inside an SVG (mixed mode): how many <image> elements, their sizes at savefig dpi, and the vector rest.
import base64
import io
import xml.etree.ElementTree as ET
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from PIL import Image

plt.rcParams["svg.hashsalt"] = "raster"
fig, axs = plt.subplots(1, 2, figsize=(5, 2.5))
x = np.linspace(0, 1, 2000)
axs[0].plot(x, np.sin(40 * x), rasterized=True)
axs[0].plot(x, np.cos(40 * x))
axs[1].scatter(x[::10], np.sin(x[::10] * 9), rasterized=True, s=3)
axs[1].set_rasterization_zorder(5)
axs[1].fill_between(x, 0, 0.5, zorder=1)
axs[1].text(0.5, 0.5, "vector text", zorder=10)
S = "{http://www.w3.org/2000/svg}"
for dpi in (72, 150):
    buf = io.BytesIO()
    fig.savefig(buf, format="svg", dpi=dpi, metadata={"Date": None})
    root = ET.fromstring(buf.getvalue())
    images = list(root.iter(S + "image"))
    sizes = []
    for im in images:
        data = im.get("{http://www.w3.org/1999/xlink}href").split(",", 1)[1]
        with Image.open(io.BytesIO(base64.b64decode(data))) as png:
            sizes.append(png.size)
    print(dpi, len(images), sizes, [(im.get("width"), im.get("height")) for im in images])
    print(dpi, len(list(root.iter(S + "path"))) > 10, len(buf.getvalue()))
print([a.get_rasterized() for a in axs[0].lines], axs[1].get_rasterization_zorder())
