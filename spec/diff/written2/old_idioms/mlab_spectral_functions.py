# matplotlib.mlab's spectral helpers on deterministic signals: psd, csd, cohere, specgram, magnitude/phase/angle spectra, detrends.
import numpy as np
from matplotlib import mlab

fs = 100.0
t = np.arange(0, 4, 1 / fs)
x = np.sin(2 * np.pi * 10 * t) + 0.5 * np.sin(2 * np.pi * 23 * t)
y = np.sin(2 * np.pi * 10 * t + 0.5) + 0.1 * t
pxx, f = mlab.psd(x, NFFT=128, Fs=fs, noverlap=64)
print(len(f), round(float(f[np.argmax(pxx)]), 3), round(float(pxx.max()), 5))
pxy, f2 = mlab.csd(x, y, NFFT=128, Fs=fs)
print(np.round(np.abs(pxy[[12, 13, 30]]), 5).tolist())
coh, f3 = mlab.cohere(x, y, NFFT=64, Fs=fs, noverlap=32)
print(np.round(coh[:4], 4).tolist())
spec, freqs, times = mlab.specgram(x, NFFT=64, Fs=fs, noverlap=32)
print(spec.shape, np.round(times[:3], 3).tolist(), round(float(freqs[-1]), 2))
for name in ("magnitude_spectrum", "angle_spectrum", "phase_spectrum", "complex_spectrum"):
    s, fr = getattr(mlab, name)(x[:128], Fs=fs)
    print(name, len(s), np.round(np.abs(s[[0, 13]]), 5).tolist())
print(np.round(mlab.detrend_mean(np.array([1.0, 2, 3, 6])), 4).tolist(), np.round(mlab.detrend_linear(np.array([1.0, 2, 4, 4])), 4).tolist())
print(np.round(mlab.detrend(np.array([1.0, 2, 3, 6]), key="mean"), 4).tolist(), mlab.detrend_none([1, 2]))
print(np.round(mlab.window_hanning(np.ones(5)), 4).tolist(), mlab.window_none(np.ones(3)).tolist())
pxx2, f4 = mlab.psd(x, NFFT=100, Fs=fs, window=mlab.window_none, sides="twosided", scale_by_freq=False)
print(len(f4), round(float(f4[0]), 3), round(float(pxx2.sum()), 4))
kde = mlab.GaussianKDE(np.array([0.0, 1, 1, 2, 5]), bw_method="silverman")
print(round(float(kde.covariance_factor()), 5), np.round(kde.evaluate(np.array([0.0, 1, 3])), 5).tolist())
pxx3, f5 = mlab.psd(x, NFFT=128, Fs=fs, pad_to=256, detrend="linear")
print(len(f5), round(float(f5[np.argmax(pxx3)]), 3))
spec2, freqs2, t2 = mlab.specgram(x, NFFT=32, Fs=fs, noverlap=0, mode="magnitude")
print(spec2.shape, round(float(spec2.max()), 4))
