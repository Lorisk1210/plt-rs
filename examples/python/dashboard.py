"""Subplot mosaic dashboard: four views of an online shop's year laid out
by name, the revenue trend across the top."""

import numpy as np

import pltrs.pyplot as plt

months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
revenue = np.array([82, 79, 91, 95, 102, 99, 94, 97, 108, 117, 139, 162])  # k€
target = np.linspace(80, 140, 12)
weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
orders = [610, 580, 640, 700, 820, 960, 720]
products = ["headphones", "keyboards", "monitors", "webcams", "chairs", "lamps"]
sold = [1840, 1420, 960, 870, 610, 430]
channels = ["search", "social", "e-mail", "direct"]
share = [42, 23, 15, 20]

fig, axd = plt.subplot_mosaic(
    "RRP;WCP", figsize=(9, 5.5), layout="constrained", width_ratios=[1.2, 1, 1.1]
)
fig.suptitle("Online shop: 2024 at a glance")

ax = axd["R"]
ax.plot(months, revenue, marker="o", linewidth=2, label="revenue")
ax.plot(months, target, linestyle="--", color="0.5", linewidth=1, label="target")
ax.fill_between(months, revenue, target, where=revenue >= target, alpha=0.2, interpolate=True)
ax.set_title("Monthly revenue (k€)")
ax.legend(loc="upper left")

ax = axd["P"]
ax.barh(products[::-1], sold[::-1], color="C2")
ax.set_title("Units sold by product")
ax.grid(axis="y", visible=False)

ax = axd["W"]
ax.bar(weekdays, orders, color="C1")
ax.set_title("Orders by weekday")

ax = axd["C"]
ax.pie(share, labels=channels, autopct="%1.0f%%", startangle=90, wedgeprops=dict(width=0.45))
ax.set_title("Traffic by channel")

fig.savefig("dashboard.png")
