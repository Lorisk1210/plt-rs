# Two shifted images, their difference with a diverging colormap, the cross-correlation peak and the aligned overlay.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:64, 0:64]
a = np.exp(-((x - 28) ** 2 + (y - 30) ** 2) / 40) + 0.5 * np.exp(-((x - 40) ** 2 + (y - 20) ** 2) / 20)
b = np.roll(np.roll(a, 5, axis=1), -3, axis=0)
xc = np.real(np.fft.ifft2(np.fft.fft2(a) * np.conj(np.fft.fft2(b))))
dy, dx = np.unravel_index(xc.argmax(), xc.shape)
dy, dx = (dy + 32) % 64 - 32, (dx + 32) % 64 - 32
aligned = np.roll(np.roll(b, dx, axis=1), dy, axis=0)
fig, axs = plt.subplots(1, 4, figsize=(14, 3.5))
axs[0].imshow(np.dstack([a, b, np.zeros_like(a)]) / max(a.max(), b.max()))
d = axs[1].imshow(b - a, cmap="RdBu_r", vmin=-1, vmax=1)
fig.colorbar(d, ax=axs[1], shrink=0.7)
axs[2].imshow(np.fft.fftshift(xc), cmap="viridis")
axs[2].plot(32 + dx, 32 + dy, "r+", ms=12)
axs[3].imshow(np.dstack([a, aligned, np.zeros_like(a)]) / a.max())
for ax, t in zip(axs, ["overlay", "difference", "xcorr", "aligned"]):
    ax.set_title(t)
    ax.axis("off")
print(int(dx), int(dy), round(float(np.abs(aligned - a).max()), 6))
fig.savefig("registration.png", dpi=50)
