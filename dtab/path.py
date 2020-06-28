from dtab.tree import Leaf
from dtab.util import u

__all__ = ["Path"]


class PathBase(type):
    def read(cls, s):
        from dtab.parser import NameTreeParsers

        return NameTreeParsers.parsePath(s)

    @property
    def empty(cls):
        if not hasattr(cls, "_empty"):
            cls._empty = Path()
        return cls._empty

    @property
    def showable_chars(cls):
        return "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_:.#$%-"


class Path(PathBase("PathBase", (object,), {})):
    @classmethod
    def Utf8(cls, *elems):
        return cls(*[u(elem) for elem in elems])

    def __init__(self, *elems):
        self._elems = []
        for e in elems:
            self.append(e)

    def append(self, value):
        if isinstance(value, Path):
            self._elems.extend(value.elems)
        elif isinstance(value, Leaf):
            self.append(value.value)
        else:
            self._elems.append(value)

    @property
    def elems(self):
        return self._elems

    def startswith(self, other):
        return self.show.startswith(other.show)

    @property
    def size(self):
        return len(self.elems)

    @property
    def is_empty(self):
        return self.size == 0

    @property
    def show(self):
        return "" if self.is_empty else "/" + "/".join(self.elems)

    def __eq__(self, other):
        return str(self) == str(other)

    def __ne__(self, other):
        return not self.__eq__(other)

    def __str__(self):
        return "Path({})".format(self.show)

    def __add__(self, other):
        if isinstance(other, self.__class__):
            args = self.elems + other.elems
            return self.__class__(*args)
        if isinstance(other, (list, tuple)):
            args = self.elems + list(other)
            return self.__class__.Utf8(*args)
        return object.__add__(self, other)

    @classmethod
    def is_showable(cls, char):
        if isinstance(char, int):
            char = chr(char)
        return all(c in cls.showable_chars for c in char)
