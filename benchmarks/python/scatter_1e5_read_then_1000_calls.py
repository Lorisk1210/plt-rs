"""A scatter of 10⁵ points whose offsets the program read once and still
holds, then 1,000 setter calls on it. The array ``get_offsets()`` returns
is the collection's own in matplotlib; pltrs compares it with its own
offsets before each native call that reads the collection, and before
none that only sets a style: the 1,000 calls must cost what they cost a
scatter nobody read (``python/tests/test_mut1.py`` holds them to no
comparison at all), and the draw one pass over the array."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(27)
x = rng.uniform(100_000, 0.0, 1.0)
y = rng.uniform(100_000, 0.0, 1.0)


def draw():
    fig, ax = plt.subplots()
    points = ax.scatter(x, y)
    offsets = points.get_offsets()
    for k in range(1000):
        points.set_linewidth(0.5 + 0.001 * k)
    assert len(offsets) == 100_000
    return fig


if __name__ == "__main__":
    save(draw())
