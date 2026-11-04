"""
Programs of the differential tester (``spec/diff/generators``), mutated.

``spec/diff/generate.py`` writes its programs under real matplotlib (the
reference venv), looking at matplotlib's API: every getter and setter of
every artist, every plotting method with argument variants, pyplot, the
formatters, norms, transforms, layouts, toolkits, and random sequences of
them. They are written once into the fuzzer's directory. A program taken
from them is mutated with its seed: its literals are replaced, now and
then, by the borderline and absurd values of `values`, and draws, saves,
pickles and events are slipped in between its statements. Its imports
name pltrs instead of matplotlib.
"""

import ast
import pathlib
import random
import subprocess

from . import actions, values

#: The repository's root.
ROOT = pathlib.Path(__file__).resolve().parents[3]

#: The reference venv's interpreter, which has matplotlib.
REFERENCE_PYTHON = ROOT / ".venv" / "bin" / "python"

#: Statements slipped between a program's own.
STRESS = [
    "plt.gcf().canvas.draw()",
    "plt.gcf().savefig(io.BytesIO(), format={format!r})",
    "pickle.loads(pickle.dumps(plt.gcf())).canvas.draw()",
    "plt.gcf().tight_layout()",
    "MouseEvent('button_press_event', plt.gcf().canvas, 40, 40, 1)._process()",
    "KeyEvent('key_press_event', plt.gcf().canvas, 'l', 40, 40)._process()",
    "plt.gca().set_xlim({a}, {b})",
    "plt.gca().autoscale()",
]


def load(directory):
    """The differential tester's programs, written into *directory*
    (once, with the reference venv) and read from there; none without
    the reference venv."""
    corpus = directory / "corpus"
    if not corpus.is_dir():
        if not REFERENCE_PYTHON.exists():
            return []
        subprocess.run([str(REFERENCE_PYTHON), str(ROOT / "spec" / "diff" / "generate.py"),
                        str(corpus)], cwd=directory, check=True, capture_output=True)
    return [path.read_text() for path in sorted(corpus.rglob("*.py"))]


def _to_pltrs(statement):
    return (statement.replace("mpl_toolkits", "pltrs.mpl_toolkits")
            .replace("matplotlib", "pltrs"))


class _Literals(ast.NodeTransformer):
    """Replaces literals by random values, each with probability `rate`."""

    def __init__(self, rng, rate):
        self.rng = rng
        self.rate = rate
        self.values = values.Values(rng, 1.0)

    def _replacement(self, source):
        return ast.parse(source, mode="eval").body

    def visit_Constant(self, node):
        if self.rng.random() >= self.rate:
            return node
        if isinstance(node.value, bool) or node.value is None:
            return self._replacement(self.values.any_value())
        if isinstance(node.value, (int, float)):
            return self._replacement(self.values.number())
        if isinstance(node.value, str):
            return self._replacement(self.values.string() if self.rng.random() < 0.7
                                     else self.values.any_value())
        return node

    def visit_JoinedStr(self, node):
        # The parts of an f-string are no values of their own.
        return node

    def visit_List(self, node):
        if self.rng.random() < self.rate:
            return self._replacement(self.values.array())
        return self.generic_visit(node)


def mutate(source, seed):
    """The statements of the program *source*, mutated with *seed*."""
    rng = random.Random(seed)
    statements = [ast.get_source_segment(source, node) for node in ast.parse(source).body]
    rate = rng.choice([0.0, 0.02, 0.05, 0.15])
    mutated = list(actions.HEADER)
    for statement in statements:
        statement = _to_pltrs(statement)
        if not statement.startswith(("import", "from")):
            tree = _Literals(rng, rate).visit(ast.parse(statement))
            statement = ast.unparse(ast.fix_missing_locations(tree))
        mutated.append(statement)
        if rng.random() < 0.08:
            stress = values.Values(rng, 0.5)
            mutated.append(values.choose(rng, STRESS).format(
                format=values.choose(rng, actions.FORMATS), a=stress.number(),
                b=stress.number()))
    return mutated
