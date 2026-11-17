#####
Plots
#####

Inline code, with the configuration's defaults:

.. plot::

    plt.plot(WAVE)

Inline code shown with ``:include-source:`` and a code caption, under a
prefix of its own:

.. plot::
    :include-source:
    :code-caption: The code of a bar chart
    :filename-prefix: bars

    fig, ax = plt.subplots()
    ax.bar(['a', 'b', 'c'], [3, 1, 2], color=['C0', 'C1', 'C2'])
    ax.set_title('Bars')

The source left out with ``:include-source: false``, a caption, alt text
and the image options:

.. plot::
    :include-source: false
    :caption: A scatter plot with a caption.
    :alt: Points along a parabola
    :scale: 50
    :align: center
    :class: wide-plot

    x = np.linspace(-1, 1, 21)
    plt.scatter(x, x ** 2, c=x, cmap='viridis')
    plt.colorbar()

Width and height, and no link to the source:

.. plot::
    :width: 300px
    :height: 200px
    :show-source-link: false

    plt.imshow(np.outer(np.arange(8), np.arange(8)), origin='lower')

A link to the source asked for:

.. plot::
    :show-source-link: true

    plt.hist(np.cos(np.arange(200) * 0.37), bins=12)

Doctest syntax, detected:

.. plot::

    A doctest:

    >>> fig, ax = plt.subplots()
    >>> ax.plot([1, 3, 2], marker='s')
    [<matplotlib.lines.Line2D object at 0x...>]

Doctest syntax, declared:

.. plot::
    :format: doctest

    >>> plt.plot(range(5), 'r--')

Python syntax, declared, though it looks like a doctest's output:

.. plot::
    :format: python

    lines = plt.plot([2, 1, 3])
    text = plt.text(0.5, 2.5, '>>> not a prompt')

Two figures, and a split at ``plt.show()``:

.. plot::

    plt.figure()
    plt.plot([0, 1], [0, 1])
    plt.show()
    plt.figure()
    plt.plot([0, 1], [1, 0])

A script given by its file name (relative to ``plot_basedir``), with its
caption in the content:

.. plot:: spiral.py

    The caption of the spiral, given in the content.

A function of a script:

.. plot:: functions.py rings

A function of the same script, its source included, with its caption
given by the option:

.. plot:: functions.py petals
    :include-source:
    :caption: The petals' caption, from the option.

A script reading a data file from the working directory:

.. plot:: from_data.py

Mathtext, a legend and an annotation, under the rcParams of
``plot_rcparams``:

.. plot::

    fig, ax = plt.subplots()
    ax.plot(WAVE, label=r'$\sin(\theta)$')
    ax.annotate('peak', xy=(12, 1), xytext=(25, 0.5),
                arrowprops={'arrowstyle': '->'})
    ax.legend()
