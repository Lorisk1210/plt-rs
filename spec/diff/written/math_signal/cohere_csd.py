# Coherence and cross spectral density of two related signals with Axes.cohere and Axes.csd.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
dt = 0.01
t = np.arange(0, 30, dt)
nse1, nse2 = rng.normal(size=(2, t.size))
s1 = np.sin(2 * np.pi * 10 * t) + nse1
s2 = np.sin(2 * np.pi * 10 * t) + nse2
fig, (a, b) = plt.subplots(2, 1, figsize=(7, 5))
cxy, f = a.cohere(s1, s2, 256, 1.0 / dt)
a.set_ylabel("coherence")
pxy, f2 = b.csd(s1, s2, NFFT=128, Fs=1.0 / dt)
b.set_ylabel("CSD (dB)")
fig.tight_layout()
print(round(float(f[np.argmax(cxy)]), 3))
fig.savefig("coherence.png")
