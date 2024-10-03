"""The standard _for_ tag."""

from __future__ import annotations

from typing import TYPE_CHECKING
from typing import Any
from typing import Iterator
from typing import Mapping
from typing import TextIO

from liquid2 import Markup
from liquid2 import Node
from liquid2.ast import BlockNode
from liquid2.builtin import LoopExpression
from liquid2.context import RenderContext
from liquid2.exceptions import BreakLoop
from liquid2.exceptions import ContinueLoop
from liquid2.tag import Tag
from liquid2.tokens import TokenStream

if TYPE_CHECKING:
    from liquid2 import TokenT
    from liquid2.context import RenderContext


class ForNode(Node):
    """The standard _for_ tag."""

    __slots__ = ("expression", "block", "default")

    def __init__(
        self,
        token: TokenT,
        expression: LoopExpression,
        block: BlockNode,
        default: BlockNode | None,
    ) -> None:
        super().__init__(token)
        self.expression = expression
        self.block = block
        self.default = default

    def __str__(self) -> str:
        tag_str = f"for ({self.expression}) {{ {self.block} }}"

        if self.default:
            tag_str += f" else {{ {self.default} }}"

        return tag_str

    def render_to_output(self, context: RenderContext, buffer: TextIO) -> int:
        """Render the node to the output buffer."""
        it, length = self.expression.evaluate(context)

        if length:
            character_count = 0
            name = self.expression.identifier
            token = self.expression.token

            forloop = ForLoop(
                name=f"{name}-{self.expression.iterable}",
                it=it,
                length=length,
                parentloop=context.parentloop(token),
            )

            namespace = {
                "forloop": forloop,
                name: None,
            }

            # Extend the context. Essentially giving priority to `ForLoopDrop`, then
            # delegating `get` and `assign` to the outer context.
            with context.loop(namespace, forloop):
                for itm in forloop:
                    namespace[name] = itm
                    try:
                        character_count += self.block.render(context, buffer)
                    except ContinueLoop:
                        continue
                    except BreakLoop:
                        break

            return character_count

        return self.default.render(context, buffer) if self.default else 0

    async def render_to_output_async(
        self, context: RenderContext, buffer: TextIO
    ) -> int:
        """Render the node to the output buffer."""
        it, length = await self.expression.evaluate_async(context)

        if length:
            character_count = 0
            name = self.expression.identifier
            token = self.expression.token

            forloop = ForLoop(
                name=f"{name}-{self.expression.iterable}",
                it=it,
                length=length,
                parentloop=context.parentloop(token),
            )

            namespace = {
                "forloop": forloop,
                name: None,
            }

            # Extend the context. Essentially giving priority to `ForLoopDrop`, then
            # delegating `get` and `assign` to the outer context.
            with context.loop(namespace, forloop):
                for itm in forloop:
                    namespace[name] = itm
                    try:
                        character_count += await self.block.render_async(
                            context, buffer
                        )
                    except ContinueLoop:
                        continue
                    except BreakLoop:
                        break

            return character_count

        return await self.default.render_async(context, buffer) if self.default else 0


class ForTag(Tag):
    """The standard _for_ tag."""

    block = True
    node_class = ForNode
    end_block = frozenset(["endfor", "else"])

    def parse(self, stream: TokenStream) -> Node:
        """Parse tokens from _stream_ into an AST node."""
        token = next(stream)
        assert isinstance(token, Markup.Tag)
        expression = LoopExpression.parse(TokenStream(token.expression))

        parse_block = self.env.parser.parse_block

        block_token = stream.current()
        assert block_token is not None
        block = BlockNode(block_token, parse_block(stream, end=self.end_block))

        default: BlockNode | None = None

        if stream.is_tag("else"):
            next(stream)
            default_token = stream.current()
            assert default_token is not None
            default_block = parse_block(stream, self.end_block)
            default = BlockNode(default_token, default_block)

        return self.node_class(
            token,
            expression,
            block,
            default,
        )


class ForLoop(Mapping[str, object]):
    """Loop helper variables."""

    __slots__ = (
        "name",
        "it",
        "length",
        "item",
        "_index",
        "parentloop",
    )

    _keys = frozenset(
        [
            "name",
            "length",
            "index",
            "index0",
            "rindex",
            "rindex0",
            "first",
            "last",
            "parentloop",
        ]
    )

    def __init__(
        self,
        name: str,
        it: Iterator[object],
        length: int,
        parentloop: object,
    ):
        self.name = name
        self.it = it
        self.length = length

        self.item = None
        self._index = -1  # Step is called before `next(it)`
        self.parentloop = parentloop

    def __repr__(self) -> str:  # pragma: no cover
        return f"ForLoop(name='{self.name}', length={self.length})"

    def __getitem__(self, key: str) -> object:
        if key in self._keys:
            return getattr(self, key)
        raise KeyError(key)

    def __len__(self) -> int:
        return len(self._keys)

    def __next__(self) -> object:
        self.step()
        return next(self.it)

    def __iter__(self) -> Iterator[Any]:
        return self

    def __str__(self) -> str:
        return "ForLoop"

    @property
    def index(self) -> int:
        """The 1-based index of the current loop iteration."""
        return self._index + 1

    @property
    def index0(self) -> int:
        """The 0-based index of the current loop iteration."""
        return self._index

    @property
    def rindex(self) -> int:
        """The 1-based index, counting from the right, of the current loop iteration."""
        return self.length - self._index

    @property
    def rindex0(self) -> int:
        """The 0-based index, counting from the right, of the current loop iteration."""
        return self.length - self._index - 1

    @property
    def first(self) -> bool:
        """True if this is the first iteration, false otherwise."""
        return self._index == 0

    @property
    def last(self) -> bool:
        """True if this is the last iteration, false otherwise."""
        return self._index == self.length - 1

    def step(self) -> None:
        """Move the for loop helper forward to the next iteration."""
        self._index += 1


class BreakNode(Node):
    """Parse tree node for the standard _break_ tag."""

    __slots__ = ()

    def __str__(self) -> str:
        return "`break`"

    def render_to_output(self, _context: RenderContext, _buffer: TextIO) -> int:
        """Render the node to the output buffer."""
        raise BreakLoop("break")


class ContinueNode(Node):
    """Parse tree node for the standard _continue_ tag."""

    def __str__(self) -> str:
        return "`continue`"

    def render_to_output(self, _context: RenderContext, _buffer: TextIO) -> int:
        """Render the node to the output buffer."""
        raise ContinueLoop("continue")


class BreakTag(Tag):
    """The built-in "break" tag."""

    block = False

    def parse(self, stream: TokenStream) -> BreakNode:
        """Parse tokens from _stream_ into an AST node."""
        return BreakNode(stream.current())  # type: ignore


class ContinueTag(Tag):
    """The built-in "continue" tag."""

    block = False

    def parse(self, stream: TokenStream) -> ContinueNode:
        """Parse tokens from _stream_ into an AST node."""
        return ContinueNode(stream.current())  # type: ignore
