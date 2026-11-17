#######
Context
#######

A plot outside the context: ``a`` is not seen by the context.

.. plot::

    a = 'outside'
    plt.plot([1, 2])

The context starts:

.. plot::
    :context:

    assert 'a' not in globals()
    a = 3
    fig, ax = plt.subplots()
    ax.plot(range(a))

``:nofigs:`` runs code and shows nothing:

.. plot::
    :context:
    :nofigs:

    a += 2
    ax.set_title('changed without a figure shown')

The context's figure, kept, drawn again:

.. plot::
    :context:

    ax.plot(range(a), 'o')

``close-figs`` keeps the variables and closes the figures:

.. plot::
    :context: close-figs

    plt.plot(range(a), range(a)[::-1])

``reset`` forgets them:

.. plot::
    :context: reset

    assert 'a' not in globals()
    plt.bar([0, 1], [2, 1])

An rcParam set in the context, which ``plot_apply_rcparams`` resets or
keeps for the next plot in it:

.. plot::
    :context:

    plt.rcParams['lines.linewidth'] = 6
    plt.figure()
    plt.plot([1, 3, 2])

.. plot::
    :context: close-figs

    plt.plot([2, 3, 1])
