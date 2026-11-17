# Gouraud-shaded pcolormesh and tripcolor in PDF: shading dictionaries (/ShadingType 4) and their streams.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.tri as mtri

fig, axs = plt.subplots(1, 3, figsize=(7, 2.5))
x, y = np.meshgrid(np.linspace(0, 1, 5), np.linspace(0, 1, 4))
z = x * y
axs[0].pcolormesh(x, y, z, shading="gouraud")
axs[1].pcolormesh(x, y, z[:-1, :-1], shading="flat")
tri = mtri.Triangulation(x.ravel(), y.ravel())
axs[2].tripcolor(tri, z.ravel(), shading="gouraud")
buf = io.BytesIO()
fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
data = buf.getvalue()
print(len(re.findall(rb"/ShadingType 4", data)), len(re.findall(rb"/Type /Pattern", data)) >= 0)
print(sorted(set(re.findall(rb"/BitsPerCoordinate (\d+)", data))), sorted(set(re.findall(rb"/BitsPerComponent (\d+)", data))))
print(sorted(set(re.findall(rb"/BitsPerFlag (\d+)", data))), len(re.findall(rb"/Decode \[", data)) > 0)
print(len(tri.triangles), tri.triangles.shape)
with plt.rc_context({"pdf.compression": 0}):
    buf2 = io.BytesIO()
    fig.savefig(buf2, format="pdf", metadata={"CreationDate": None})
    print(len(re.findall(rb" sh\b", buf2.getvalue())), len(buf2.getvalue()) > len(data))
fig.savefig("gouraud.pdf", metadata={"CreationDate": None})
fig.savefig("gouraud.png")
print(len(axs[0].collections), type(axs[0].collections[0]).__name__, axs[0].collections[0]._shading)
