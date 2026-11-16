# Spectrogram of a chirp signal with Axes.specgram and a colour bar in dB.
import numpy as np
import matplotlib.pyplot as plt

fs = 2000
t = np.arange(0, 2, 1 / fs)
signal = np.sin(2 * np.pi * (50 + 200 * t) * t) + 0.1 * np.sin(2 * np.pi * 600 * t)
fig, (a, b) = plt.subplots(2, 1, figsize=(7, 5), sharex=True)
a.plot(t, signal, lw=0.3)
a.set_ylabel("amplitude")
Pxx, freqs, bins, im = b.specgram(signal, NFFT=256, Fs=fs, noverlap=128, cmap="magma")
b.set_ylabel("frequency [Hz]")
b.set_xlabel("time [s]")
fig.colorbar(im, ax=[a, b], label="dB")
print(Pxx.shape, freqs[:3])
fig.savefig("chirp.png")
