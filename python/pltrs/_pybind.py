"""
How matplotlib's C++ extensions, made with pybind11, take their arguments,
for the classes and functions pltrs (Python) defines in their place
(`.ft2font`, the Agg renderer): the same calls refused, with pybind11's
messages.

pybind11 binds a call to the parameters of the C++ function, converting
each argument to its C++ type. A call it cannot bind fails with
``TypeError: f(): incompatible function arguments``, which lists the
function's signature as its docstring gives it, and the arguments of the
call (`incompatible`); `binds` refuses calls so. A value the extension
casts itself, where it does not convert, fails with ``RuntimeError: Unable
to cast Python instance ...`` (`cast_error`).

The conversions are pybind11's casters as matplotlib's extensions are
built (without ``PYBIND11_DETAILED_ERROR_MESSAGES``), each a predicate of
the value (`ACCEPTS`, by the type the signature names).
"""

import functools
import inspect

import numpy as np


def cast_error(value):
    """pybind11's error for *value*, from which a C++ value is cast."""
    return RuntimeError(
        f"Unable to cast Python instance of type {type(value)} to C++ type '?' "
        "(#define PYBIND11_DETAILED_ERROR_MESSAGES or compile in debug mode "
        "for details)")


def incompatible(name, signatures, args, kwargs):
    """pybind11's error for the call ``name(*args, **kwargs)`` of a function
    of *signatures* (the texts of its overloads, as the function's docstring
    gives them, without the name), to none of which the call binds."""
    invoked = ", ".join(map(repr, args))
    if kwargs:
        invoked += "; kwargs: " + ", ".join(
            f"{key}={value!r}" for key, value in kwargs.items())
    supported = "".join(f"\n    {number}. {signature}"
                        for number, signature in enumerate(signatures, 1))
    return TypeError(
        f"{name}(): incompatible function arguments. The following argument "
        f"types are supported:{supported}\n\nInvoked with: {invoked}")


def _has(value, method):
    """Whether *value*'s type defines the special method *method*."""
    return hasattr(type(value), method)


def double(value):
    """Whether pybind11 casts *value* to a C++ ``double``: a number, by
    ``__float__`` or ``__index__`` (no text)."""
    return not isinstance(value, (str, bytes)) and (
        _has(value, "__float__") or _has(value, "__index__"))


def integer(value):
    """Whether pybind11 casts *value* to a C++ integer: by ``__index__``,
    never a float."""
    return not isinstance(value, (float, np.floating)) and _has(value, "__index__")


def boolean(value):
    """Whether pybind11 casts *value* to a C++ ``bool``: None, or what has
    a truth value of its own as a number (not a container, not text)."""
    return value is None or _has(value, "__bool__")


def text(value):
    """Whether pybind11 casts *value* to a C++ string."""
    return isinstance(value, str)


def texts(value):
    """Whether pybind11 casts *value* to a sequence of strings."""
    return (not isinstance(value, (str, bytes)) and _has(value, "__len__")
            and _has(value, "__getitem__") and all(map(text, value)))


def language_ranges(value):
    """Whether pybind11 casts *value* to a sequence of ``(language, start,
    end)`` triples: a sequence (no text) of sequences of a string and two
    integers."""
    return (not isinstance(value, (str, bytes)) and _has(value, "__len__")
            and _has(value, "__getitem__")
            and all(not isinstance(item, (str, bytes)) and _has(item, "__len__")
                    and len(item) == 3 and text(item[0])
                    and integer(item[1]) and integer(item[2]) for item in value))


def optional_text(value):
    """Whether pybind11 casts *value* to a C string: a string, or None."""
    return value is None or text(value)


def buffer(value):
    """Whether *value* exports a buffer."""
    try:
        memoryview(value)
    except TypeError:
        return False
    return True


def _instance(name):
    """The predicate of a class of the extension named *name* (its last
    component, as the signature names it)."""
    def accepts(value):
        return any(cls.__name__ == name for cls in type(value).__mro__)
    return accepts


