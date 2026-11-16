# Web analytics: sessions with annotations of releases, funnel bars, device pie, and bounce rate scatter.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
idx = pd.date_range("2024-02-01", periods=90, freq="D")
sessions = pd.Series(1000 + 300 * np.sin(np.arange(90) / 7 * 2 * np.pi) + np.arange(90) * 5 + rng.normal(0, 60, 90), index=idx)
releases = {"v1.4": "2024-02-20", "v1.5": "2024-03-25"}
fig, axd = plt.subplot_mosaic("AAA;BCD", figsize=(12, 7), layout="constrained")
axd["A"].plot(sessions, color="C0")
for name, day in releases.items():
    y = sessions[day]
    axd["A"].annotate(name, (pd.Timestamp(day), y), xytext=(0, 30), textcoords="offset points",
                      arrowprops=dict(arrowstyle="->"), ha="center")
axd["A"].set_title("sessions")
stages = ["visit", "product", "cart", "checkout", "paid"]
counts = [10000, 5200, 2100, 1400, 900]
bars = axd["B"].bar(stages, counts, color=plt.cm.Blues(np.linspace(0.9, 0.4, 5)))
axd["B"].bar_label(bars, labels=[f"{c / counts[0]:.0%}" for c in counts])
axd["B"].tick_params(axis="x", rotation=30)
devices = {"mobile": 58, "desktop": 35, "tablet": 7}
axd["C"].pie(devices.values(), labels=devices.keys(), autopct="%d%%", startangle=90)
pages = rng.uniform(5, 300, 40)
bounce = 70 - 15 * np.log10(pages) + rng.normal(0, 5, 40)
sc = axd["D"].scatter(pages, bounce, c=bounce, cmap="RdYlGn_r", s=25)
axd["D"].set_xscale("log")
axd["D"].set_xlabel("pages per session x100")
fig.colorbar(sc, ax=axd["D"], label="bounce %")
print([round(b.get_height()) for b in bars], [t.get_text() for t in axd["B"].texts])
fig.savefig("web.png", dpi=60)
