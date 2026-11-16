# Patient vitals chart: heart rate and blood pressure (error-bar style ranges) sharing time, SpO2 panel with threshold fill.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(152)
t = pd.date_range("2024-02-10 06:00", periods=48, freq="30min")
hr = 72 + 10 * np.sin(np.arange(48) / 6) + rng.normal(0, 3, 48)
sys_bp = 125 + rng.normal(0, 6, 48); dia_bp = 80 + rng.normal(0, 4, 48)
spo2 = np.clip(97 + rng.normal(0, 1.2, 48), 88, 100)
fig, (a, b, c) = plt.subplots(3, 1, figsize=(9, 8), sharex=True)
a.plot(t, hr, "r.-")
a.axhspan(60, 100, color="green", alpha=0.08)
a.set_ylabel("HR [bpm]")
b.vlines(t, dia_bp, sys_bp, color="C0")
b.plot(t, sys_bp, "v", color="C0", ms=4); b.plot(t, dia_bp, "^", color="C0", ms=4)
b.set_ylabel("BP [mmHg]")
c.plot(t, spo2, "C2")
c.fill_between(t, spo2, 94, where=spo2 < 94, color="red", alpha=0.4, interpolate=True)
c.set_ylim(85, 101)
c.set_ylabel("SpO2 [%]")
fig.autofmt_xdate()
print(round(float(hr.mean()), 3), int((spo2 < 94).sum()))
fig.savefig("vitals.png", dpi=50)
