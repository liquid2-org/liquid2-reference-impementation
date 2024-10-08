"""JSONPath selector definitions."""

from __future__ import annotations

import random
from abc import ABC
from abc import abstractmethod
from contextlib import suppress
from typing import TYPE_CHECKING
from typing import Any
from typing import Iterable
from typing import Mapping
from typing import Sequence

from .exceptions import JSONPathIndexError
from .exceptions import JSONPathTypeError
from .filter_expressions import FilterContext

if TYPE_CHECKING:
    from liquid2 import TokenT

    from .environment import _JSONPathEnvironment
    from .filter_expressions import FilterExpression
    from .node import JSONPathNode
    from .query import JSONPathQuery
    from .query import SelectorTuple


class JSONPathSelector(ABC):
    """Base class for all JSONPath selectors."""

    __slots__ = ("env", "token")

    def __init__(self, *, env: _JSONPathEnvironment, token: TokenT) -> None:
        self.env = env
        self.token = token

    @abstractmethod
    def resolve(self, node: JSONPathNode) -> Iterable[JSONPathNode]:
        """Apply the segment/selector to _node_.

        Arguments:
            node: A node matched by preceding segments/selectors.

        Returns:
            The `JSONPathNode` instances created by applying this selector to _node_.
        """

    def as_tuple(self) -> SelectorTuple | str:
        """Return this selector as a tuple of strings and/or nested tuples."""
        return str(self)


class NameSelector(JSONPathSelector):
    """The name selector."""

    __slots__ = ("name",)

    def __init__(
        self,
        *,
        env: _JSONPathEnvironment,
        token: TokenT,
        name: str,
    ) -> None:
        super().__init__(env=env, token=token)
        self.name = name

    def __str__(self) -> str:
        return self.name

    def __eq__(self, __value: object) -> bool:
        return isinstance(__value, NameSelector) and self.name == __value.name

    def __hash__(self) -> int:
        return hash(self.name)

    def resolve(self, node: JSONPathNode) -> Iterable[JSONPathNode]:
        """Select a value from a dict/object by its property/key."""
        if isinstance(node.value, Mapping):
            with suppress(KeyError):
                yield node.new_child(node.value[self.name], self.name)


class IndexSelector(JSONPathSelector):
    """The array index selector."""

    __slots__ = ("index", "_as_key")

    def __init__(
        self,
        *,
        env: _JSONPathEnvironment,
        token: TokenT,
        index: int,
    ) -> None:
        if index < env.min_int_index or index > env.max_int_index:
            raise JSONPathIndexError("index out of range", token=token)

        super().__init__(env=env, token=token)
        self.index = index
        self._as_key = str(self.index)

    def __str__(self) -> str:
        return str(self.index)

    def __eq__(self, __value: object) -> bool:
        return isinstance(__value, IndexSelector) and self.index == __value.index

    def __hash__(self) -> int:
        return hash(self.index)

    def _normalized_index(self, obj: Sequence[object]) -> int:
        if self.index < 0 and len(obj) >= abs(self.index):
            return len(obj) + self.index
        return self.index

    def resolve(self, node: JSONPathNode) -> Iterable[JSONPathNode]:
        """Select an element from an array by index."""
        if isinstance(node.value, Sequence):
            norm_index = self._normalized_index(node.value)
            with suppress(IndexError):
                yield node.new_child(node.value[self.index], norm_index)


class SliceSelector(JSONPathSelector):
    """Array/List slicing selector."""

    __slots__ = ("slice",)

    def __init__(
        self,
        *,
        env: _JSONPathEnvironment,
        token: TokenT,
        start: int | None = None,
        stop: int | None = None,
        step: int | None = None,
    ) -> None:
        super().__init__(env=env, token=token)
        self._check_range(start, stop, step)
        self.slice = slice(start, stop, step)

    def __str__(self) -> str:
        stop = self.slice.stop if self.slice.stop is not None else ""
        start = self.slice.start if self.slice.start is not None else ""
        step = self.slice.step if self.slice.step is not None else "1"
        return f"{start}:{stop}:{step}"

    def __eq__(self, __value: object) -> bool:
        return isinstance(__value, SliceSelector) and self.slice == __value.slice

    def __hash__(self) -> int:
        return hash(str(self))

    def _check_range(self, *indices: int | None) -> None:
        for i in indices:
            if i is not None and (
                i < self.env.min_int_index or i > self.env.max_int_index
            ):
                raise JSONPathIndexError("index out of range", token=self.token)

    def _normalized_index(self, obj: Sequence[object], index: int) -> int:
        if index < 0 and len(obj) >= abs(index):
            return len(obj) + index
        return index

    def resolve(self, node: JSONPathNode) -> Iterable[JSONPathNode]:
        """Select a range of values from an array/list."""
        if isinstance(node.value, Sequence) and self.slice.step != 0:
            idx = self.slice.start or 0
            step = self.slice.step or 1
            for element in node.value[self.slice]:
                yield node.new_child(element, self._normalized_index(node.value, idx))
                idx += step


