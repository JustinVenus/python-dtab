from dtab.parser import NameTreeParsers
from dtab.name import Name
from dtab.path import Path
from dtab.tree import NameTree

__all__ = ['Dentry', 'Dtab']


class DtabBase(type):

  @property
  def fail(cls):
    """A failing delegation table."""
    if not hasattr(cls, '_fail'):
      cls._fail = cls.read('/=>!')
    return cls._fail

  @property
  def empty(cls):
    """An empty delegation table"""
    if not hasattr(cls, '_empty'):
      cls._empty = cls([])
    return cls._empty

  @property
  def base(cls):
    """The base, or "system", or "global", delegation table applies to
       every request in this process.  It is generally set at process
       startup, and not changed thereafter."""
    if not hasattr(cls, '_base'):
      cls._base = cls.empty
    return cls._base

  @base.setter
  def base(cls, value):
    if isinstance(value, cls):
      cls._base = value
      return cls._base
    raise TypeError("{} is not derived from {}".format(value, cls.__name__))


class Dtab(DtabBase('DtabBase', (object,), {})):
  """A Dtab (short for delegation table) comprises a sequence of
     delegation rules.  Together, these describe how to bind a
     dtab.path.Path to a set of
     dtab.address.Addr.  dtab.naming.DefaultInterpreter
     implements the default binding strategy.
  """

  @classmethod
  def read(cls, s):
    """Parse a Dtab from string `s` whit concrete syntax

       {{{
       dtab	::= dentry ';' dtab | dentry
       }}}

       where the production `dentry` is from the grammar documented in
       dtab.Dentry.read
    """
    return NameTreeParsers.parseDtab(s)

  def __init__(self, delegation_table):
    self._dentries = []
    for args in delegation_table:
      if isinstance(args, Dentry):
        self._dentries.append(args)
      elif isinstance(args, list):
        self._dentries.append(Dentry(*args))
      else:
        raise TypeError("Input must be coercible to a  Dentry")
    self._public = [d for d in self._dentries]
    self._dentries.reverse()  # must invert the List[Dentry] for lookup

  @property
  def dentries(self):
    """List[Dentry] provided to the Dtab's constructor"""
    return self._public

  @property
  def length(self):
    """Represents the number of Dentry instances mapped"""
    return len(self._public)

  @property
  def is_empty(self):
    return self.length == 0

  def lookup(self, path):
    """Lookup the given `path` with this dtab"""
    matches = []
    # don't use public dentries
    for dentry in self._dentries:
      if dentry.prefix.matches(path):
        suffix = path.elems[dentry.prefix.size:]
        matches.append(dentry.nametree.map(
            lambda pfx: Name.Path(pfx + suffix)))
    if not len(matches):
      return NameTree.Neg
    elif len(matches) == 1:
      return matches[0]
    return NameTree.Alt(*matches)

  def __iter__(self):
    return iter(self._dentries)

  def __add__(self, other):
    if isinstance(other, Dentry):
      return self.copy(dentry=other)
    elif isinstance(other, Dtab) and not other.is_empty:
      elems = other.dentries
      while True:
        instance = self.copy(dentry=elems.pop(0))
        if not len(elems):
          return instance
    elif isinstance(other, Dtab) and other.is_empty:
      return self
    elif isinstance(other, Dtab) and self.is_empty:
      return other
    raise TypeError("unsupported operand type(s) for +: '{}' and '{}'".format(
        type(self).__name__, type(other).__name__))

  def __eq__(self, other):
    return self.__str__() == other.__str__()

  def __ne__(self, other):
    return not self.__eq__(other)

  def copy(self, dentry=None):
    """Constructs a new Dtab with `dentry` appended if provided"""
    dentries = self.dentries
    if dentry is not None:
      dentries.append(dentry)
    return self.__class__(dentries)

  @property
  def show(self):
    return ";".join([d.show for d in self.dentries])

  def __str__(self):
    return "{}({})".format(self.__class__.__name__, self.show)

  def pretty_print(self):
    print("Dtab({})".format(self.length))
    for dentry in self.dentries:
      print(" {} => {}".format(dentry.prefix.show, dentry.nametree.__str__()))


