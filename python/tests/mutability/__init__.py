"""
What programs change in place, and whether the artist follows.

matplotlib's artists are Python objects whose state is plain attributes,
and many of their getters hand out that state itself: the list of paths
a collection draws, the array a scatter places its markers by, the points
of a view box, the matrix of a transform. Programs change those objects
in place and expect the next draw to show it (the half-violin idiom,
``body.get_paths()[0].vertices[:, 0] = ...``). The same holds for objects
a program gives an artist and matplotlib keeps without copying.

This package measures that surface. A *row* names one object reached from
one artist (`surface`: every public getter and attribute of every kind of
artist; `handed`: the idioms and the helper objects; `given`: objects
given to artists), changes it in place and records what follows
(`measure.outcome`): whether a second read is the same object, whether it
shows the change, whether the next draw, the data limits, a measure and a
pickle do. ``python -m mutability --record`` records matplotlib's
outcomes in ``reference.json``; ``python/tests/test_mut1.py`` holds
pltrs to them.
"""
