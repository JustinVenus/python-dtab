"""
Name module for DTab.
"""

from .dtab_core import Path


class Name:
    """Name class for DTab."""
    
    @staticmethod
    def Path(path):
        """Create a path name."""
        if isinstance(path, str):
            return Path.read(path)
        return path 