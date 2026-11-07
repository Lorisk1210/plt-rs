"""Pick events of lines, markers, patches, texts, collections and legends."""

from . import at, one_axes, scenario


@scenario("pick")
def pick(plt, log):
    fig, ax = one_axes(plt)
    line, = ax.plot([1, 4], [8, 8], picker=True, pickradius=5, label="line")
    marker, = ax.plot([6], [8], "o", picker=3, label="marker")
    patch = ax.add_patch(plt.Rectangle((1, 1), 2, 2, picker=True,
                                       label="patch"))
    ax.text(6, 2, "text", picker=True, label="text")
    scatter = ax.scatter([7, 8], [5, 5], s=100, picker=True, label="scatter")
    legend = ax.legend(handles=[line, marker, patch, scatter],
                       loc="lower left", bbox_to_anchor=(0.05, 0.3))
    legend.set_picker(True)
    legend.set_label("legend")
    for handle in legend.get_lines():
        handle.set_picker(5)
        handle.set_label(f"legend handle {handle.get_label()}")
    for text in legend.get_texts():
        text.set_picker(True)
        text.set_label(f"legend text {text.get_text()}")
    return fig


@pick.steps
def _(d):
    x, y = at(2.5, 8)
    for dy in (0, 4, 8):
        d.move(x, y + dy)
        d.click(1)
    d.move(*at(2.5, 8))
    d.click(3)
    d.click(2)
    d.scroll(1)
    for point in [at(6, 8), at(2, 2), at(7.5, 5), at(8, 5)]:
        d.move(*point)
        d.click(1)
    # The text and the legend (its handle, its text, its box), where
    # matplotlib draws them.
    for point in [(245, 82), (90, 185), (125, 184), (165, 112)]:
        d.move(*point)
        d.click(1)
    d.close()
