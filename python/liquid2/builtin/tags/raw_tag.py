"""The built in, standard implementation of the _raw_ tag."""

from __future__ import annotations

from typing import TYPE_CHECKING
from typing import TextIO

from liquid2 import Markup
from liquid2 import Node
from liquid2.tag import Tag

if TYPE_CHECKING:
    from liquid2 import TokenT
    from liquid2.context import RenderContext
    from liquid2.tokens import TokenStream


class RawNode(Node):
    """The built in, standard implementation of the _raw_ node."""

    __slots__ = ("text",)

    def __init__(self, token: TokenT, text: str) -> None:
        super().__init__(token)
        self.text = text

    def __str__(self) -> str:
        return self.text

    def render_to_output(self, _context: RenderContext, buffer: TextIO) -> int:
        """Render the node to the output buffer."""
        return buffer.write(self.text)


class RawTag(Tag):
    """The standard _raw_ tag."""

    block = False
    node_class = RawNode

    def parse(self, stream: TokenStream) -> Node:
        """Parse tokens from _stream_ into an AST node."""
        token = stream.current()
        assert isinstance(token, Markup.Raw)
        return self.node_class(token, token.text)
