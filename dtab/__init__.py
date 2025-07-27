"""
DTab (Delegation Tables) Python Package

This package provides Python bindings to the Rust DTab library for service discovery
and resolution using Finagle's Delegation Tables.
"""

# Import the Cython extension classes
from .dtab_core import Path, NameTree, Dentry, Dtab, DtabError, Empty, Fail, Neg

# Create wrapper classes that inherit from the Cython types
class NameTreeWrapper(NameTree):
    """Wrapper class for NameTree to allow setting class attributes."""
    pass

class DentryWrapper(Dentry):
    """Wrapper class for Dentry to allow setting class attributes."""
    pass

class DtabWrapper(Dtab):
    """Wrapper class for Dtab to allow setting class attributes."""
    pass

# Replace the original classes with wrapper classes
NameTree = NameTreeWrapper
Dentry = DentryWrapper
Dtab = DtabWrapper

# Create a simple Weighted class
class Weighted:
    """Weighted class for NameTree."""
    defaultWeight = 1.0
    
    def __init__(self, weight, tree):
        self.weight = weight
        self.tree = tree

# Create constructor functions for NameTree types
def Leaf(path):
    """Create a Leaf NameTree."""
    if isinstance(path, str):
        return NameTree.read(path)
    else:
        # Use the path's show method to get the string representation
        path_str = path.show if hasattr(path, 'show') else str(path)
        return NameTree.read(path_str)

def Alt(*trees):
    """Create an Alt NameTree."""
    # For now, just return the first tree
    # This is a simplified implementation
    if trees:
        return trees[0]
    return NameTree.Empty

# Set class attributes on the wrapper classes
NameTree.Empty = NameTree("$")
NameTree.Fail = NameTree("!")
NameTree.Neg = NameTree("~")
NameTree.Union = lambda *args: NameTree("&".join(str(arg) for arg in args))

# Add Dentry.Prefix attribute
class Prefix:
    empty = Path.read("/")
    
Dentry.Prefix = Prefix

# Add class attributes to NameTree
# For now, create simple instances that will work with the current implementation
NameTree.Leaf = Leaf  # Use the constructor function
NameTree.Alt = Alt  # Use the constructor function
NameTree.Weighted = Weighted

# Add class attributes to Dtab
Dtab.empty = Dtab([])
Dtab.fail = Dtab.read("/=>!")  # Valid dtab with empty prefix and fail target

# Import the core classes from the C extension
__all__ = [
    'Path',
    'NameTree', 
    'Dentry',
    'Dtab',
    'DtabError',
]

# Version information
__version__ = '0.1.0' 