"""Liquid specific Exceptions and warnings."""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from _liquid2 import TokenT


class Error(Exception):
    """Base class for all Liquid exceptions."""

    def __init__(
        self,
        *args: object,
        token: TokenT | None,
        filename: str | Path | None = None,
        source: str | None = None,
    ):
        super().__init__(*args)
        self.token = token
        self.filename = filename
        self.source = source

    def __str__(self) -> str:
        # TODO:
        return super().__str__()

    @property
    def message(self) -> object:
        """The exception's error message if one was given."""
        # TODO
        if self.args:
            return self.args[0]
        return None

    @property
    def name(self) -> str:
        """The name of the template that raised this exception.

        An empty string is return if a name is not available.
        """
        # TODO:
        if isinstance(self.filename, Path):
            return self.filename.as_posix()
        if self.filename:
            return str(self.filename)
        return ""


class LiquidInterrupt(Exception):  # noqa: N818
    """Loop interrupt exception."""


class StopRender(Exception):  # noqa: N818
    """Template inheritance interrupt.

    An interrupt used to signal that `BoundTemplate.render_with_context` should stop
    rendering more nodes. This is used by template inheritance tags and is not an error
    condition.
    """


class LiquidEnvironmentError(Error):
    """An exception raised due to a misconfigured environment."""


class LiquidSyntaxError(Error):
    """Exception raised when there is a parser error."""


class TemplateInheritanceError(Error):
    """An exceptions raised when template inheritance tags are used incorrectly.

    This could occur when parsing a template or at render time.
    """


class RequiredBlockError(TemplateInheritanceError):
    """An exception raised when a required block has not been overridden."""


class LiquidTypeError(Error):
    """Exception raised when an error occurs at render time."""


class DisabledTagError(Error):
    """Exception raised when an attempt is made to render a disabled tag."""


class NoSuchFilterFunc(Error):  # noqa: N818
    """Exception raised when a filter lookup fails."""


class FilterError(Error):
    """Exception raised when a filter fails."""

    def __init__(
        self,
        *args: object,
        filename: str | Path | None = None,
        source: str | None = None,
    ):
        super().__init__(*args, token=None, filename=filename, source=source)


class FilterArgumentError(FilterError):
    """Exception raised when a filter's arguments are invalid."""


class FilterValueError(FilterError):
    """Exception raised when a filters value is invalid."""


class TemplateNotFound(Error):  # noqa: N818
    """Exception raised when a template could not be found."""

    def __init__(
        self,
        *args: object,
        filename: str | Path | None = None,
        source: str | None = None,
    ):
        super().__init__(*args, token=None, filename=filename, source=source)

    def __str__(self) -> str:
        msg = super().__str__()
        return f"template not found {msg}"


class ResourceLimitError(Error):
    """Base class for exceptions relating to resource limits."""


class ContextDepthError(ResourceLimitError):
    """Exception raised when the maximum context depth is reached.

    Usually indicates recursive use of `render` or `include` tags.
    """


class LoopIterationLimitError(ResourceLimitError):
    """Exception raised when the loop iteration limit has been exceeded."""


class OutputStreamLimitError(ResourceLimitError):
    """Exception raised when an output stream limit has been exceeded."""


class LocalNamespaceLimitError(ResourceLimitError):
    """Exception raised when a local namespace limit has been exceeded."""


# LiquidValueError inheriting from LiquidSyntaxError does not make complete sense.
# The alternative is to have multiple to_int functions that raise more appropriate
# exceptions depending on whether we are parsing or rendering when attempting to
# convert long strings to integers.


class LiquidValueError(LiquidSyntaxError):
    """Exception raised when a cast from str to int exceeds the length limit."""


class UndefinedError(Error):
    """Exception raised by the StrictUndefined type."""


class BreakLoop(LiquidInterrupt):
    """Exception raised when a BreakNode is rendered."""


class ContinueLoop(LiquidInterrupt):
    """Exception raised when a ContinueNode is rendered."""


class TemplateTraversalError(Error):
    """Exception raised when an AST node or expression can not be visited."""


class LiquidWarning(UserWarning):
    """Base warning."""


class LiquidSyntaxWarning(LiquidWarning):
    """Replaces LiquidSyntaxError when in WARN mode."""


class LiquidTypeWarning(LiquidWarning):
    """Replaces LiquidTypeError when in WARN mode."""


class FilterWarning(LiquidWarning):
    """Replaces filter exceptions when in WARN mode."""


class CacheCapacityValueError(ValueError):
    """An exception raised when the LRU cache is given a zero or negative capacity."""
