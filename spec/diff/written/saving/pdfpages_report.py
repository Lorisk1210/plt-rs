# Multi-page PDF report: one page per product with its weekly demand, plus document info.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages

rng = np.random.default_rng(7)
products = ["alpha", "beta", "gamma", "delta"]
weeks = np.arange(1, 27)

with PdfPages("demand_report.pdf") as pdf:
    for name in products:
        demand = 100 + np.cumsum(rng.normal(0, 5, weeks.size))
        fig, ax = plt.subplots(figsize=(8.27, 4))
        ax.plot(weeks, demand, color="tab:green")
        ax.fill_between(weeks, demand.min(), demand, alpha=0.2, color="tab:green")
        ax.set_title(f"Weekly demand: {name}")
        ax.set_xlabel("week")
        pdf.savefig(fig)
        plt.close(fig)
    d = pdf.infodict()
    d["Title"] = "Demand report"
    d["Author"] = "Ops"
    print("pages:", pdf.get_pagecount())

print("done")
