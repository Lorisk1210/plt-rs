"""
Checks the GitHub Actions workflows (``.github/workflows/*.yml``) without
running them, with a reader of the YAML they use (block mappings and
sequences, flow ``[...]`` and ``{...}``, quoted and plain scalars,
``|`` blocks, comments) and the rules that matter here:

* they run only when started by hand or by a tag (``workflow_dispatch``,
  ``push`` of ``tags``): the repository is private and runs cost minutes;
* every job has ``runs-on``, ``timeout-minutes`` and steps, each step
  ``uses`` or ``run``; actions are pinned to a commit (40 hex digits);
* ``needs`` names jobs that exist, without cycles; ``needs.<job>.outputs``
  and ``steps.<id>.outputs`` name outputs and steps that exist; a job's
  ``if`` uses no context it cannot see (``matrix``, ``steps``, ...);
* the scripts the steps run exist in the tree;
* a job that may publish (``id-token: write``) runs for a tag only, in an
  environment.

    python python/tools/check_workflows.py [WORKFLOW...]

prints each problem and exits with 1 if there is one.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]

#: The events a workflow may run for.
ALLOWED_EVENTS = {"workflow_dispatch", "push"}

#: The contexts a job's ``if`` may use (GitHub's "Contexts" reference).
JOB_IF_CONTEXTS = {"github", "needs", "vars", "inputs"}


class YamlError(ValueError):
    """YAML this reader does not take, with its line."""


def _strip_comment(line):
    """*line* without its comment (a ``#`` after a space, outside quotes)."""
    quote = None
    for index, char in enumerate(line):
        if quote:
            if char == quote:
                quote = None
        elif char in "\"'" and (index == 0 or line[index - 1] in " [{,:"):
            quote = char
        elif char == "#" and (index == 0 or line[index - 1] in " \t"):
            return line[:index].rstrip()
    return line.rstrip()


def _split_flow(text):
    """The comma-separated items of the inside of a flow collection."""
    items, depth, quote, start = [], 0, None, 0
    for index, char in enumerate(text):
        if quote:
            quote = None if char == quote else quote
        elif char in "\"'":
            quote = char
        elif char in "[{":
            depth += 1
        elif char in "]}":
            depth -= 1
        elif char == "," and depth == 0:
            items.append(text[start:index].strip())
            start = index + 1
    last = text[start:].strip()
    return items + [last] if last else items


def _key_value(text):
    """``(key, rest)`` of a mapping entry ``key: rest`` or ``key:``, or
    None if *text* is no mapping entry."""
    found = re.match(r"""("[^"]*"|'[^']*'|[^\s"'{}\[\],#][^:]*?)\s*:(\s+|$)""", text)
    if found is None:
        return None
    return scalar(found.group(1)), text[found.end():].strip()


def scalar(text):
    """The value of an inline YAML value: a flow collection, a quoted or
    plain scalar (``true``/``false`` as booleans, the rest as strings)."""
    if text.startswith("[") and text.endswith("]"):
        return [scalar(item) for item in _split_flow(text[1:-1])]
    if text.startswith("{") and text.endswith("}"):
        mapping = {}
        for item in _split_flow(text[1:-1]):
            entry = _key_value(item)
            if entry is None:
                raise YamlError(f"not a mapping entry: {item}")
            mapping[entry[0]] = scalar(entry[1])
        return mapping
    if len(text) >= 2 and text[0] == text[-1] == '"':
        return text[1:-1].replace('\\"', '"').replace("\\\\", "\\")
    if len(text) >= 2 and text[0] == text[-1] == "'":
        return text[1:-1].replace("''", "'")
    return {"true": True, "false": False}.get(text, text)


class _Reader:
    """Reads a document line by line, by indentation."""

    def __init__(self, text):
        self.raw = text.splitlines()
        self.lines = []
        for number, line in enumerate(self.raw):
            content = _strip_comment(line)
            if content.strip():
                if "\t" in line[:len(line) - len(line.lstrip())]:
                    raise YamlError(f"line {number + 1}: a tab in the indentation")
                self.lines.append([len(content) - len(content.lstrip()), content.strip(), number])
        self.pos = 0

    def peek(self):
        return self.lines[self.pos] if self.pos < len(self.lines) else None

    def block(self, parent):
        """The block nested deeper than the indentation *parent*, or None."""
        line = self.peek()
        if line is None or line[0] <= parent:
            return None
        return self.sequence(line[0]) if line[1].startswith("- ") or line[1] == "-" \
            else self.mapping(line[0])

    def mapping(self, indent):
        result = {}
        while (line := self.peek()) and line[0] == indent and not line[1].startswith("-"):
            entry = _key_value(line[1])
            if entry is None:
                raise YamlError(f"line {line[2] + 1}: not a mapping entry: {line[1]}")
            key, rest = entry
            if key in result:
                raise YamlError(f"line {line[2] + 1}: {key} given twice")
            self.pos += 1
            result[key] = self.value(rest, indent, line[2])
        if (line := self.peek()) and line[0] > indent:
            raise YamlError(f"line {line[2] + 1}: unexpected indentation")
        return result

    def sequence(self, indent):
        items = []
        while (line := self.peek()) and line[0] == indent and (
                line[1].startswith("- ") or line[1] == "-"):
            after = line[1][1:].lstrip()
            if not after:
                self.pos += 1
                items.append(self.block(indent))
            elif _key_value(after) and not after.startswith(("[", "{", '"', "'")):
                # "- key: value" starts a mapping indented as its key.
                line[0], line[1] = indent + len(line[1]) - len(after), after
                items.append(self.mapping(line[0]))
            else:
                self.pos += 1
                items.append(self.value(after, indent, line[2]))
        return items

    def value(self, rest, indent, number):
        if rest in ("|", "|-", "|+", ">", ">-", ">+"):
            return self.block_scalar(indent, number)
        if rest:
            return scalar(rest)
        line = self.peek()
        if line and line[0] == indent and line[1].startswith("-"):
            return self.sequence(indent)
        return self.block(indent)

    def block_scalar(self, indent, number):
        """The text of a ``|`` block after line *number*: the raw lines
        indented deeper than *indent*."""
        text, end = [], number + 1
        while end < len(self.raw) and (not self.raw[end].strip() or (
                len(self.raw[end]) - len(self.raw[end].lstrip()) > indent)):
            text.append(self.raw[end])
            end += 1
        while self.pos < len(self.lines) and self.lines[self.pos][2] < end:
            self.pos += 1
        margin = min((len(line) - len(line.lstrip()) for line in text if line.strip()), default=0)
        return "\n".join(line[margin:] for line in text).strip("\n") + "\n"


def parse(text):
    """The YAML document *text* as dicts, lists, strings and booleans."""
    reader = _Reader(text)
    document = reader.block(-1)
    if reader.peek() is not None:
        raise YamlError(f"line {reader.peek()[2] + 1}: unexpected indentation")
    return document


def _strings(value):
    """Every string in *value*, however nested."""
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for item in value.values():
            yield from _strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from _strings(item)


def _expressions(text):
    """The insides of the ``${{ }}`` of *text*."""
    return re.findall(r"\$\{\{(.*?)\}\}", text)


def _trigger_problems(workflow):
    events = workflow.get("on")
    if not isinstance(events, dict):
        return ["'on' is not a mapping of events"]
    found = [f"runs for {event}" for event in events if event not in ALLOWED_EVENTS]
    push = events.get("push")
    if "push" in events and (not isinstance(push, dict) or set(push) != {"tags"}):
        found.append("runs for pushes other than of tags")
    return found


def _job_problems(name, job, jobs):
    """What is wrong with the job *name*."""
    found = []
    for key in ("runs-on", "timeout-minutes", "steps"):
        if key not in job:
            found.append(f"{name}: no {key}")
    needs = job.get("needs", [])
    needs = [needs] if isinstance(needs, str) else needs
    found += [f"{name}: needs the unknown job {need}" for need in needs if need not in jobs]
    condition = job.get("if", "")
    for context in re.findall(r"\b([a-z_]+)\.", condition):
        if context not in JOB_IF_CONTEXTS:
            found.append(f"{name}: its if uses {context}, which a job's if cannot see")
    ids = {step.get("id") for step in job.get("steps", []) if isinstance(step, dict)}
    for text in _strings(job):
        for expression in _expressions(text):
            for job_name, output in re.findall(r"needs\.([\w-]+)\.outputs\.([\w-]+)", expression):
                if job_name not in needs:
                    found.append(f"{name}: uses outputs of {job_name}, which it does not need")
                elif output not in jobs.get(job_name, {}).get("outputs", {}):
                    found.append(f"{name}: {job_name} has no output {output}")
            for step in re.findall(r"steps\.([\w-]+)\.", expression):
                if step not in ids:
                    found.append(f"{name}: no step {step}")
    for step in job.get("steps", []):
        found += [f"{name}: {problem}" for problem in _step_problems(step)]
    if job.get("permissions", {}).get("id-token") == "write":
        if "environment" not in job or "refs/tags/" not in condition:
            found.append(f"{name}: may publish, but not for tags only in an environment")
    return found


def _step_problems(step):
    """What is wrong with one step."""
    if not isinstance(step, dict) or ("uses" in step) == ("run" in step):
        return [f"the step {step!r} must have uses or run"]
    found = []
    if "uses" in step and not re.fullmatch(r"[\w.-]+/[\w./-]+@[0-9a-f]{40}", step["uses"]):
        found.append(f"{step['uses']} is not pinned to a commit")
    for script in re.findall(r"((?:\.github|python)/[\w./-]+\.(?:sh|py))", step.get("run", "")):
        if not (ROOT / script).is_file():
            found.append(f"runs {script}, which the tree lacks")
    return found


def problems(text):
    """What is wrong with the workflow *text*."""
    try:
        workflow = parse(text)
    except YamlError as error:
        return [f"YAML: {error}"]
    if not isinstance(workflow, dict) or not isinstance(workflow.get("jobs"), dict):
        return ["no jobs"]
    found = _trigger_problems(workflow)
    if workflow.get("permissions") != {"contents": "read"}:
        found.append("the workflow's permissions are not contents: read")
    jobs = workflow["jobs"]
    for name, job in jobs.items():
        found += _job_problems(name, job, jobs)
    for text_value in _strings(workflow):
        if text_value.count("${{") != text_value.count("}}"):
            found.append(f"unbalanced expression in {text_value[:60]!r}")
    found += _cycle_problems(jobs)
    return found


def _cycle_problems(jobs):
    """Whether the jobs' ``needs`` loop."""
    state = {}

    def visit(name):
        if state.get(name) == "done":
            return False
        if state.get(name) == "open":
            return True
        state[name] = "open"
        needs = jobs.get(name, {}).get("needs", [])
        looped = any(visit(need) for need in ([needs] if isinstance(needs, str) else needs))
        state[name] = "done"
        return looped

    return ["the jobs' needs loop"] if any(visit(name) for name in jobs) else []


def main(argv):
    """Checks the workflows named (all of the tree's by default)."""
    paths = [pathlib.Path(arg) for arg in argv] or sorted(
        (ROOT / ".github" / "workflows").glob("*.yml"))
    failed = False
    for path in paths:
        found = problems(path.read_text(encoding="utf-8"))
        print(f"{path.name}: {'fine' if not found else 'WRONG'}")
        for problem in found:
            print(f"  {problem}")
        failed = failed or bool(found)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
