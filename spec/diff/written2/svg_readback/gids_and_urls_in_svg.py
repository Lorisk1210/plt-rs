# Set gid and url on many artists, save SVG and parse it with ElementTree: find each gid, its element and the link targets.
import io
import xml.etree.ElementTree as ET
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "gids"
fig, ax = plt.subplots(figsize=(4, 3))
ln, = ax.plot([0, 1], [0, 1], gid="the-line", url="https://example.org/line")
sc = ax.scatter([0.2, 0.8], [0.5, 0.5], gid="the-dots", urls=["https://example.org/a", "https://example.org/b"])
bars = ax.bar([0.3, 0.6], [0.2, 0.4], width=0.1)
for k, b in enumerate(bars):
    b.set_gid(f"bar-{k}")
    b.set_url(f"https://example.org/bar/{k}")
t = ax.text(0.5, 0.9, "linked", gid="the-text", url="https://example.org/text")
im = ax.imshow(np.eye(3), extent=(0, 0.2, 0, 0.2), gid="the-image")
ax.set_gid("main-axes")
fig.set_gid("the-figure")
buf = io.BytesIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
root = ET.fromstring(buf.getvalue())
ns = {"svg": "http://www.w3.org/2000/svg", "xlink": "http://www.w3.org/1999/xlink"}
ids = [el.get("id") for el in root.iter() if el.get("id")]
for gid in ["the-figure", "main-axes", "the-line", "the-dots", "bar-0", "bar-1", "the-text", "the-image"]:
    el = next((e for e in root.iter() if e.get("id") == gid), None)
    print(gid, None if el is None else el.tag.split("}")[1], None if el is None else len(list(el)))
links = [a.get("{http://www.w3.org/1999/xlink}href") for a in root.iter("{http://www.w3.org/2000/svg}a")]
print(sorted(links))
print(len(ids), len(set(ids)) == len(ids))
print(root.get("width"), root.get("height"), root.get("viewBox"))
