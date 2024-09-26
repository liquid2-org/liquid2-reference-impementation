"""Test the Rust lexer."""

import operator
from dataclasses import dataclass

import pytest
from _liquid2 import tokenize


@dataclass
class Case:
    """Test case helper."""

    name: str
    source: str
    want: str


TEST_CASES = [
    Case(
        name="empty template",
        source="",
        want="",
    ),
    Case(
        name="just text content",
        source="Hello",
        want="Hello",
    ),
    Case(
        name="just whitespace",
        source=" \n ",
        want=" \n ",
    ),
    Case(
        name="just output",
        source="{{ foo }}",
        want="{{ foo }}",
    ),
    Case(
        name="hello liquid",
        source="Hello, {{ you }}!",
        want="Hello, {{ you }}!",
    ),
    Case(
        name="output whitespace control",
        source=(
            "Hello, "
            "{{- you -}}, {{+ you +}}, {{~ you ~}}, "
            "{{+ you -}}, {{~ you -}}, {{- you +}}!"
        ),
        want=(
            "Hello, "
            "{{- you -}}, {{+ you +}}, {{~ you ~}}, "
            "{{+ you -}}, {{~ you -}}, {{- you +}}!"
        ),
    ),
    Case(
        name="raw tag",
        source="Hello, {% raw %}{{ you }}{% endraw %}!",
        want="Hello, {% raw %}{{ you }}{% endraw %}!",
    ),
    Case(
        name="raw tag whitespace control",
        source="Hello, {%- raw +%}{{ you }}{%~ endraw -%}!",
        want="Hello, {%- raw +%}{{ you }}{%~ endraw -%}!",
    ),
    Case(
        name="comment tag",
        source="Hello, {# some comment {{ foo }} #}{{ you }}!",
        want="Hello, {# some comment {{ foo }} #}{{ you }}!",
    ),
    Case(
        name="comment tag whitespace control",
        source="Hello, {#- some comment {{ foo }} +#}{{ you }}!",
        want="Hello, {#- some comment {{ foo }} +#}{{ you }}!",
    ),
    Case(
        name="comment tag, nested",
        source="Hello, {## some comment {# other comment #} ##}{{ you }}!",
        want="Hello, {## some comment {# other comment #} ##}{{ you }}!",
    ),
    Case(
        name="assign tag",
        source="{% assign x = true %}",
        want="{% assign x = true %}",
    ),
    Case(
        name="assign tag whitespace control",
        source="{%~ assign x = true -%}",
        want="{%~ assign x = true -%}",
    ),
    Case(
        name="assign tag, filter",
        source="{% assign x = true | default: foo %}",
        want="{% assign x = true | default : foo %}",
    ),
    Case(
        name="assign tag, filters",
        source="{% assign x = true | default: foo | upcase %}",
        want="{% assign x = true | default : foo | upcase %}",
    ),
    Case(
        name="assign tag, condition",
        source="{% assign x = true if y %}",
        want="{% assign x = true if y %}",
    ),
    Case(
        name="assign tag, condition, tail filter",
        source="{% assign x = true if y || upcase %}",
        want="{% assign x = true if y || upcase %}",
    ),
    Case(
        name="assign tag, condition, tail filters",
        source="{% assign x = true if y || upcase | join : 'foo' %}",
        want="{% assign x = true if y || upcase | join : 'foo' %}",
    ),
    Case(
        name="assign tag, condition and alternative",
        source="{% assign x = true if y else z %}",
        want="{% assign x = true if y else z %}",
    ),
    Case(
        name="assign tag, condition and alternative, filter",
        source="{% assign x = true if y else z | upcase %}",
        want="{% assign x = true if y else z | upcase %}",
    ),
    Case(
        name="if tag",
        source="{% if foo %}bar{% endif %}",
        want="{% if foo %}bar{% endif %}",
    ),
    Case(
        name="if tag, else",
        source="{% if foo %}a{% else %}b{% endif %}",
        want="{% if foo %}a{% else %}b{% endif %}",
    ),
    Case(
        name="if tag, elsif",
        source="{% if foo %}a{% elsif bar %}b{% endif %}",
        want="{% if foo %}a{% elsif bar %}b{% endif %}",
    ),
    Case(
        name="if tag, elsif, whitespace control",
        source="{%- if foo ~%}a{%+ elsif bar +%}b{%~ endif -%}",
        want="{%- if foo ~%}a{%+ elsif bar +%}b{%~ endif -%}",
    ),
]


@pytest.mark.parametrize("case", TEST_CASES, ids=operator.attrgetter("name"))
def test_parser(case: Case) -> None:
    """Test the Rust parser."""
    # dump(case.source)
    assert "".join(str(t) for t in tokenize(case.source)) == case.want
