"""Gaps in plt-rs's coverage of matplotlib's API, closed: cells added to a
table one by one (``Table.add_cell``) and a 3-D z axis of dates
(``Axes3D.zaxis_date``)."""

import numpy as np

from spec_support import case


@case("api_gaps_a/table_add_cell", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    table = ax.table(cellText=[["1", "2"], ["3", "4"]], colLabels=["A", "B"], loc="center",
                     colWidths=[0.2, 0.2])
    table.add_cell(3, 0, 0.2, 0.1, text="sum", loc="center", facecolor="#ddeeff")
    table.add_cell(3, 1, 0.2, 0.1, text="10")
    table.add_cell(1, -1, 0.15, 0.05, text="row", loc="left")
    table.add_cell(0, 1, 0.3, 0.05, text="wide B", facecolor="gold")


@case("api_gaps_a/axes3d_zaxis_date")
def _(fig):
    ax = fig.add_subplot(projection="3d")
    t = np.linspace(0, 4 * np.pi, 60)
    ax.plot(np.cos(t), np.sin(t), 19000 + t)
    ax.zaxis_date()


@case("api_gaps_a/axes3d_zaxis_date_concise", rc={"date.converter": "concise"})
def _(fig):
    ax = fig.add_subplot(projection="3d")
    ax.set_zlim(19000, 19000.5)
    ax.zaxis_date()
