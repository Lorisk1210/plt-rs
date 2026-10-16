"""Smoke-test figures exercising the most common pyplot calls."""

from spec_support import case


@case("basic/empty_axes")
def _(fig):
    fig.subplots()


@case("basic/line")
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3, 4], [1, 4, 2, 3])
