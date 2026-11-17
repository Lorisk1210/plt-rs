# Write SVG to a path, a .svgz path, a text stream and a binary stream; decompress the svgz and compare documents.
import gzip
import io
import os
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "files"
fig, ax = plt.subplots(figsize=(3, 2))
ax.plot([0, 1, 2], [2, 0, 1])
fig.savefig("plain.svg", metadata={"Date": None})
fig.savefig("compressed.svgz", metadata={"Date": None})
fig.savefig("explicit", format="svgz", metadata={"Date": None})
text = io.StringIO()
fig.savefig(text, format="svg", metadata={"Date": None})
binary = io.BytesIO()
fig.savefig(binary, format="svg", metadata={"Date": None})
with open("plain.svg", "rb") as f:
    plain = f.read()
with gzip.open("compressed.svgz") as f:
    unz = f.read()
with open("compressed.svgz", "rb") as f:
    print(f.read(2) == b"\x1f\x8b")
print(plain == unz, plain == binary.getvalue(), plain.decode() == text.getvalue())
print(sorted(f for f in os.listdir(".") if f.startswith(("plain", "compressed", "explicit"))))
with open("explicit", "rb") as f:
    print(f.read(2) == b"\x1f\x8b")
with open("handle.svg", "w") as f:
    fig.savefig(f, format="svg", metadata={"Date": None})
with open("handle.svg", "rb") as f:
    print(f.read() == plain)
try:
    fig.savefig(io.StringIO(), format="svgz")
except Exception as e:
    print(type(e).__name__)
print(len(plain) > 1000, plain.count(b"\n") > 20)
