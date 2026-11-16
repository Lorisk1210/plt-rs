# Power spectral density (psd) and magnitude spectrum of two sines plus noise.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
fs = 1000
t = np.arange(0, 1, 1 / fs)
x = np.sin(2 * np.pi * 50 * t) + 0.5 * np.sin(2 * np.pi * 120 * t) + 0.3 * rng.normal(size=t.size)
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
Pxx, f = axs[0].psd(x, NFFT=256, Fs=fs)
axs[0].set_title("psd")
spec, freqs, line = axs[1].magnitude_spectrum(x, Fs=fs, scale="dB")
axs[1].set_title("magnitude (dB)")
axs[2].phase_spectrum(x, Fs=fs)
axs[2].set_title("phase")
fig.tight_layout()
print(int(f[np.argmax(Pxx)]), int(freqs[np.argmax(spec)]))
fig.savefig("spectra.png")
