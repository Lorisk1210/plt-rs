# Spectrum with detected peaks marked and labelled, baseline subtraction panel, and a peak table.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 100, 2000)
centers, heights = [15, 32, 47, 68, 81], [1.0, 0.6, 1.4, 0.8, 0.5]
signal = sum(h * np.exp(-((x - c) / 1.2) ** 2) for c, h in zip(centers, heights))
baseline = 0.2 + 0.004 * x
y = signal + baseline
peaks = [i for i in range(1, len(x) - 1) if signal[i] > signal[i - 1] and signal[i] > signal[i + 1] and signal[i] > 0.3]
fig, (a, b, c) = plt.subplots(3, 1, figsize=(8, 9), gridspec_kw={"height_ratios": [2, 2, 1]})
a.plot(x, y, "k", lw=0.8)
a.plot(x, baseline, "r--", lw=0.8, label="baseline")
a.legend()
b.plot(x, y - baseline, lw=0.8)
b.plot(x[peaks], (y - baseline)[peaks], "rv")
for i in peaks:
    b.annotate(f"{x[i]:.1f}", (x[i], (y - baseline)[i]), xytext=(0, 6), textcoords="offset points", ha="center", fontsize=8)
c.axis("off")
c.table(cellText=[[f"{x[i]:.2f}", f"{(y - baseline)[i]:.3f}"] for i in peaks], colLabels=["position", "height"], loc="center")
fig.tight_layout()
print(len(peaks), [round(float(x[i]), 2) for i in peaks])
fig.savefig("peaks.png", dpi=50)
