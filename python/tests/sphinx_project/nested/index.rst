######
Nested
######

A plot in a page in a directory:

.. plot::

    plt.plot([3, 1, 2])

A script given relative to ``plot_basedir``, from the nested page:

.. plot:: functions.py rings
    :caption: Rings again, from the nested page.

figure-mpl from the nested page:

.. figure-mpl:: ../_generated/picture.png
    :srcset: ../_generated/picture.png, ../_generated/picture.2x.png 2.00x
