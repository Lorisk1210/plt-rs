# Vibration analysis: raw accelerometer signal, its FFT amplitude with peak labels, and a spectrogram with a run-up.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(180)
fs = 2000
t = np.arange(0, 4, 1 / fs)
f_run = 20 + 15 * t
sig = np.sin(2 * np.pi * np.cumsum(f_run) / fs) + 0.5 * np.sin(2 * np.pi * 120 * t) + 0.2 * rng.normal(size=t.size)
spec = np.abs(np.fft.rfft(sig)) / t.size * 2
freqs = np.fft.rfftfreq(t.size, 1 / fs)
fig, axs = plt.subplots(3, 1, figsize=(9, 9))
axs[0].plot(t, sig, lw=0.3)
axs[1].plot(freqs, spec)
for f0 in (120,):
    k = np.argmin(np.abs(freqs - f0))
    axs[1].annotate(f"{freqs[k]:.0f} Hz", (freqs[k], spec[k]), xytext=(10, 10), textcoords="offset points")
axs[1].set_xlim(0, 300)
P, F, T, im = axs[2].specgram(sig, NFFT=256, Fs=fs, noverlap=192, cmap="viridis")
axs[2].set_ylim(0, 300)
fig.tight_layout()
print(round(float(freqs[spec[100:].argmax() + 100]), 2), P.shape)
fig.savefig("vibration.png", dpi=50)
