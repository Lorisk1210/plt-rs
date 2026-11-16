# Save PNG with custom text metadata, read it back with Pillow, and print the key/value pairs plus image mode and dpi.
import matplotlib.pyplot as plt
from PIL import Image

fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
ax.scatter([1, 2, 3], [3, 1, 2])
ax.set_title("tagged")
fig.savefig("tagged.png", dpi=144, metadata={"Title": "Tagged plot", "Author": "QA", "Description": "three points",
                                            "Software": None, "Experiment": "run-42"})
with Image.open("tagged.png") as im:
    info = {k: v for k, v in im.info.items() if isinstance(v, str)}
    print(sorted(info.items()))
    print(im.mode, im.size, tuple(round(v) for v in im.info.get("dpi", (0, 0))))
