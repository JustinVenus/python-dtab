import string

from dtab.error import IllegalArgumentException
from dtab.path import Path
from dtab.tree import NameTree
from dtab.util import PY3, u

if PY3:
    from io import StringIO as BytesIO
else:
    from io import BytesIO

__all__ = ["NameTreeParsers"]

EOI = 2 ** 8 - 1
FORWARD_SLASH = 92
WHITESPACE = list(map(ord, string.whitespace))
WILDCARD = 42


def to_ordinal(func):
    def inner(self, char):
        if not isinstance(char, int):
            char = ord(char)
        return func(self, char)

    if hasattr(func, "__doc__"):
        inner.__doc__ = func.__doc__
    # extract the inner name, so we
    # can read a stacktrace reasonably
    if PY3:
        inner.__name__ = func.__name__
    else:
        inner.func_name = func.func_name
    return inner


class NameTreeParsers(object):
    @classmethod
    def parsePath(cls, path):
        return cls(path).parse_all_path()

    @classmethod
    def parseNameTree(cls, name):
        return cls(name).parse_all_name_tree()

    @classmethod
    def parseDentry(cls, data):
        return cls(data).parse_all_dentry()

    @classmethod
    def parseDentryPrefix(cls, data):
        return cls(data).parse_all_dentry_prefix()

    @classmethod
    def parseDtab(cls, dtab):
        return cls(dtab).parse_all_dtab()

    def __init__(self, str_input):
        # avoiding circular dependencies
        from dtab import Dentry, Dtab

        self._dentry_cls = Dentry
        self._dtab_cls = Dtab
        self._str_input = u(str_input)
        self._index = 0

    def illegal(self, expected, found):
        if isinstance(expected, int):
            expected = self.string_of_char(expected)
        if isinstance(found, int):
            found = self.string_of_char(found)
        if self.at_end:
            display = "{}[]".format(self.string)
        else:
            display = "{}[{}]{}".format(
                self.string[0 : self.index],
                self.string[self.index],
                self.string[self.index + 1 :],
            )
        raise IllegalArgumentException(
            "{} expected but {} found at '{}'".format(expected, found, display)
        )

    def string_of_char(self, char):
        if char == EOI:
            return "end of input"
        return "'{}'".format(chr(char))

    @property
    def string(self):
        return self._str_input

    @property
    def ord(self):
        return ord(self.chr)

    @property
    def chr(self):
        return self.string[self.index]

    @property
    def index(self):
        return self._index

    @property
    def at_end(self):
        return self.index >= self.size

    @property
    def size(self):
        return len(self.string)

    @property
    def peek(self):
        if self.at_end:
            return EOI
        return self.ord

    def next(self):
        self._index += 1

    @to_ordinal
    def maybe_eat(self, char):
        if self.peek != char:
            return False
        self.next()
        return True

    @to_ordinal
    def eat(self, char):
        if not self.maybe_eat(char):
            self.illegal(char, self.peek)

    def eat_whitespace(self):
        while not self.at_end and (self.ord in WHITESPACE or self.chr == u("#")):
            if self.chr == u("#"):
                self.eat_line()
            else:
                self.next()

    def eat_line(self):
        while not self.at_end and self.chr != u("\n"):
            self.next()
        if not self.at_end:
            self.eat("\n")

    def ensure_end(self):
        if not self.at_end:
            self.illegal(EOI, self.peek)

    def parse_hex_char(self):
        c = chr(self.peek)
        try:
            int(c, 16)
            self.next()
            return ord(c)
        except ValueError:
            self.illegal("hex char", c)

    @to_ordinal
    def is_label_char(self, char):
        return Path.is_showable(char) or char == FORWARD_SLASH

    def parse_label(self):
        bio = BytesIO()
        while True:
            c = self.peek
            if Path.is_showable(c):
                self.next()
                bio.write(chr(c))
            elif c == FORWARD_SLASH:
                self.next()
                self.eat(u("x"))
                fst = self.parse_hex_char()
                snd = self.parse_hex_char()
                bio.write(chr(int(fst, 16) << 4 | int(snd, 16)))
            else:
                self.illegal("label char", c)
            if not self.is_label_char(self.peek):
                break
        if PY3:
            return bio.getvalue()
        else:
            return bio.getvalue().decode("utf-8")

    @to_ordinal
    def is_dentry_prefix_elem_char(self, char):
        return self.is_label_char(char) or char == WILDCARD

    def parse_dentry_prefix_elem(self):
        if self.peek == WILDCARD:
            self.next()
            return self._dentry_cls.Prefix.AnyElem
        return self._dentry_cls.Prefix.Label(self.parse_label())

    @to_ordinal
    def is_number_char(self, char):
        char = chr(char)
        return char.isdigit() or char == u(".")

    def parse_number(self):
        result = ""
        seendot = False
        while self.is_number_char(self.peek):
            if self.peek == ord("."):
                seendot = (
                    True if not seendot else self.illegal("number char", self.peek)
                )
            result += chr(self.peek)
            self.next()
        if len(result) == 1 and result[0] == u("."):
            self.illegal("weight", ".")
        return float(result)

    def parse_dentry_prefix(self):
        self.eat_whitespace()
        self.eat("/")

        if not self.is_dentry_prefix_elem_char(self.peek):
            return self._dentry_cls.Prefix.empty
        elems = []
        while True:
            elems.append(self.parse_dentry_prefix_elem())
            if not self.maybe_eat("/"):
                break
        return self._dentry_cls.Prefix(*elems)

    def parse_path(self):
        self.eat_whitespace()
        self.eat("/")
        if not self.is_label_char(self.peek):
            return Path.empty
        labels = []
        while True:
            labels.append(self.parse_label())
            if not self.maybe_eat("/"):
                break
        return Path(*labels)

    def parse_tree(self):
        trees = []
        while True:
            trees.append(self.parse_tree1())
            self.eat_whitespace()
            if not self.maybe_eat("|"):
                break

        if len(trees) > 1:
            return NameTree.Alt(*trees)
        return trees[0]

    def parse_tree1(self):
        trees = []
        while True:
            trees.append(self.parse_weighted())
            self.eat_whitespace()
            if not self.maybe_eat("&"):
                break
        if len(trees) > 1:
            return NameTree.Union(*trees)
        return trees[0].tree

    def parse_simple(self):
        self.eat_whitespace()
        c = chr(self.peek)

        if c == u("("):
            self.next()
            tree = self.parse_tree()
            self.eat_whitespace()
            self.eat(")")
            return tree

        if c == u("/"):
            return NameTree.Leaf(self.parse_path())

        if c == u("!"):
            self.next()
            return NameTree.Fail

        if c == u("~"):
            self.next()
            return NameTree.Neg

        if c == u("$"):
            self.next()
            return NameTree.Empty

        self.illegal("simple", c)

    def parse_weighted(self):
        self.eat_whitespace()
        weight = None
        if not self.is_number_char(self.peek):
            weight = NameTree.Weighted.defaultWeight
        else:
            weight = self.parse_number()
            self.eat_whitespace()
            self.eat("*")
            self.eat_whitespace()
        return NameTree.Weighted(weight, self.parse_simple())

    def parse_dentry(self):
        prefix = self.parse_dentry_prefix()
        self.eat_whitespace()
        self.eat("=")
        self.eat(">")
        tree = self.parse_tree()
        return self._dentry_cls(prefix, tree)

    def parse_dtab(self):
        dentries = []
        while True:
            self.eat_whitespace()
            if not self.at_end:
                dentries.append(self.parse_dentry())
                self.eat_whitespace()
            if not self.maybe_eat(";"):
                break
        return self._dtab_cls(dentries)

    def __parse_all(self, parsed):
        self.eat_whitespace()
        self.ensure_end()
        return parsed

    def parse_all_path(self):
        return self.__parse_all(self.parse_path())

    def parse_all_name_tree(self):
        return self.__parse_all(self.parse_tree())

    def parse_all_dentry(self):
        return self.__parse_all(self.parse_dentry())

    def parse_all_dentry_prefix(self):
        if self.size == 0:
            return self._dentry_cls.Prefix.empty
        return self.__parse_all(self.parse_dentry_prefix())

    def parse_all_dtab(self):
        if self.size == 0:
            return self._dtab_cls.empty
        return self.__parse_all(self.parse_dtab())
