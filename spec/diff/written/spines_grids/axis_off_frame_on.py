# Keep the frame but drop ticks (set_frame_on, axis('off'), set_axis_off/on) for image panels.
import numpy as np
import matplotlib.pyplot as plt

img = np.random.default_rng(3).random((10, 10))
fig, axs = plt.subplots(1, 4, figsize=(10, 2.8))
for ax in axs:
    ax.imshow(img, cmap="bone")
axs[0].set_title("default")
axs[1].set_xticks([]); axs[1].set_yticks([])
axs[1].set_title("frame only")
axs[2].set_axis_off()
axs[2].set_title("axis off")
axs[3].set_frame_on(False)
axs[3].set_title("no frame")
print([ax.axison for ax in axs], axs[3].get_frame_on())
fig.savefig("frames.png")
