from logging import getLogger
from unittest import TestCase

from dtab import Dentry, Dtab
from dtab.name import Name
from dtab.path import Path
from dtab.tree import NameTree

log = getLogger(__name__)
# see http://twitter.github.io/finagle/guide/Names.html for behavior


class DtabTest(TestCase):
    def test_concat_d1_d2(self):
        d1 = Dtab.read("/foo => /bar")
        d2 = Dtab.read("/foo=>/biz;/biz=>/$/inet/0/8080;/bar=>/$/inet/0/9090")

        self.assertTrue(
            d1 + d2
            == Dtab.read(
                """
        /foo=>/bar;
        /foo=>/biz;
        /biz=>/$/inet/0/8080;
        /bar=>/$/inet/0/9090
    """
            )
        )

    def test_dtab_read_ignores_comment_line(self):
        withComments = Dtab.read(
            """
# a comment
      /#foo => /biz  # another comment
             | ( /bliz & # yet another comment
                 /bluth ) # duh bluths
             ; #finalmente
      #/ignore=>/me;
    """
        )
        dtab = Dtab(
            [
                Dentry(
                    Path.Utf8("#foo"),
                    NameTree.Alt(
                        NameTree.Leaf(Path.Utf8("biz")),
                        NameTree.Union(
                            NameTree.Weighted(
                                NameTree.Weighted.defaultWeight,
                                NameTree.Leaf(Path.Utf8("bliz")),
                            ),
                            NameTree.Weighted(
                                NameTree.Weighted.defaultWeight,
                                NameTree.Leaf(Path.Utf8("bluth")),
                            ),
                        ),
                    ),
                )
            ]
        )

        s = "Dtab(Label(#foo)=>NameTree.Leaf(Path(/biz)),"
        s += "NameTree.Union(NameTree.Weighted(1.0,NameTree.Leaf(Path(/bliz))),"
        s += "NameTree.Weighted(1.0,NameTree.Leaf(Path(/bluth)))))"

        self.assertTrue(str(dtab) == s)

        self.assertTrue(withComments == dtab)

    def test_d1_concat_dtab_empty(self):
        d1 = Dtab.read("/foo=>/bar;/biz=>/baz")
        self.assertTrue(d1 + Dtab.empty == d1)

    def test_is_collection(self):
        # these are mostly just compilation tests.
        dtab = Dtab.empty
        dtab += Dentry.read("/a => /b")
        dtab += Dentry.read("/c => /d")

        # FIXME
        # dtab1 = Dtab(
        #    map(lambda d: Dentry.read(
        #        "{}=>{}".format(d.prefix.show.upper(), d.nametree.show.upper())), dtab))
        #
        # self.assertTrue(dtab1.size, 2)

    def test_allows_trailing_semicolon(self):
        dtab = Dtab.read(
            """
      /b => /c;
      /a => /b;
    """
        )
        self.assertTrue(dtab.length == 2)

    def test_dtab_rewrites_with_wildcard(self):
        dtab = Dtab.read("/a/*/c => /d")

        nametree = dtab.lookup(Path.read("/a/b/c/e/f"))
        leaf = NameTree.Leaf(Name.Path(Path.read("/d/e/f")))
        self.assertTrue(nametree == leaf)
