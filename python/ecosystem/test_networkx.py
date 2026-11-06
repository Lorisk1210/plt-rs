"""networkx' drawing (``draw_networkx`` and its parts) on pltrs as
matplotlib."""

import pytest

nx = pytest.importorskip("networkx")


@pytest.fixture
def graph():
    """A house graph at fixed positions."""
    G = nx.house_graph()
    pos = {0: (0, 0), 1: (1, 0), 2: (0, 1), 3: (1, 1), 4: (0.5, 2.0)}
    return G, pos


def test_draw_networkx(check, graph):
    import matplotlib.pyplot as plt
    G, pos = graph
    nx.draw_networkx(G, pos, node_color="tab:orange", edge_color="tab:gray")
    check(plt.gca())


def test_edge_labels(check, graph):
    import matplotlib.pyplot as plt
    G, pos = graph
    nx.draw_networkx_nodes(G, pos, node_size=300)
    nx.draw_networkx_edges(G, pos, width=2)
    nx.draw_networkx_edge_labels(G, pos, edge_labels={(0, 1): "a", (2, 3): "b"})
    check(plt.gca())


def test_display(check, graph):
    import matplotlib.pyplot as plt
    G, pos = graph
    nx.set_node_attributes(G, pos, "pos")
    nx.display(G, canvas=plt.figure().add_subplot())
    check(plt.gca())
