# Images in PDF: XObject widths and heights, colour spaces, /SMask for alpha, and image.composite_image on and off.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def images(rc, alpha=None):
    with plt.rc_context(rc):
        fig, ax = plt.subplots(figsize=(3, 3), dpi=72)
        ax.imshow(np.arange(12).reshape(3, 4), extent=(0, 1, 0, 1), alpha=alpha)
        ax.imshow(np.ones((2, 2, 3)) * [1, 0, 0], extent=(1, 2, 1, 2))
        ax.set_xlim(0, 2)
        ax.set_ylim(0, 2)
        buf = io.BytesIO()
        fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
        plt.close(fig)
    data = buf.getvalue()
    objs = re.findall(rb"/Subtype /Image(.*?)>>", data, re.S)
    out = []
    for o in objs:
        w = re.search(rb"/Width (\d+)", o)
        h = re.search(rb"/Height (\d+)", o)
        cs = re.search(rb"/ColorSpace /(\w+)", o)
        out.append((int(w.group(1)), int(h.group(1)), cs.group(1).decode() if cs else None, b"/SMask" in o))
    return out


print(images({}))
print(images({}, alpha=0.5))
print(images({"image.composite_image": False}))
print(images({"image.composite_image": True, "image.interpolation": "nearest"}))
print(images({"savefig.dpi": 300}))
