# Save a sales line chart as a 300 dpi PNG for a slide, cropped tight with a small pad.
import numpy as np
import matplotlib.pyplot as plt

months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
sales = [12.1, 13.4, 15.0, 14.2, 16.8, 18.9, 21.3, 20.7, 18.2, 16.0, 14.9, 19.5]

plt.figure(figsize=(6, 3.5))
plt.plot(months, sales, marker="o", color="tab:blue", linewidth=2)
plt.title("Monthly sales 2024 (k units)")
plt.ylabel("k units")
plt.grid(alpha=0.3)
plt.ylim(0, 25)
for m, s in zip(months, sales):
    if s == max(sales):
        plt.annotate(f"peak {s}", (m, s), textcoords="offset points", xytext=(0, 8), ha="center")

plt.savefig("sales_300dpi.png", dpi=300, bbox_inches="tight", pad_inches=0.05)
plt.savefig("sales_screen.png", dpi=96)
print("saved")
