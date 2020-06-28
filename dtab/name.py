from dtab.path import Path


class Address(object):
    @property
    def pending(self):
        raise NotImplementedError()


class Bound(object):
    def __init__(self, address):
        self.address = address


class NameBase(type):
    Path = property(lambda _: Path)
    Bound = property(lambda _: Bound)


class Name(NameBase("NameBase", (object,), {})):
    pass


__all__ = ["Name"]
