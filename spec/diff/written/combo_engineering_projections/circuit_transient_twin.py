# RC/RLC transients: capacitor voltage and current on twin axes, time constant markers, and an envelope of the RLC ringing.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 5e-3, 1000)
R, C, L = 1e3, 1e-6, 50e-3
vc = 5 * (1 - np.exp(-t / (R * C)))
i = 5 / R * np.exp(-t / (R * C))
alpha, w0 = 100 / (2 * L), 1 / np.sqrt(L * C)
wd = np.sqrt(w0 ** 2 - alpha ** 2)
v_rlc = 5 * np.exp(-alpha * t) * np.cos(wd * t)
fig, (a, b) = plt.subplots(2, 1, figsize=(9, 7), sharex=True)
a.plot(t * 1e3, vc, "C0", label="Vc")
tw = a.twinx(); tw.plot(t * 1e3, i * 1e3, "C1", label="I")
for k in range(1, 4):
    a.axvline(k * R * C * 1e3, color="gray", ls=":")
    a.text(k * R * C * 1e3, 0.2, f"{k}τ", ha="center")
a.set_ylabel("V"); tw.set_ylabel("mA")
b.plot(t * 1e3, v_rlc, lw=0.8)
b.plot(t * 1e3, 5 * np.exp(-alpha * t), "r--"); b.plot(t * 1e3, -5 * np.exp(-alpha * t), "r--")
b.set_xlabel("time [ms]")
print(round(float(vc[-1]), 4), round(float(wd / (2 * np.pi)), 2))
fig.savefig("transients.png", dpi=50)
