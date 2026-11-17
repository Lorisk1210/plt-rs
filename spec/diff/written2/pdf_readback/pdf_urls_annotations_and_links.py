# url= on lines, text, patches and images becomes /Link annotations in the PDF; count them and read their rectangles and URIs.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

fig, ax = plt.subplots(figsize=(4, 3))
ax.text(0.1, 0.9, "text link", url="https://example.org/text")
ax.text(0.5, 0.5, "rotated link", rotation=45, url="https://example.org/rotated")
ax.add_patch(Rectangle((0.6, 0.1), 0.2, 0.2, url="https://example.org/rect"))
ax.imshow(np.eye(2), extent=(0, 0.2, 0, 0.2), url="https://example.org/image")
ax.plot([0, 1], [1, 0], url="https://example.org/line")
buf = io.BytesIO()
fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
data = buf.getvalue()
uris = re.findall(rb"/URI \(([^)]*)\)", data)
print(sorted(u.decode() for u in uris))
rects = re.findall(rb"/Subtype /Link.*?/Rect \[\s*([-0-9. ]+)\]", data, re.S)
print(len(re.findall(rb"/Subtype /Link", data)), len(rects))
print([[round(float(v), 1) for v in r.split()] for r in rects][:3])
print(len(re.findall(rb"/QuadPoints", data)))
print(b"/Annots" in data)
fig.savefig("links.pdf", metadata={"CreationDate": None})
with open("links.pdf", "rb") as f:
    print(f.read() == data)
fig2, ax2 = plt.subplots()
ax2.set_url("https://example.org/axes")
ax2.set_title("t", url="https://example.org/title")
buf2 = io.BytesIO()
fig2.savefig(buf2, format="pdf", metadata={"CreationDate": None})
print(sorted(u.decode() for u in re.findall(rb"/URI \(([^)]*)\)", buf2.getvalue())))
