"""
Checking core metadata (``METADATA``, ``PKG-INFO``) against the
specification, version 2.4 (https://packaging.python.org/specifications/
core-metadata/), with the standard library's ``email`` parser, as PyPI
and pip read it: the fields it knows, used once where they may be used
once, each value well formed (PEP 440 versions, PEP 508 requirements,
specifier sets, SPDX licence expressions, ``label, URL`` project links).

The backend checks what it writes before it writes it, and
``python/tools/check_dist.py`` checks the archives built elsewhere.
"""

import email.parser
import email.policy
import re

#: The fields of version 2.4, and whether each may appear more than once.
FIELDS = {
    "Metadata-Version": False, "Name": False, "Version": False, "Dynamic": True,
    "Platform": True, "Supported-Platform": True, "Summary": False,
    "Description": False, "Description-Content-Type": False, "Keywords": False,
    "Author": False, "Author-email": False, "Maintainer": False,
    "Maintainer-email": False, "License": False, "License-Expression": False,
    "License-File": True, "Classifier": True, "Requires-Dist": True,
    "Requires-Python": False, "Requires-External": True, "Project-URL": True,
    "Provides-Extra": True, "Provides-Dist": True, "Obsoletes-Dist": True,
    "Home-page": False, "Download-URL": False,
}

#: A distribution name (PEP 508).
NAME = re.compile(r"[A-Z0-9]|[A-Z0-9][A-Z0-9._-]*[A-Z0-9]", re.IGNORECASE)

#: A public version with an optional local label (PEP 440, normalised).
VERSION = re.compile(
    r"([1-9][0-9]*!)?(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))*"
    r"((a|b|rc)(0|[1-9][0-9]*))?(\.post(0|[1-9][0-9]*))?(\.dev(0|[1-9][0-9]*))?"
    r"(\+[a-z0-9]+(\.[a-z0-9]+)*)?")

#: One version specifier (PEP 440).
SPECIFIER = r"\s*(~=|===?|!=|<=|>=|<|>)\s*[A-Za-z0-9_.*+!-]+\s*"

#: A specifier set.
SPECIFIERS = re.compile(rf"{SPECIFIER}(,{SPECIFIER})*")

#: A requirement (PEP 508): a name, extras, specifiers, an environment marker.
REQUIREMENT = re.compile(
    rf"(?P<name>{NAME.pattern})\s*(\[\s*[A-Za-z0-9._-]+(\s*,\s*[A-Za-z0-9._-]+)*\s*\])?"
    rf"\s*(\(\s*(?P<p>{SPECIFIERS.pattern})\s*\)|(?P<s>{SPECIFIERS.pattern}))?\s*(;.+)?",
    re.IGNORECASE)

#: Description content types.
CONTENT_TYPE = re.compile(
    r"text/(markdown|x-rst|plain)(\s*;\s*charset=UTF-8)?(\s*;\s*variant=(GFM|CommonMark))?",
    re.IGNORECASE)

#: An SPDX licence identifier, or a ``LicenseRef-``.
LICENSE_ID = re.compile(r"(LicenseRef-)?[A-Za-z0-9.-]+\+?")


def _license_expression_problem(expression):
    """Why *expression* is not an SPDX licence expression, or None."""
    tokens = re.findall(r"\(|\)|[^\s()]+", expression)
    depth, expect_id = 0, True
    for token in tokens:
        if token == "(" and expect_id:
            depth += 1
        elif token == ")" and not expect_id and depth:
            depth -= 1
        elif token in ("AND", "OR", "WITH") and not expect_id:
            expect_id = True
        elif expect_id and LICENSE_ID.fullmatch(token) and token not in ("AND", "OR", "WITH"):
            expect_id = False
        else:
            return f"unexpected {token!r}"
    return None if tokens and not expect_id and depth == 0 else "incomplete"


def _value_problem(field, value):
    """Why *value* is not a valid value of *field*, or None."""
    checks = {
        "Name": lambda: NAME.fullmatch(value),
        "Version": lambda: VERSION.fullmatch(value),
        "Requires-Python": lambda: SPECIFIERS.fullmatch(value),
        "Requires-Dist": lambda: REQUIREMENT.fullmatch(value),
        "Description-Content-Type": lambda: CONTENT_TYPE.fullmatch(value),
        "Classifier": lambda: re.fullmatch(r"[^:]+( :: [^:]+)+", value),
        "Project-URL": lambda: re.fullmatch(r"[^,]{1,32}, https?://\S+", value),
        "License-File": lambda: value and not value.startswith("/") and ".." not in value.split("/"),
        "Summary": lambda: "\n" not in value,
    }
    if field == "License-Expression":
        problem = _license_expression_problem(value)
        return None if problem is None else f"{problem} in {value!r}"
    check = checks.get(field)
    return None if check is None or check() else f"{value!r}"


def parse(text):
    """The core metadata *text* as an ``email.message.Message``."""
    return email.parser.Parser(policy=email.policy.compat32).parsestr(text)


def problems(text):
    """
    What is wrong with the core metadata *text*, as sentences: empty
    when it holds. A description must be the message body, and is then
    required when a content type is named.
    """
    message = parse(text)
    found = []
    for field in dict.fromkeys(message.keys()):
        values = message.get_all(field)
        if field not in FIELDS:
            found.append(f"unknown field {field}")
            continue
        if len(values) > 1 and not FIELDS[field]:
            found.append(f"{field} given {len(values)} times")
        found += [f"bad {field}: {problem}" for value in values
                  if (problem := _value_problem(field, value))]
    for required in ("Metadata-Version", "Name", "Version"):
        if required not in message:
            found.append(f"no {required}")
    if message.get("Metadata-Version") not in ("2.1", "2.2", "2.3", "2.4"):
        found.append(f"unsupported Metadata-Version {message.get('Metadata-Version')}")
    if "License-Expression" in message and "License" in message:
        found.append("License next to License-Expression")
    if "Description-Content-Type" in message and not message.get_payload().strip():
        found.append("a Description-Content-Type but no description")
    return found


def lower_bounds(text):
    """The lowest version each requirement of the metadata *text*
    allows (its ``>=``), by distribution name, for the requirements that
    have one and no environment marker."""
    message = parse(text)
    bounds = {}
    for requirement in message.get_all("Requires-Dist") or []:
        found = REQUIREMENT.fullmatch(requirement)
        if found is None or ";" in requirement:
            continue
        specifiers = found.group("p") or found.group("s") or ""
        lowest = re.search(r">=\s*([A-Za-z0-9_.+!-]+)", specifiers)
        if lowest:
            bounds[found.group("name")] = lowest.group(1)
    return bounds
