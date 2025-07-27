"""
Error module for DTab.
"""

from .dtab_core import DtabError


class IllegalArgumentException(DtabError):
    """Exception raised for illegal arguments."""
    pass 