class DentryBase(type):
  Prefix = property(lambda _: Prefix)

  @property
  def nop(cls):
    if not hasattr(cls, '_nop'):
      # The prefix to this is an illegal path in the sense that the
      # concrete syntax will not admit it.  It will do for a no-op.
      cls._nop = Dentry(cls.Prefix(cls.Prefix.Label("/")), NameTree.Neg)
    return cls._nop

  def __call__(cls, prefix, dst):
    if isinstance(prefix, Path):
      prefix = cls.Prefix(*prefix.elems)
    return type.__call__(cls, prefix, dst)


class Dentry(DentryBase('DentryBase', (object,), {})):
  """Dentry describes a delegation table entry."""

  @classmethod
  def read(cls, s):
    """Parse a Dentry from the string `s` with concrete syntax:
       {{{
       dentry	::= prefix '=>' tree
       }}}

       where the production `prefix` is from the grammar documented in
       Prefix.read and the production `tree` is from the grammar
       documented in dtab.tree.NameTree.read.
    """
    return NameTreeParsers.parseDentry(s)

  def __init__(self, prefix, nametree):
    """`prefix` describes the paths that the entry applies to.
       `nametree` describes the resulting tree for this prefix on lookup."""
    if not isinstance(prefix, self.__class__.Prefix):
      raise TypeError("'{}' is not derived from {}.{}".format(
          prefix, self.__class__.__name__, self.__class__.Prefix.__name__))
    if not isinstance(nametree, NameTree):
      raise TypeError("'{}' is not derived from {}".format(nametree, type(NameTree)))
    self._prefix = prefix
    self._nametree = nametree

  @property
  def nametree(self):
    return self._nametree

  @property
  def prefix(self):
    return self._prefix

  def __eq__(self, other):
    return ((other.prefix.show == self.prefix.show) and
            (other.nametree.show == self.nametree.show))

  @property
  def show(self):
    return '{}=>{}'.format(self.prefix.show, self.nametree.show)

  def __str__(self):
    return 'Dentry({})'.format(self.show)


class Elem(object):

  def __ne__(self, other):
    return not self.__eq__(other)


class AnyElem(Elem):
  show = property(lambda _: "*")

  def __str__(self):
    return "AnyElem"

  def __eq__(self, other):
    return True

AnyElem = AnyElem()  # singleton


class Label(Elem):

  def __init__(self, buf):
    if not buf:
      raise ValueError("Input is empty")
    self._buf = buf

  @property
  def buf(self):
    return self._buf

  @property
  def show(self):
    return Path.show_elem(self.buf)

  def __str__(self):
    return "Label({})".format(self.buf)

  def __eq__(self, other):
    if isinstance(other, Label):
      return self.buf == other.buf
    return other == self.buf


class PrefixBase(type):
  AnyElem = property(lambda _: AnyElem)
  Elem = property(lambda _: Elem)
  Label = property(lambda _: Label)

  @property
  def empty(cls):
    if not hasattr(cls, '_empty'):
      cls._empty = Prefix()
    return cls._empty


class Prefix(PrefixBase('PrefixBase', (object,), {})):

  @classmethod
  def read(cls, s):
    """Parses `s` as a prefix matching expression with concrete syntax

       {{{
       path	::= '/' elems | '/'

       elems	::= elem '/' elem | elem

       elem	::= '*' | label

       label	::= (\\x[a-f0-9][a-f0-9]|[0-9A-Za-z:.#$%-_])+

       }}}

       for example

       {{{
       /foo/bar/baz
       /foo/&#47;*&#47;bar/baz
       /
       }}}

       parses into the path

       {{{
       Prefix(Label(foo),Label(bar),Label(baz))
       Prefix(Label(foo),AnyElem,Label(bar),Label(baz))
       Prefix()
       }}}
    """
    return NameTreeParsers.parseDentryPrefix(s)

  def __init__(self, *elems):
    self._elems = []
    for e in elems:
      if isinstance(e, Elem):
        self._elems.append(e)
        continue
      self._elems.append(Label(e))

  @property
  def size(self):
    return len(self.elems)

  @property
  def elems(self):
    return self._elems

  def matches(self, path):
    if self.size > path.size:
      return False
    i = 0
    while i != self.size:
      if self.elems[i] != path.elems[i]:
        return False
      i += 1
    return True

  @property
  def show(self):
    return ",".join([str(e) for e in self.elems])

  def __str__(self):
    return "Prefix({})".format(self.show)
