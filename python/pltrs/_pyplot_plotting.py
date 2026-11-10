"""
pyplot's wrappers of the plotting methods of `~.axes.Axes`
(``plt.hist``, ``plt.errorbar``, ...), which ``pyplot`` exports.

matplotlib generates these wrappers (``boilerplate.py``); each calls the
method of the same name on the current Axes (`.pyplot.gca`). Here they
are made from the methods themselves, so that each keeps its method's
signature (without ``self``) and docstring.
"""

import inspect
import linecache
import os

from . import _api, _docstring
from ._pyplot_figure import _add_pyplot_note
from .axes import Axes

#: The plotting methods pyplot wraps, by name.
_METHODS = (
    "axline", "barh", "broken_barh", "errorbar", "eventplot", "fill",
    "fill_between", "fill_betweenx", "hist", "hlines", "stackplot", "stairs",
    "stem", "step", "vlines", "boxplot", "violinplot", "pie",
    "contour", "contourf", "clabel", "tricontour", "tricontourf", "triplot",
    "pcolor", "pcolormesh", "quiver", "quiverkey", "barbs", "streamplot",
    "bar_label", "grouped_bar", "pie_label", "hexbin", "hist2d",
    "spy", "psd", "csd", "cohere", "magnitude_spectrum", "angle_spectrum",
    "phase_spectrum", "specgram", "acorr", "xcorr", "ecdf", "tripcolor",
    "table",
)


#: The colour-mapped artist of what a plotting method returns, by method
#: name: pyplot makes it the current image (matplotlib's ``sci`` calls in
#: ``boilerplate.py``).
_MAPPABLES = {
    "pcolor": lambda ret: ret,
    "pcolormesh": lambda ret: ret,
    "hexbin": lambda ret: ret,
    "tripcolor": lambda ret: ret,
    "quiver": lambda ret: ret,
    "hist2d": lambda ret: ret[-1],
    "specgram": lambda ret: ret[-1],
    "streamplot": lambda ret: ret.lines,
    "contour": lambda ret: ret if ret._A is not None else None,
    "contourf": lambda ret: ret if ret._A is not None else None,
    "tricontour": lambda ret: ret if ret._A is not None else None,
    "tricontourf": lambda ret: ret if ret._A is not None else None,
}


#: The module the pyplot functions are pyplot's.
_PYPLOT = __name__.rpartition(".")[0] + ".pyplot"


def _with_signature(name, signature, call):
    """
    A function *name* of pyplot's taking the parameters of *signature* and
    handing them to *call* (``call(args, kwargs)``), as the functions
    matplotlib's ``boilerplate.py`` writes do: arguments that do not bind
    to them fail with Python's errors naming *name*, and each parameter
    goes on by keyword (by position the positional-only ones and those
    before ``*args``), *data* only when given.
    """
    Parameter = inspect.Parameter
    namespace = {"_call": call, "__name__": _PYPLOT}
    by_position = any(parameter.kind is Parameter.VAR_POSITIONAL
                      for parameter in signature.parameters.values())
    parameters, positional, keywords = [], [], []
    previous = None
    for index, parameter in enumerate(signature.parameters.values()):
        kind, text = parameter.kind, parameter.name
        if previous is Parameter.POSITIONAL_ONLY and kind is not previous:
            parameters.append("/")
        if kind is Parameter.KEYWORD_ONLY and previous not in (
                Parameter.KEYWORD_ONLY, Parameter.VAR_POSITIONAL):
            parameters.append("*")
        previous = kind
        if parameter.default is not Parameter.empty:
            namespace[f"_default{index}"] = parameter.default
            text += f"=_default{index}"
        if kind is Parameter.VAR_POSITIONAL:
            parameters.append(f"*{text}")
            positional.append(f"*{text}")
            continue
        if kind is Parameter.VAR_KEYWORD:
            parameters.append(f"**{text}")
            keywords.append(f"**{text}")
            continue
        parameters.append(text)
        if kind is Parameter.POSITIONAL_ONLY or (
                kind is Parameter.POSITIONAL_OR_KEYWORD and by_position):
            positional.append(parameter.name)
        elif parameter.name == "data":
            keywords.append("**({'data': data} if data is not None else {})")
        else:
            keywords.append(f"{parameter.name!r}: {parameter.name}")
    if previous is Parameter.POSITIONAL_ONLY:
        parameters.append("/")
    source = (f"def {name}({', '.join(parameters)}):\n"
              f"    return _call(({''.join(f'{p}, ' for p in positional)}), "
              f"{{{', '.join(keywords)}}})\n")
    # Named as a file of the package, so that `.warn_external` passes over
    # its frame, with its source where tracebacks find it.
    filename = os.path.join(os.path.dirname(__file__), f"<pyplot.{name}>")
    linecache.cache[filename] = (len(source), None, source.splitlines(True), filename)
    exec(_docstring.compiled(source, filename), namespace)  # noqa: S102
    return namespace[name]