class WildcardSelector(JSONPathSelector):
    """The wildcard selector."""

    def __init__(self, *, env: _JSONPathEnvironment, token: TokenT) -> None:
        super().__init__(env=env, token=token)

    def __str__(self) -> str:
        return "*"

    def __eq__(self, __value: object) -> bool:
        return isinstance(__value, WildcardSelector)

    def __hash__(self) -> int:
        return hash("*")

    def resolve(self, node: JSONPathNode) -> Iterable[JSONPathNode]:
        """Select all elements from a array/list or values from a dict/object."""
        if isinstance(node.value, Mapping):
            if self.env.nondeterministic:
                _members = list(node.value.items())
                random.shuffle(_members)
                members: Iterable[Any] = iter(_members)
            else:
                members = node.value.items()

            for name, val in members:
                yield node.new_child(val, name)

        elif isinstance(node.value, Sequence):
            for i, element in enumerate(node.value):
                yield node.new_child(element, i)


class Filter(JSONPathSelector):
    """Filter array/list items or dict/object values with a filter expression."""

    __slots__ = ("expression",)

    def __init__(
        self,
        *,
        env: _JSONPathEnvironment,
        token: TokenT,
        expression: FilterExpression,
    ) -> None:
        super().__init__(env=env, token=token)
        self.expression = expression

    def __str__(self) -> str:
        return f"?{self.expression}"

    def __eq__(self, __value: object) -> bool:
        return isinstance(__value, Filter) and self.expression == __value.expression

    def __hash__(self) -> int:
        return hash(str(self.expression))

    def resolve(self, node: JSONPathNode) -> Iterable[JSONPathNode]:  # noqa: PLR0912
        """Select array/list items or dict/object values where with a filter."""
        if isinstance(node.value, Mapping):
            if self.env.nondeterministic:
                _members = list(node.value.items())
                random.shuffle(_members)
                members: Iterable[Any] = iter(_members)
            else:
                members = node.value.items()

            for name, val in members:
                context = FilterContext(
                    env=self.env,
                    current=val,
                    root=node.root,
                )
                try:
                    if self.expression.evaluate(context):
                        yield node.new_child(val, name)
                except JSONPathTypeError as err:
                    if not err.token:
                        err.token = self.token
                    raise

        elif isinstance(node.value, Sequence):
            for i, element in enumerate(node.value):
                context = FilterContext(
                    env=self.env,
                    current=element,
                    root=node.root,
                )
                try:
                    if self.expression.evaluate(context):
                        yield node.new_child(element, i)
                except JSONPathTypeError as err:
                    if not err.token:
                        err.token = self.token
                    raise


class SingularQuerySelector(JSONPathSelector):
    """Nested query selector."""

    __slots__ = ("query",)

    def __init__(
        self,
        *,
        env: _JSONPathEnvironment,
        token: TokenT,
        query: JSONPathQuery,
    ) -> None:
        super().__init__(env=env, token=token)
        self.query = query
        self.query.token = token  # XXX: bit of a hack

    def __str__(self) -> str:
        return str(self.query)

    def __eq__(self, value: object) -> bool:
        return isinstance(value, SingularQuerySelector) and self.query == value.query

    def __hash__(self) -> int:
        return hash(str(self.query))

    def _normalized_index(self, index: int, obj: Sequence[object]) -> int:
        if index < 0 and len(obj) >= abs(index):
            return len(obj) + index
        return index

    def resolve(self, node: JSONPathNode) -> Iterable[JSONPathNode]:
        """Select array items or object values using the result of an embedded query."""
        # XXX: assuming root query
        nodes = self.query.find(node.root)
        if not nodes:
            return

        value = nodes[0].value

        if isinstance(value, int) and isinstance(node.value, Sequence):
            norm_index = self._normalized_index(value, node.value)
            with suppress(IndexError):
                yield node.new_child(node.value[value], norm_index)

        if isinstance(value, str) and isinstance(node.value, Mapping):
            with suppress(KeyError):
                yield node.new_child(node.value[value], value)

    def as_tuple(self) -> SelectorTuple | str:
        """Return this selector as a tuple of strings and/or nested tuples."""
        return self.query.as_tuple()
