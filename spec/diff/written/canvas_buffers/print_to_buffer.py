# print_to_buffer gives raw RGBA bytes and the size; rebuild an image with Pillow.
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image

fig, ax = plt.subplots(figsize=(3, 2), dpi=80)
ax.bar([1, 2, 3], [2, 4, 3], color="royalblue")
buf, (w, h) = fig.canvas.print_to_buffer()
print(w, h, len(buf) == w * h * 4)
img = Image.frombuffer("RGBA", (w, h), buf, "raw", "RGBA", 0, 1)
img.save("rebuilt.png")
arr = np.frombuffer(buf, dtype=np.uint8).reshape(h, w, 4)
print(arr[h // 2, w // 2].tolist())
