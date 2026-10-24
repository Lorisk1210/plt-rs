"""matplotlib's non-image tests from ``tests/test_figure.py`` that draw:
``test_subfigure_pdf`` (a bar label in a subfigure of a constrained
figure), ``test_removed_axis`` (a shared axes removed before drawing) and
``test_subfigure_spanning`` (subfigures on blocks of a grid, an axes in
each, laid out). Their assertions are probes of the ``mpl_figure`` area;
the cases draw what the tests make.
"""

from figures._mpl_tests import group_case

GROUP = "mpl_tests_figure"
mpl_test = group_case(GROUP)


@mpl_test("subfigure_pdf")
def _(fig):
    fig.set_layout_engine("constrained")
    sub_fig = fig.subfigures()
    ax = sub_fig.add_subplot(111)
    b = ax.bar(1, 1)
    ax.bar_label(b)


@mpl_test("removed_axis")
def _(fig):
    axs = fig.subplots(2, sharex=True)
    axs[0].remove()


@mpl_test("subfigure_spanning")
def _(fig):
    fig.set_layout_engine("constrained")
    gs = fig.add_gridspec(3, 3)
    sub_figs = [fig.add_subfigure(gs[0, 0]), fig.add_subfigure(gs[0:2, 1]),
                fig.add_subfigure(gs[2, 1:3]), fig.add_subfigure(gs[0:, 1:])]
    for sub_fig in sub_figs:
        sub_fig.add_subplot()
