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

    def read(cls, s):
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
        if tree is cls.Fail or tree is cls.Neg or tree is cls.Empty:
            return tree

    def simplify(cls, tree):
        if isinstance(tree, cls.Alt) and not len(tree):
            return cls.Neg
        elif isinstance(tree, cls.Alt) and len(tree) == 1:
            return cls.simplify(tree[0])
        elif isinstance(tree, cls.Alt):
            trees, accum = list(tree.trees), []
            while True:  # avoid recursion
                if not trees:
                    break
                head = trees[:1][0]
                tail = trees[1:]
                r1 = cls.simplify(head)
                if r1 is cls.Fail:
                    accum.append(cls.Fail)
                    break
                elif r1 is cls.Neg:
                    trees = list(tail)
                    continue
                elif r1 == head:
                    trees = list(tail)
                    accum.append(head)
                    continue
            if not accum:
                return cls.Neg
            elif len(accum) == 1:
                return accum[0]
            return cls.Alt.fromSeq(accum)
        elif isinstance(tree, cls.Union) and not len(tree):
            return cls.Neg
        elif isinstance(tree, cls.Union) and len(tree) == 1:
            return cls.simplify(tree[0].tree)  # extract tree from Weighted
        elif isinstance(tree, cls.Union):
            trees, accum = list(tree.trees), []
            while True:  # avoid recursion
                if not trees:
                    break
                head = trees[:1][0]
                tail = trees[1:]
                r1 = cls.simplify(head.tree)
                if r1 is cls.Fail or r1 is cls.Neg:
                    trees = list(tail)
                    continue
                elif r1 == head.tree:
                    trees = list(tail)
                    accum.append(cls.Weighted(head.weight, head.tree))
                    continue
            if not accum:
                return cls.Neg
            elif len(accum) == 1:
                return accum[0].tree
            return cls.Union.fromSeq(accum)
        return tree


class NameTree(NameTreeBase("NameTreeBase", (object,), {})):
    __slots__ = tuple("_simplified")

    @property
    def simplified(self):
        result = getattr(self, "_simplified", None)
        if result is None:
            self._simplified = self.__class__.simplify(self)
            return self._simplified
        return result

    def eval(self):
        result = self.__class__.evaluate(self)
        if isinstance(result, (Fail, Neg)):
            return None
        elif isinstance(result, Leaf):
            return result.value
        raise RuntimeError("bug")

    @property
    def show(self):
        raise NotImplementedError()

    def map(self, func):
        return self.__class__.map_tree(self, func)

    def __str__(self):
        return "NameTree.{}({})".format(self.__class__.__name__, self.show)

    __repr__ = __str__

    def __eq__(self, other):
        if isinstance(other, self.__class__):
            return self.__str__() == other.__str__()
        raise NotImplementedError()

    def __ne__(self, other):
        return not self.__eq__(other)


class Alt(NameTree):
    __slots__ = ("_trees",)

    @classmethod
    def fromSeq(cls, seq):
        return cls(*seq)

    def __init__(self, *trees):
        self._trees = []
        for tree in trees:
            if not isinstance(tree, NameTree):
                raise TypeError("{} is not a NameTree".format(tree))
            self._trees.append(tree)

    def __iter__(self):
        return iter(self._trees)

    def __getitem__(self, idx):
        return self._trees[idx]

    @property
    def trees(self):
        return self._trees

    @property
    def show(self):
        return ",".join([t.__str__() for t in self.trees])

    def __len__(self):
        return len(self.trees)


class Empty(NameTree):
    __slots__ = tuple()

    @property
    def show(self):
        return "Empty"

    def __str__(self):
        return "NameTree.Empty"

    __repr__ = __str__

    def __eq__(self, other):
        return self is other


Empty = Empty()  # singleton


class Fail(NameTree):
    __slots__ = tuple()

    @property
    def show(self):
        return "Fail"

    def __str__(self):
        return "NameTree.Fail"

    __repr__ = __str__

    def __eq__(self, other):
        return self is other


Fail = Fail()  # singleton


class Leaf(NameTree):
    __slots__ = ("_value",)

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
    __slots__ = tuple()

    @property
    def show(self):
        return "Neg"

    def __str__(self):
        return "NameTree.Neg"

    __repr__ = __str__

    def __eq__(self, other):
        return self is other


Neg = Neg()  # singleton


class Union(NameTree):
    __slots__ = ("_trees",)

    @classmethod
    def fromSeq(cls, trees):
        return cls(*trees)

    def __init__(self, *trees):
        self._trees = []
        for tree in trees:
            if not isinstance(tree, Weighted):
                raise TypeError("{} is not a Weighted Nametree".format(tree))
            self._trees.append(tree)

    def __iter__(self):
        return iter(self._trees)

    def __getitem__(self, idx):
        return self._trees[idx]

    @property
    def trees(self):
        return self._trees

    @property
    def show(self):
        return ",".join([t.__str__() for t in self.trees])

    def __len__(self):
        return len(self.trees)


class classproperty(object):
    def __init__(self, f):
        self.f = f

    def __get__(self, obj, owner):
        return self.f(owner)


class Weighted(NameTree):
    defaultWeight = classproperty(lambda _: 1.0)
    __slots__ = ("_tree", "_weight")

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
