"""Reduces a difference to the fewest statements still showing it.

``reduce(setup, difference, run_program)`` cuts the program of
``difference`` after the top-level statement where it shows, keeps the
imports and the statements defining the names that statement uses
(transitively), and then drops the remaining statements one at a time,
last first, as long as the cut program still shows a difference of the
same cause under both implementations. The result goes into the
difference as ``reduced``. Image differences are not reduced (every
statement draws), nor are those of the corpora's programs (they run in
their trees).
"""

from __future__ import annotations

import ast
import pathlib
from typing import Any, Callable

#: The most runs (each under both implementations) one reduction makes.
MAX_TRIES = 40


def _names(node: ast.AST, context: type) -> set[str]:
    return {n.id for n in ast.walk(node) if isinstance(n, ast.Name) and isinstance(n.ctx, context)}


def _defined(statement: ast.stmt) -> set[str]:
    names = _names(statement, ast.Store)
    if isinstance(statement, (ast.Import, ast.ImportFrom)):
        names |= {(a.asname or a.name).split(".")[0] for a in statement.names}
    if isinstance(statement, (ast.FunctionDef, ast.ClassDef)):
        names.add(statement.name)
    return names


def _slice(statements: list[ast.stmt], target: int) -> list[int]:
    """The statements the target needs: imports, and the definitions of the
    names it uses, transitively."""
    keep = {target}
    needed = _names(statements[target], ast.Load)
    for index in range(target - 1, -1, -1):
        statement = statements[index]
        if isinstance(statement, (ast.Import, ast.ImportFrom)):
            keep.add(index)
        elif _defined(statement) & needed:
            keep.add(index)
            needed |= _names(statement, ast.Load)
    return sorted(keep)


def _source(text: str, statements: list[ast.stmt], kept: list[int]) -> str:
    return "\n".join(ast.get_source_segment(text, statements[i]) or ast.unparse(statements[i]) for i in kept) + "\n"


def reduce(setup: Any, difference: dict[str, Any], run_program: Callable) -> None:
    """Reduce *difference* in place (its ``reduced`` program)."""
    from driver import Program

    if difference["kind"] == "image" or difference.get("line") is None or difference["program"].startswith("corpus/"):
        return
    path = pathlib.Path(difference["program_path"]) if "program_path" in difference else None
    if path is None:
        return
    text = path.read_text()
    statements = ast.parse(text).body
    target = next((i for i, s in enumerate(statements)
                   if s.lineno <= difference["line"] <= (s.end_lineno or s.lineno)), None)
    if target is None:
        return
    mode = "steps" if difference["program"].startswith("generated/") else "program"
    directory = setup.out / "reduce"
    directory.mkdir(parents=True, exist_ok=True)
    tries = 0

    def shows(kept: list[int]) -> bool:
        """Whether the statements *kept* still show the difference."""
        nonlocal tries
        tries += 1
        candidate = directory / f"{abs(hash(difference['cause'])):x}-{tries}.py"
        candidate.write_text(_source(text, statements, kept))
        result = run_program(setup, Program(f"reduce/{candidate.stem}", candidate, mode))
        return any(d["cause"] == difference["cause"] for d in result.get("differences", []))

    kept = _slice(statements, target)
    if not shows(kept):
        kept = list(range(target + 1))
        if not shows(kept):
            return
    for index in reversed(kept[:-1]):
        if tries >= MAX_TRIES:
            break
        if isinstance(statements[index], (ast.Import, ast.ImportFrom)):
            continue
        trial = [i for i in kept if i != index]
        if shows(trial):
            kept = trial
    difference["reduced"] = _source(text, statements, kept).strip()
