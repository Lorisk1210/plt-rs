# Count defs in an SVG: clip paths, markers reused through <use>, patterns for hatches and images, with hashsalt for stable ids.
import io
import re
import xml.etree.ElementTree as ET
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "defs"
fig, axs = plt.subplots(1, 2, figsize=(6, 3))
axs[0].plot(np.arange(20), np.arange(20) % 5, "o-", ms=4)
axs[0].plot(np.arange(20), np.arange(20) % 3, "s", ms=4)
axs[1].bar(range(3), [1, 2, 3], hatch="//", fill=False)
axs[1].imshow(np.eye(4), extent=(0, 1, 0, 1))
buf = io.BytesIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
text = buf.getvalue().decode()
root = ET.fromstring(buf.getvalue())
S = "{http://www.w3.org/2000/svg}"
print(len(list(root.iter(S + "clipPath"))), len(list(root.iter(S + "use"))), len(list(root.iter(S + "pattern"))),
      len(list(root.iter(S + "image"))))
marker_defs = [p.get("id") for p in root.iter(S + "path") if p.get("id", "").startswith("m")]
print(len(marker_defs), sorted({len(m) for m in marker_defs}))
uses = [u.get("{http://www.w3.org/1999/xlink}href") for u in root.iter(S + "use")]
print(sorted({u for u in uses if u}) == sorted({"#" + m for m in marker_defs if "#" + m in uses}))
clips = re.findall(r'clip-path="url\(#([^)]+)\)"', text)
print(len(clips), len(set(clips)))
img = next(root.iter(S + "image"))
print(img.get("{http://www.w3.org/1999/xlink}href")[:22], img.get("width"), img.get("height"))
buf2 = io.BytesIO()
fig.savefig(buf2, format="svg", metadata={"Date": None})
print(buf2.getvalue() == buf.getvalue())
with plt.rc_context({"svg.image_inline": False}):
    fig.savefig("external.svg", metadata={"Date": None})
import os
print(sorted(f for f in os.listdir(".") if f.startswith("external")))
