# Control systems: Bode magnitude/phase with EngFormatter frequencies, gain/phase margin markers, and a Nyquist plot.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import EngFormatter

w = np.logspace(-1, 3, 500)
s = 1j * w
H = 100 / ((s + 1) * (s / 10 + 1) * (s / 100 + 1))
mag, phase = 20 * np.log10(np.abs(H)), np.degrees(np.unwrap(np.angle(H)))
wc = w[np.argmin(np.abs(mag))]
fig = plt.figure(figsize=(12, 5))
a = fig.add_subplot(2, 2, 1); b = fig.add_subplot(2, 2, 3, sharex=a); c = fig.add_subplot(1, 2, 2)
a.semilogx(w, mag); a.axhline(0, color="k", lw=0.5); a.axvline(wc, color="r", ls=":")
b.semilogx(w, phase); b.axhline(-180, color="k", lw=0.5)
b.xaxis.set_major_formatter(EngFormatter("rad/s"))
c.plot(H.real, H.imag, label="H(jw)")
c.plot(H.real, -H.imag, "--", color="C0", alpha=0.5)
c.plot(-1, 0, "r+", ms=12)
c.set_aspect("equal"); c.set_xlim(-5, 20); c.legend()
fig.tight_layout()
print(round(float(wc), 4), round(float(phase[np.argmin(np.abs(mag))] + 180), 3))
fig.savefig("bode.png", dpi=50)
