"""Time series with dates: a year of daily visits to a website, a weekly
rhythm and a seasonal trend, with its 7-day mean and a campaign marked."""

from datetime import date, timedelta

import numpy as np

import pltrs.dates as mdates
import pltrs.pyplot as plt
from samples import Samples

days = [date(2024, 1, 1) + timedelta(days=i) for i in range(366)]
t = np.arange(366)
weekly = np.where(t % 7 >= 5, -900, 250)  # 2024 began on a Monday: weekends are quiet
seasonal = 4200 + 900 * np.cos(2 * np.pi * (t - 20) / 366) + 2.5 * t
campaign = np.where((t >= 244) & (t < 274), 1400 * np.sin(np.pi * (t - 244) / 30), 0)
visits = seasonal + weekly + campaign + Samples(5).normal(366, 0, 260)
mean = np.convolve(visits, np.ones(7), mode="valid") / 7

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
ax.plot(days, visits, color="0.75", linewidth=0.8, label="daily")
ax.plot(days[3:-3], mean, linewidth=2, label="7-day mean")
ax.axvspan(date(2024, 9, 1), date(2024, 10, 1), color="C1", alpha=0.12, label="campaign")
locator = mdates.AutoDateLocator()
ax.xaxis.set_major_locator(locator)
ax.xaxis.set_major_formatter(mdates.ConciseDateFormatter(locator))
ax.set_title("Daily visits to the website")
ax.set_ylabel("visits")
ax.legend(loc="upper center", ncols=3)

fig.savefig("time_series.png")
