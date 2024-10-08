"""Template static analysis test cases."""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING
from typing import Any
from typing import Iterable
from typing import Mapping
from typing import Sequence
from typing import TypeAlias

import pytest
from liquid2 import Environment
from liquid2.static_analysis import Span

if TYPE_CHECKING:
    from liquid2 import Template
    from liquid2.static_analysis import TemplateAnalysis


@pytest.fixture
def env() -> Environment:  # noqa: D103
    return Environment()


class MockSpan:
    """A mock span containing the location of a variable, tag or filter."""

    __slots__ = ("template_name", "start", "end")

    def __init__(self, start: int, end: int, template_name: str = "<string>") -> None:
        self.template_name = template_name
        self.start = start
        self.end = end

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, (Span, MockSpan))
            and self.template_name == other.template_name
            and self.start == other.start
            and self.end == other.end
        )

    def __hash__(self) -> int:
        return hash((self.template_name, self.start, self.end))

    def __str__(self) -> str:
        return f"{self.template_name}[{self.start}:{self.end}]"


_Span = MockSpan
MockRefs: TypeAlias = Mapping[str, MockSpan | Sequence[MockSpan]]


def _assert(
    template: Template,
    *,
    local_refs: MockRefs,
    global_refs: MockRefs,
    all_refs: MockRefs | None = None,
    failed_visits: MockRefs | None = None,
    unloadable: MockRefs | None = None,
    raise_for_failures: bool = True,
    filters: MockRefs | None = None,
    tags: MockRefs | None = None,
) -> None:
    all_refs = {**global_refs} if all_refs is None else all_refs

    async def coro() -> TemplateAnalysis:
        return await template.analyze_async(raise_for_failures=raise_for_failures)

    def _assert_refs(refs: TemplateAnalysis) -> None:
        assert _as_strings(refs.local_variables) == _as_strings(local_refs)
        assert _as_strings(refs.global_variables) == _as_strings(global_refs)
        assert _as_strings(refs.variables) == _as_strings(all_refs)

        if failed_visits:
            assert _as_strings(refs.failed_visits) == _as_strings(failed_visits)
        else:
            assert len(refs.failed_visits) == 0

        if unloadable:
            assert _as_strings(refs.unloadable_partials) == _as_strings(unloadable)
        else:
            assert len(refs.unloadable_partials) == 0

        if filters:
            assert _as_strings(refs.filters) == _as_strings(filters)
        else:
            assert len(refs.filters) == 0

        if tags:
            assert _as_strings(refs.tags) == _as_strings(tags)
        else:
            assert len(refs.tags) == 0

    _assert_refs(template.analyze(raise_for_failures=raise_for_failures))
    _assert_refs(asyncio.run(coro()))


def _as_strings(
    refs: Mapping[Any, Any],
) -> dict[str, list[str]]:
    _refs: dict[str, list[str]] = {}
    for k, v in refs.items():
        if isinstance(v, Iterable):
            _refs[str(k)] = [str(_v) for _v in v]
        else:
            _refs[str(k)] = [str(v)]
    return _refs


def test_analyze_output(env: Environment) -> None:
    source = r"{{ x | default: y, allow_false: z }}"

    _assert(
        env.from_string(source),
        local_refs={},
        global_refs={
            "x": _Span(3, 4),
            "y": _Span(16, 17),
            "z": _Span(32, 33),
        },
        filters={
            "default": _Span(7, 14),
        },
    )


def test_bracketed_query_notation(env: Environment) -> None:
    source = r"{{ x['y'].title }}"

    _assert(
        env.from_string(source),
        local_refs={},
        global_refs={"x.y.title": _Span(3, 15)},
    )


def test_quoted_name_notation(env: Environment) -> None:
    source = r"{{ some['foo.bar'] }}"

    _assert(
        env.from_string(source),
        local_refs={},
        global_refs={"some['foo.bar']": _Span(3, 18)},
    )


def test_nested_queries(env: Environment) -> None:
    source = r"{{ x[y.z].title }}"

    _assert(
        env.from_string(source),
        local_refs={},
        global_refs={
            "x[y.z].title": _Span(3, 15),
            "y.z": _Span(5, 8),
        },
    )


def test_analyze_assign(env: Environment) -> None:
    source = r"{% assign x = y | append: z %}"

    _assert(
        env.from_string(source),
        local_refs={"x": _Span(10, 11)},
        global_refs={
            "y": _Span(14, 15),
            "z": _Span(26, 27),
        },
        filters={"append": _Span(18, 24)},
        tags={"assign": _Span(0, 30)},
    )


def test_analyze_capture(env: Environment) -> None:
    source = r"{% capture x %}{% if y %}z{% endif %}{% endcapture %}"

    _assert(
        env.from_string(source),
        local_refs={"x": _Span(11, 12)},
        global_refs={
            "y": _Span(21, 22),
        },
        tags={
            "capture": _Span(0, 15),
            "if": _Span(15, 25),
        },
    )


def test_analyze_case(env: Environment) -> None:
    source = "\n".join(
        [
            "{% case x %}",
            "{% when y %}",
            "  {{ a }}",
            "{% when z %}",
            "  {{ b }}",
            "{% endcase %}",
        ]
    )

    # TODO: else

    _assert(
        env.from_string(source),
        local_refs={},
        global_refs={
            "x": _Span(8, 9),
            "y": _Span(21, 22),
            "a": _Span(31, 32),
            "z": _Span(44, 45),
            "b": _Span(54, 55),
        },
        tags={
            "case": _Span(0, 12),
            "when": [
                _Span(13, 25),
                _Span(36, 48),
            ],
        },
    )


def test_analyze_cycle(env: Environment) -> None:
    source = r"{% cycle x: a, b %}"

    _assert(
        env.from_string(source),
        local_refs={},
        global_refs={
            "a": _Span(12, 13),
            "b": _Span(15, 16),
        },
        tags={"cycle": _Span(0, 19)},
    )


def test_analyze_decrement(env: Environment) -> None:
    source = r"{% decrement x %}"

    _assert(
        env.from_string(source),
        local_refs={"x": _Span(13, 14)},
        global_refs={},
        tags={"decrement": _Span(0, 17)},
    )


def test_analyze_echo(env: Environment) -> None:
    source = r"{% echo x | default: y, allow_false: z %}"

    _assert(
        env.from_string(source),
        local_refs={},
        global_refs={
            "x": _Span(8, 9),
            "y": _Span(21, 22),
            "z": _Span(37, 38),
        },
        filters={
            "default": _Span(12, 19),
        },
        tags={"echo": _Span(0, 41)},
    )
