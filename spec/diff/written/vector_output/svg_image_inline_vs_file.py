# SVG with an embedded image: inline base64 (default) vs svg.image_inline=False (separate files).
import os
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(3, 3))
ax.imshow(np.arange(16).reshape(4, 4), cmap="viridis")
with plt.rc_context({"svg.hashsalt": "a"}):
    fig.savefig("inline.svg", metadata={"Date": None})
    with plt.rc_context({"svg.image_inline": False}):
        fig.savefig("external.svg", metadata={"Date": None})
with open("inline.svg") as f:
    print("base64" in f.read())
print(sorted(n for n in os.listdir(".") if n.startswith("external")))
