"""
Which nodes of the transform trees of a pickled figure had their values.

matplotlib pickles its trees with the values their nodes computed and
whether they are valid, so that an unpickled figure computes anew, at its
first draw, only what the pickled one had not, and numpy warns of that
only where a leaf is no ordinary number (see `._transform_replay`); but
for what is built on the figure's resolution, which matplotlib's
``Figure.__setstate__`` sets anew, invalidating it.

The package does not pickle its trees: an unpickled object builds them
again over pltrs (Rust)'s leaves (`._transform_trees`). Its pickled state
says which nodes were valid instead (`of`), and the trees built again
compute those, quietly (`restore`).
"""

import numpy as np

from .transforms import BboxBase, Transform, TransformNode

#: The attributes of a node holding the nodes it is made of.
_NODE_PARTS = ("_a", "_b", "_x", "_y", "_child", "_bbox", "_transform",
               "_boxin", "_boxout", "_scale_trans")


def _parts(node):
    """The nodes *node* is made of."""
    parts = (node.__dict__.get(name) for name in _NODE_PARTS)
    return [part for part in parts if isinstance(part, TransformNode)]


def _nodes(d, tree_keys):
    """The nodes of the trees among the attributes *d* of their owner
    (those named by *tree_keys* and by its projection), with the nodes
    they are made of, each once, in an order that follows from how the
    trees are made only."""
    names = sorted((tree_keys & d.keys()) | set(d.get("_transform_trees", ())))
    stack = [d[name] for name in reversed(names)
             if isinstance(d.get(name), TransformNode)]
    nodes, seen = [], set()
    while stack:
        node = stack.pop()
        if id(node) not in seen:
            seen.add(id(node))
            nodes.append(node)
            stack.extend(reversed(_parts(node)))
    return nodes


def of(d, tree_keys):
    """Whether each node of the trees among the attributes *d* of their
    owner has its value, in the order of `_nodes`."""
    return [node._invalid == TransformNode._VALID for node in _nodes(d, tree_keys)]


def _rests_on(node, leaf, known):
    """Whether *node* is built on *leaf* (*known*: the answers so far, by
    node)."""
    key = id(node)
    if key not in known:
        known[key] = False
        known[key] = node is leaf or any(
            _rests_on(part, leaf, known) for part in _parts(node))
    return known[key]


def restore(d, tree_keys, valid, dpi_scale_trans):
    """Compute the nodes of the trees among the attributes *d* of an
    unpickled object, just built, that *valid* says had their values when
    it was pickled (`of`), quietly; not those built on the resolution,
    *dpi_scale_trans* of the object's figure."""
    nodes = _nodes(d, tree_keys)
    if len(nodes) != len(valid):
        return
    on_resolution = {}
    with np.errstate(all="ignore"):
        for node, was_valid in zip(nodes, valid):
            if not was_valid or _rests_on(node, dpi_scale_trans, on_resolution):
                continue
            if isinstance(node, BboxBase):
                node.get_points()
            elif isinstance(node, Transform):
                affine = node.get_affine()
                if node.is_affine:
                    affine.get_matrix()
