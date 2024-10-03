"""The standard _cycle_ tag."""

from __future__ import annotations

from typing import TYPE_CHECKING
from typing import TextIO

from liquid2 import Markup
from liquid2 import Node
from liquid2 import Token
from liquid2.builtin import parse_primitive
from liquid2.builtin import parse_string_or_identifier
from liquid2.context import RenderContext
from liquid2.exceptions import LiquidSyntaxError
from liquid2.stringify import to_liquid_string
from liquid2.tag import Tag
from liquid2.tokens import TokenStream

if TYPE_CHECKING:
    from liquid2 import TokenT
    from liquid2.context import RenderContext
    from liquid2.expression import Expression


class CycleNode(Node):
    """The standard _cycle_ tag."""

    __slots__ = ("name", "items", "cycle_hash")

    def __init__(
        self, token: TokenT, name: str | None, items: list[Expression]
    ) -> None:
        super().__init__(token)
        self.name = name
        self.items = tuple(items)
        self.cycle_hash = hash((self.name, self.items))

    def render_to_output(self, context: RenderContext, buffer: TextIO) -> int:
        """Render the node to the output buffer."""
        index = context.cycle(self.cycle_hash, len(self.items))
        return buffer.write(
            to_liquid_string(
                self.items[index].evaluate(context), auto_escape=context.auto_escape
            )
        )

    async def render_to_output_async(
        self, context: RenderContext, buffer: TextIO
    ) -> int:
        """Render the node to the output buffer."""
        index = context.cycle(self.cycle_hash, len(self.items))
        return buffer.write(
            to_liquid_string(
                await self.items[index].evaluate_async(context),
                auto_escape=context.auto_escape,
            )
        )


class CycleTag(Tag):
    """The standard _cycle_ tag."""

    block = False
    node_class = CycleNode

    def parse(self, stream: TokenStream) -> Node:
        """Parse tokens from _stream_ into an AST node."""
        token = stream.current()
        assert isinstance(token, Markup.Tag)

        if not token.expression:
            raise LiquidSyntaxError("expected a group name or item list", token=token)

        expr_stream = TokenStream(token.expression)

        # Does this cycle tag define a name followed by a colon, before listing
        # items to cycle through?
        if isinstance(expr_stream.peek(), Token.Colon):
            name: str | None = parse_string_or_identifier(next(expr_stream, None))
            expr_stream.expect(Token.Colon)
            expr_stream.next()
        else:
            name = None

        items: list[Expression] = []

        # We must have at least one item
        items.append(parse_primitive(expr_stream.next()))

        while True:
            item_token = expr_stream.next()

            match item_token:
                case None:
                    break
                case Token.Comma():
                    pass
                case _:
                    raise LiquidSyntaxError(
                        "expected a comma separated list, "
                        f"found '{item_token.__class__.__name__}'",
                        token=item_token,
                    )

            # Trailing commas are OK
            item_token = expr_stream.next()

            match item_token:
                case None:
                    break
                case _:
                    items.append(parse_primitive(item_token))

        return self.node_class(token, name, items)
