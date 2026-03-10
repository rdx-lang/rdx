"""RDX (Reactive Document eXpressions) parser for Python.

Parse .rdx documents at Rust speed. Returns plain Python dicts.

    >>> import rdx_parser
    >>> ast = rdx_parser.parse("# Hello\\n\\n<Notice type=\\"info\\">World</Notice>\\n")
    >>> ast["type"]
    'root'
    >>> ast["children"][1]["name"]
    'Notice'
"""

from ._rdx import (
    parse,
    parse_with_defaults,
    parse_with_transforms,
    validate,
    collect_text,
    query_all,
    version,
)

__all__ = [
    "parse",
    "parse_with_defaults",
    "parse_with_transforms",
    "validate",
    "collect_text",
    "query_all",
    "version",
]
