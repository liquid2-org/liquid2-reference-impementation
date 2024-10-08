"""The built in, standard implementation of the comment node."""

from __future__ import annotations

from typing import TYPE_CHECKING
from typing import TextIO

from liquid2 import Markup
from liquid2 import Node
from liquid2.tag import Tag

if TYPE_CHECKING:
    from liquid2 import TokenT
    from liquid2.ast import MetaNode
    from liquid2.context import RenderContext
    from liquid2.tokens import TokenStream


class CommentNode(Node):
    """The built in, standard implementation of the comment node."""

    __slots__ = ("text",)

    def __init__(self, token: TokenT, text: str) -> None:
        super().__init__(token)
        self.text = text

    def __str__(self) -> str:
        return self.text

    def render_to_output(self, _context: RenderContext, _buffer: TextIO) -> int:
        """Render the node to the output buffer."""
        return 0

    def children(self) -> list[MetaNode]:
        """Return a list of child nodes and/or expressions associated with this node."""
        return []


class Comment(Tag):
    """The built in pseudo tag representing template comments."""

    block = False
    node_class = CommentNode

    def parse(self, stream: TokenStream) -> Node:
        """Parse tokens from _stream_ into an AST node."""
        token = stream.current()
        assert isinstance(token, Markup.Comment)
        return self.node_class(token, token.text)
