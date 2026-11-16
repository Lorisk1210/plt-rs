# Grid of stations' anomalies as a map (pcolormesh with TwoSlopeNorm) and the mean anomaly series with coloured bars.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import TwoSlopeNorm

rng = np.random.default_rng(156)
lon = np.linspace(-10, 30, 21); lat = np.linspace(35, 70, 15)
Lon, Lat = np.meshgrid(lon, lat)
anom = 1.5 * np.exp(-((Lon - 15) ** 2 + (Lat - 55) ** 2) / 150) - 0.5 + rng.normal(0, 0.2, Lon.shape)
years = np.arange(1990, 2024)
series = 0.03 * (years - 1990) - 0.4 + rng.normal(0, 0.15, years.size)
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 4.5), gridspec_kw={"width_ratios": [1.2, 1]})
norm = TwoSlopeNorm(vmin=anom.min(), vcenter=0, vmax=anom.max())
m = a.pcolormesh(Lon, Lat, anom, cmap="RdBu_r", norm=norm, shading="nearest")
a.set_aspect(1 / np.cos(np.deg2rad(52)))
fig.colorbar(m, ax=a, label="°C")
b.bar(years, series, color=plt.cm.RdBu_r(TwoSlopeNorm(vmin=series.min(), vcenter=0, vmax=series.max())(series)))
b.axhline(0, color="k", lw=0.5)
print(round(float(anom.mean()), 4), round(float(series[-5:].mean()), 4))
fig.savefig("anomaly_map.png", dpi=50)