#: What pybind11 accepts for each type a signature names, by its name; an
#: unknown name accepts anything.
ACCEPTS = {
    "typing.SupportsFloat": double,
    "typing.SupportsInt": integer,
    "typing.SupportsIndex": integer,
    "bool": boolean,
    "str": text,
    "None": lambda value: value is None,
    "object": lambda value: True,
    "collections.abc.Sequence[str]": texts,
    "collections.abc.Sequence[tuple[str, typing.SupportsInt | typing.SupportsIndex, "
    "typing.SupportsInt | typing.SupportsIndex]]": language_ranges,
    "collections.abc.Buffer": buffer,
}


def _split(text, separator):
    """*text* split at each *separator* outside brackets."""
    parts, depth, start = [], 0, 0
    index = 0
    while index < len(text):
        char = text[index]
        if char in "[(":
            depth += 1
        elif char in "])":
            depth -= 1
        elif depth == 0 and text.startswith(separator, index):
            parts.append(text[start:index])
            index += len(separator)
            start = index
            continue
        index += 1
    parts.append(text[start:])
    return parts


def _predicate(annotation):
    """The predicate of a parameter's *annotation*, alternatives joined by
    ``|``: accepted when any accepts."""
    checks = []
    for alternative in _split(annotation.strip(), " | "):
        name = alternative.strip()
        check = ACCEPTS.get(name)
        if check is None:
            check = (_instance(name.rpartition(".")[2])
                     if name.replace(".", "").replace("_", "").isalnum()
                     else (lambda value: True))
        checks.append(check)
    return lambda value: any(check(value) for check in checks)


def _predicates(signature):
    """The predicate of each parameter of the pybind11 *signature*, by
    name (``self`` left out)."""
    inside = signature[signature.index("(") + 1:signature.rindex(") ->")]
    predicates = {}
    for parameter in _split(inside, ", ")[1:]:
        name, _, annotation = parameter.partition(": ")
        if annotation:
            predicates[name] = _predicate(annotation.partition(" = ")[0])
    return predicates


def binds(signature, **accepts):
    """
    Decorator: the method takes its arguments as the method of the
    extension of *signature* does (its text as the extension's docstring
    gives it, without the name, ``{module}`` standing for the module of
    the method's class): a call that does not bind to its parameters, or
    an argument pybind11 does not convert to the parameter's type, fails
    with pybind11's `incompatible` error. The method's own parameters are
    the extension's. The decorated method's ``check(*args, **kwargs)``
    fails so without calling it, and its ``refuses(args, kwargs)`` tells
    whether it would (the arguments' types alone decide, not ``self``).
    *accepts* gives the predicates of parameters whose C++ type the
    signature does not tell (a ``const char *``, which takes None too).
    """
    predicates = {**_predicates(signature), **accepts}

    def decorate(method):
        python = inspect.signature(method)
        # The checks of the positional parameters, in order (``self``
        # first), and how many of them a call must give: the common call,
        # without keywords, is checked without binding it.
        positional = [predicates.get(parameter.name) for parameter in python.parameters.values()
                      if parameter.kind is parameter.POSITIONAL_OR_KEYWORD]
        required = sum(parameter.default is parameter.empty
                       for parameter in python.parameters.values()
                       if parameter.kind is parameter.POSITIONAL_OR_KEYWORD)

        def refuses(args, kwargs):
            if not kwargs and required <= len(args) <= len(positional):
                return any(check is not None and not check(value)
                           for check, value in zip(positional, args))
            try:
                arguments = python.bind(*args, **kwargs).arguments
            except TypeError:
                return True
            return any(not predicates[name](value)
                       for name, value in arguments.items() if name in predicates)

        def check(*args, **kwargs):
            if refuses(args, kwargs):
                raise incompatible(method.__name__, [signature.format(
                    module=type(args[0]).__module__ if args else "")], args, kwargs)

        @functools.wraps(method)
        def bound(*args, **kwargs):
            check(*args, **kwargs)
            return method(*args, **kwargs)

        bound.check = check
        bound.refuses = refuses
        return bound

    return decorate
