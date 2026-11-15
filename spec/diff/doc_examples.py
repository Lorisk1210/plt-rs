"""The code of documentation's examples, for the corpora (``corpora.py``).

Documentation shows its examples in three forms, which a corpus whose
programs are documentation pages or modules runs as cells:

* the plot directive of matplotlib's Sphinx extension (``.. plot::``),
  whose code inline (not one naming a file) Sphinx runs at every build;
* doctest blocks, paragraphs of ``>>>`` and ``...`` lines and what they
  print, run only by Sphinx's doctest builder: a corpus asks for them
  (``doctests``);
* the ``Examples`` sections of docstrings, numpydoc's, read from a
  module's syntax tree: their plot directives, doctest blocks and literal
  blocks (a paragraph ending in ``::`` and the lines indented under it).

``unescape_doctest`` and ``contains_doctest`` are those of matplotlib's
plot directive (``matplotlib.sphinxext.plot_directive``), under
matplotlib's licence: a plot directive's code is what Sphinx runs.

It uses the standard library alone.
"""

from __future__ import annotations

import ast
import re
import textwrap

#: A plot directive with its code inline: its options and code, the lines
#: indented under it (or blank). One naming a file has text after ``::``.
_PLOT = re.compile(
    r"^(?P<indent>[ \t]*)\.\. plot::[ \t]*\n(?P<body>(?:(?P=indent)[ \t]+\S.*\n|[ \t]*\n)*)", re.M)
#: A doctest block: a paragraph whose first line starts with ``>>>``.
_DOCTEST = re.compile(r"^(?P<indent>[ \t]*)>>>.*\n(?:(?P=indent)\S.*\n)*", re.M)
#: A literal block: the paragraph line ending in ``::`` that opens it, and
#: the lines indented under it (or blank).
_LITERAL = re.compile(
    r"^(?P<indent>[ \t]*)(?!\.\. )\S.*::[ \t]*\n(?P<body>(?:[ \t]*\n)+(?:(?P=indent)[ \t]+\S.*\n|[ \t]*\n)*)", re.M)
#: A doctest's line of code: its prompt and the code after it.
_DOCTEST_LINE = re.compile(r"^\s*(>>>|\.\.\.) (.*)$")
#: The heading of a numpydoc section, underlined.
_SECTION = re.compile(r"^(?P<title>\S[^\n]*)\n-{3,}[ \t]*$", re.M)


def contains_doctest(text: str) -> bool:
    """Whether *text* is doctest-formatted code (matplotlib's plot
    directive's test): no Python as it is, and a line starting ``>>>``."""
    try:
        compile(text, "<string>", "exec")
    except SyntaxError:
        return bool(re.search(r"^\s*>>>", text, re.M))
    return False


def unescape_doctest(text: str) -> str:
    """The code of doctest-formatted *text*, as matplotlib's plot directive
    runs it: the code of its prompted lines, the lines they print as
    comments; *text* itself when it is no doctest."""
    if not contains_doctest(text):
        return text
    code = ""
    for line in text.split("\n"):
        match = _DOCTEST_LINE.match(line)
        if match:
            code += match.group(2) + "\n"
        elif line.strip():
            code += "# " + line.strip() + "\n"
        else:
            code += "\n"
    return code


def _directive_code(body: str) -> str:
    """The code of a directive's *body*: dedented, its options (the field
    list its first lines are, up to a blank line) left out."""
    body = textwrap.dedent(body)
    if body.startswith(":"):
        _options, _blank, body = body.partition("\n\n")
    return unescape_doctest(body)


def page_sources(text: str, doctests: bool) -> list[tuple[int, str]]:
    """The code of a reStructuredText page's plot directives with their code
    inline and, with *doctests*, of its doctest blocks (those outside the
    directives), each with its offset in *text*."""
    found = [(match.start(), match.end(), _directive_code(match["body"])) for match in _PLOT.finditer(text)]
    if doctests:
        spans = [(start, end) for start, end, _code in found]
        found += [
            (match.start(), match.end(), unescape_doctest(textwrap.dedent(match[0])))
            for match in _DOCTEST.finditer(text)
            if not any(start <= match.start() < end for start, end in spans)
        ]
    return [(start, code) for start, _end, code in sorted(found)]


def _examples_section(docstring: str) -> str:
    """The ``Examples`` (or ``Example``) section of a numpydoc docstring,
    up to the next section's heading; empty without one."""
    headings = list(_SECTION.finditer(docstring))
    for index, heading in enumerate(headings):
        if heading["title"].strip() in ("Examples", "Example"):
            end = headings[index + 1].start() if index + 1 < len(headings) else len(docstring)
            return docstring[heading.end():end] + "\n"
    return ""


def _section_sources(section: str) -> list[str]:
    """The code of an ``Examples`` section: its plot directives with code
    inline, its doctest blocks and its literal blocks, in its order."""
    found = [(match.start(), match.end(), _directive_code(match["body"])) for match in _PLOT.finditer(section)]
    found += [(match.start(), match.end(), unescape_doctest(textwrap.dedent(match["body"])))
              for match in _LITERAL.finditer(section)]
    spans = [(start, end) for start, end, _code in found]
    found += [(match.start(), match.end(), unescape_doctest(textwrap.dedent(match[0])))
              for match in _DOCTEST.finditer(section)
              if not any(start <= match.start() < end for start, end in spans)]
    return [code for _start, _end, code in sorted(found)]


def docstring_sources(source: str) -> list[str]:
    """The code of the ``Examples`` sections of a module's docstrings (the
    module's, its classes' and its functions', in the order of their
    lines); none for a module Python cannot parse."""
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return []
    documented = sorted(
        (node for node in ast.walk(tree)
         if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef))),
        key=lambda node: getattr(node, "lineno", 0))
    sources = []
    for node in documented:
        docstring = ast.get_docstring(node, clean=True)
        if docstring:
            sources += _section_sources(_examples_section(docstring))
    return sources
