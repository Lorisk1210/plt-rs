####
Math
####

The role: :mathmpl:`\alpha^2 + \beta_i = \sum_{k=0}^{n} \frac{1}{k!}` in a
sentence, and with the default font size.

The directive:

.. mathmpl::

    \int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}

The directive with a font set and a size:

.. mathmpl::
    :fontset: stix
    :fontsize: 16

    \mathcal{F}(\omega) = \oint \sqrt[3]{x}\;\mathrm{d}x

Sans serif, small:

.. mathmpl::
    :fontset: dejavusans
    :fontsize: 8

    x_{i,j}^{2} \leq \Gamma(\nu)
