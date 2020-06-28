class NameTreeBase(type):
    Alt = property(lambda _: Alt)
    Empty = property(lambda _: Empty)
    Fail = property(lambda _: Fail)
    Leaf = property(lambda _: Leaf)
    Neg = property(lambda _: Neg)
    Union = property(lambda _: Union)
    Weighted = property(lambda _: Weighted)

    @property
    def unionFail(cls):
        return [cls.Weighted(cls.Weighted.defaultWeight, cls.Fail)]

    def read(s):
        from dtab.parser import NameTreeParsers

        return NameTreeParsers.parseNameTree(s)

    def map_tree(cls, tree, func):
        if isinstance(tree, cls.Union):
            return Union(
                *[
                    Weighted(t.weight, t.map(func))
                    for t in tree
                    if isinstance(t, Weighted)
                ]
            )
        if isinstance(tree, cls.Alt):
            return Alt(*list(map(func, tree)))
        if isinstance(tree, cls.Leaf):
            return cls.Leaf(func(tree))
        if isinstance(tree, (cls.Fail, cls.Neg, cls.Empty)):
            return tree


class NameTree(NameTreeBase("NameTreeBase", (object,), {})):
    @property
    def show(self):
        raise NotImplementedError()

    def map(self, func):
        return self.__class__.map_tree(self, func)

    def __str__(self):
        return "NameTree.{}({})".format(self.__class__.__name__, self.show)

    def __eq__(self, other):
        if isinstance(other, self.__class__):
            return self.__str__() == other.__str__()
        raise NotImplementedError()

    def __ne__(self, other):
        return not self.__eq__(other)


class Alt(NameTree):
    def __init__(self, *trees):
        self._trees = []
        for tree in trees:
            if not isinstance(tree, NameTree):
                raise TypeError("{} is not a NameTree".format(tree))
            self._trees.append(tree)

    def __iter__(self):
        return iter(self._trees)

    @property
    def trees(self):
        return self._trees

    @property
    def show(self):
        return ",".join([t.__str__() for t in self.trees])

    def __len__(self):
        return len(self.trees)


class Empty(NameTree):
    @property
    def show(self):
        return "Empty"

    def __str__(self):
        return "NameTree.{}".format(self.show)

    def __eq__(self, other):
        return self is other


Empty = Empty()  # singleton


class Fail(NameTree):
    @property
    def show(self):
        return "Fail"

    def __str__(self):
        return "NameTree.{}".format(self.show)

    def __eq__(self, other):
        return self is other


Fail = Fail()  # singleton


class Leaf(NameTree):
    def __init__(self, value):
        if not hasattr(self.__class__, "__path"):
            from dtab.path import Path

            self.__class__.__path = Path

        if isinstance(value, self.__class__):
            self._value = value.value
        else:
            self._value = value

    @property
    def value(self):
        return self._value

    @property
    def show(self):
        if isinstance(self._value, self.__class__.__path):
            return self.value.__str__()
        return self.value

    def __eq__(self, other):
        if isinstance(other, Leaf):
            return self.value == other.value
        return self.value == other

    def __add__(self, other):
        if isinstance(other, self.__class__):
            return self.__class__(self.value + other.value)
        return self.__class__(self.value + other)


class Neg(NameTree):
    @property
    def show(self):
        return "Neg"

    def __str__(self):
        return "NameTree.{}".format(self.show)

    def __eq__(self, other):
        return self is other


Neg = Neg()  # singleton


class Union(NameTree):
    @classmethod
    def from_seq(cls, trees):
        return cls(trees)

    def __init__(self, *trees):
        self._trees = []
        for tree in trees:
            if not isinstance(tree, Weighted):
                raise TypeError("{} is not a Weighted Nametree".format(tree))
            self._trees.append(tree)

    def __iter__(self):
        return iter(self._trees)

    @property
    def trees(self):
        return self._trees

    @property
    def show(self):
        return ",".join([t.__str__() for t in self.trees])


class Weighted(NameTree):
    defaultWeight = 1

    def __init__(self, weight, tree):
        if not isinstance(tree, NameTree):
            raise TypeError("{} is not a Nametree".format(tree))
        self._tree = tree
        self._weight = float(weight)

    @property
    def tree(self):
        return self._tree

    @property
    def weight(self):
        return self._weight

    @property
    def show(self):
        return "{},{}".format(self.weight, self.tree)


__all__ = ["NameTree"]
