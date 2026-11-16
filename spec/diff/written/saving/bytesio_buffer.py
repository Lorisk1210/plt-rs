# Render a chart into memory (for an email attachment) instead of a file, then check the bytes.
import io
import matplotlib.pyplot as plt
from PIL import Image

plt.rcParams["svg.hashsalt"] = "report"

fig, ax = plt.subplots(figsize=(4, 2.5), dpi=80)
ax.barh(["north", "south", "east", "west"], [23, 17, 35, 29], color="slateblue")
ax.set_xlabel("tickets")
fig.tight_layout()

buf = io.BytesIO()
fig.savefig(buf, format="png")
data = buf.getvalue()
print(data[:8])
buf.seek(0)
img = Image.open(buf)
print(img.size, img.mode)

svg = io.StringIO()
fig.savefig(svg, format="svg", metadata={"Date": None})
text = svg.getvalue()
print(text.lstrip().startswith("<?xml"), "<svg" in text)

pdf = io.BytesIO()
fig.savefig(pdf, format="pdf")
print(pdf.getvalue()[:5])
