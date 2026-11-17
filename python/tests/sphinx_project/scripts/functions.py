"""Plots made by functions the plot directive calls by name."""

import numpy as np
from matplotlib import pyplot as plt


def rings():
    """Concentric rings."""
    fig, ax = plt.subplots()
    for radius in (1, 2, 3):
        ax.add_patch(plt.Circle((0, 0), radius, fill=False, lw=radius))
    ax.set_xlim(-4, 4)
    ax.set_ylim(-4, 4)
    ax.set_aspect('equal')


def petals():
    """A rose curve."""
    theta = np.linspace(0, 2 * np.pi, 400)
    plt.figure()
    plt.plot(np.cos(4 * theta) * np.cos(theta), np.cos(4 * theta) * np.sin(theta))
    plt.axis('equal')