def _wrapper(name):
    """
    The pyplot function calling ``gca().<name>``, named and documented as
    matplotlib's ``boilerplate.py`` writes it: the method's docstring with
    the pyplot note, its signature without ``self``, and the parameter
    renamings and keyword-only changes ``_api`` decorated the method with
    (`._copy_docstring_and_deprecators`), around the signature beneath
    them.
    """
    method = getattr(Axes, name)
    mappable = _MAPPABLES.get(name)

    def call(args, kwargs):
        from . import pyplot
        ax = pyplot.gca()
        ret = getattr(ax, name)(*args, **kwargs)
        if mappable is not None and mappable(ret) is not None:
            ax._sci(mappable(ret))
        return ret

    # As ``boilerplate.py`` writes the function: with the signature beneath
    # a keyword-only change, which the decorators then make again.
    decorators, signed, layer = [], method, method
    while hasattr(layer, "__wrapped__"):
        decorator = _api.deprecation.DECORATORS.get(layer)
        if decorator is not None:
            decorators.append(decorator)
            if decorator.func is _api.make_keyword_only and signed is method:
                signed = layer.__wrapped__
        layer = layer.__wrapped__
    signature = _without_self(inspect.signature(method))
    wrapper = _with_signature(
        name, signature if signed is method else _without_self(inspect.signature(signed)),
        call)
    for decorator in reversed(decorators):
        wrapper = decorator(wrapper)
    wrapper.__signature__ = signature
    wrapper.__name__ = wrapper.__qualname__ = name
    wrapper.__module__ = _PYPLOT
    wrapper.__doc__ = method.__doc__
    _add_pyplot_note(wrapper, inspect.unwrap(method))
    return wrapper


def _without_self(signature):
    """*signature*, a method's, without its first parameter."""
    return signature.replace(parameters=list(signature.parameters.values())[1:])


def polar(*args, **kwargs):
    """
    Make a polar plot.

    call signature::

      polar(theta, r, [fmt], **kwargs)

    This is a convenience wrapper around `.pyplot.plot`. It ensures that the
    current Axes is polar (or creates one if needed) and then passes all parameters
    to ``.pyplot.plot``.

    .. note::
        When making polar plots using the :ref:`pyplot API <pyplot_interface>`,
        ``polar()`` should typically be the first command because that makes sure
        a polar Axes is created. Using other commands such as ``plt.title()``
        before this can lead to the implicit creation of a rectangular Axes, in which
        case a subsequent ``polar()`` call will fail.
    """
    from . import _api, pyplot
    from .projections.polar import PolarAxes
    fig = pyplot.gcf()
    if fig.get_axes():
        ax = pyplot.gca()
        if not isinstance(ax, PolarAxes):
            _api.warn_deprecated(
                "3.10",
                message="There exists a non-polar current Axes. Therefore, the "
                        "resulting plot from 'polar()' is non-polar. You likely "
                        "should call 'polar()' before any other pyplot plotting "
                        "commands. "
                        "Support for this scenario is deprecated in %(since)s and "
                        "will raise an error in %(removal)s"
            )
    else:
        ax = fig.add_subplot(projection="polar")
    return ax.plot(*args, **kwargs)


__all__ = [*_METHODS, "polar"]
globals().update({name: _wrapper(name) for name in _METHODS})
polar.__module__ = _PYPLOT
