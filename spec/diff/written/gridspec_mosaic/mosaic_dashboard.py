# KPI dashboard with subplot_mosaic: a wide trend panel, a pie and a bar panel from an ASCII layout.
import numpy as np
import matplotlib.pyplot as plt

layout = """
    AAB
    AAB
    CCD
"""
fig, axd = plt.subplot_mosaic(layout, figsize=(9, 6), layout="constrained")
weeks = np.arange(1, 21)
axd["A"].plot(weeks, 100 + 3 * weeks + 5 * np.sin(weeks), marker=".")
axd["A"].set_title("Revenue trend")
axd["B"].pie([55, 30, 15], labels=["web", "store", "phone"], autopct="%d%%")
axd["C"].bar(["Q1", "Q2", "Q3", "Q4"], [80, 95, 102, 120], color="C2")
axd["D"].text(0.5, 0.5, "NPS\n42", ha="center", va="center", fontsize=22)
axd["D"].axis("off")
print(sorted(axd))
fig.savefig("kpi.png")
