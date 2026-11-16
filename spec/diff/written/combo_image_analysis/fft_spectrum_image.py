# 2-D FFT of a textured image: log power spectrum, radial average profile, and a low-pass filtered reconstruction.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

y, x = np.mgrid[0:128, 0:128]
img = np.sin(2 * np.pi * x / 8) + np.sin(2 * np.pi * (x + y) / 16) + 0.3 * np.cos(2 * np.pi * y / 4)
F = np.fft.fftshift(np.fft.fft2(img))
P = np.abs(F) ** 2 + 1e-6
r = np.hypot(x - 64, y - 64).astype(int)
radial = np.bincount(r.ravel(), P.ravel()) / np.bincount(r.ravel())
lowpass = np.real(np.fft.ifft2(np.fft.ifftshift(F * (np.hypot(x - 64, y - 64) < 10))))
fig, axs = plt.subplots(1, 4, figsize=(14, 3.5))
axs[0].imshow(img, cmap="gray")
im = axs[1].imshow(P, norm=LogNorm(vmin=1, vmax=P.max()), cmap="magma")
fig.colorbar(im, ax=axs[1], shrink=0.7)
axs[2].semilogy(radial[:64])
axs[2].set_xlabel("radius")
axs[3].imshow(lowpass, cmap="gray")
print(int(np.argmax(radial[1:64])) + 1, round(float(lowpass.std()), 4))
fig.savefig("fft.png", dpi=50)
