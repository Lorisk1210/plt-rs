# With pdf.compression 0, read the page content stream operators: paths, fills, strokes, colours, dashes, text and images.
import io
import re
import collections
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["pdf.compression"] = 0
fig, ax = plt.subplots(figsize=(3, 2))
ax.plot([0, 1], [0, 1], "r--", lw=2)
ax.fill([0, 1, 1], [0, 0, 1], color=(0, 0.5, 1, 0.4))
ax.text(0.5, 0.5, "Hi")
ax.imshow(np.eye(3), extent=(0, 0.3, 0, 0.3))
buf = io.BytesIO()
fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
data = buf.getvalue()
streams = re.findall(rb"stream\r?\n(.*?)\r?\nendstream", data, re.S)
content = max(streams, key=lambda s: s.count(b" m"))
ops = collections.Counter(re.findall(rb"(?<![A-Za-z/])(m|l|c|re|h|f|S|B|W|n|q|Q|cm|Tf|Td|Tj|TJ|BT|ET|d|w|RG|rg|gs|Do|J|j)(?=\s)", content))
print(sorted((k.decode(), v) for k, v in ops.items()))
print(re.findall(rb"\[([0-9. ]+)\] ([0-9.]+) d", content)[:2])
print(re.findall(rb"([0-9.]+ [0-9.]+ [0-9.]+) RG", content)[:3])
print(len(re.findall(rb"/Type /XObject", data)), len(re.findall(rb"/Subtype /Image", data)))
print(sorted(set(re.findall(rb"/CA ([0-9.]+)", data))), sorted(set(re.findall(rb"/ca ([0-9.]+)", data))))
print(len(streams), data.count(b"/FlateDecode"))
print(re.findall(rb"/(DejaVu\w*)", data)[:1], content.count(b"BT") == content.count(b"ET"))
ax.lines[0].set_linestyle(":")
buf2 = io.BytesIO()
fig.savefig(buf2, format="pdf", metadata={"CreationDate": None})
print(re.findall(rb"\[([0-9. ]+)\] ([0-9.]+) d", buf2.getvalue())[:2])
