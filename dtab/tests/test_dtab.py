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

        self.assertEqual(
            d1 + d2,
            Dtab.read(
                """
        /foo=>/bar;
        /foo=>/biz;
        /biz=>/$/inet/0/8080;
        /bar=>/$/inet/0/9090
    """
            ),
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

        self.assertEqual(repr(dtab), s)

        self.assertEqual(withComments, dtab)

    def test_d1_concat_dtab_empty(self):
        d1 = Dtab.read("/foo=>/bar;/biz=>/baz")
        self.assertEqual(d1 + Dtab.empty, d1)

    def test_is_collection(self):
        # these are mostly just compilation tests.
        dtab = Dtab.empty
        dtab += Dentry.read("/a => /b")
        dtab += Dentry.read("/c => /d")

        dtab1 = Dtab(
            map(
                lambda d: Dentry.read(
                    "/{}=>{}".format(
                        "/".join(map(lambda x: x.buf, d.prefix.elems)),
                        d.nametree.value.show,
                    )
                ),
                dtab,
            )
        )

        self.assertTrue(dtab1.length, 2)

    def test_allows_trailing_semicolon(self):
        dtab = Dtab.read(
            """
      /b => /c;
      /a => /b;
    """
        )
        self.assertEqual(dtab.length, 2)

    def test_dtab_rewrites_with_wildcard(self):
        dtab = Dtab.read("/a/*/c => /d")

        nametree = dtab.lookup(Path.read("/a/b/c/e/f"))
        leaf = NameTree.Leaf(Name.Path(Path.read("/d/e/f")))
        self.assertEqual(nametree, leaf)

    def test_lookup_simple(self):
        dtab = Dtab.read(
            """/zk#    =>      /$/com.twitter.serverset;
                        /zk     =>      /zk#;
                        /s##    =>      /zk/zk.local.twitter.com:2181;
                        /s#     =>      /s##/prod;
                        /s      =>      /s#;"""
        )

        one = NameTree.read("/s/crawler")
        two = dtab.lookup(one)
        self.assertEqual(two, NameTree.read("/s#/crawler"))
        three = dtab.lookup(two)
        self.assertEqual(three, NameTree.read("/s##/prod/crawler"))
        four = dtab.lookup(three)
        self.assertEqual(
            four, NameTree.read("/zk/zk.local.twitter.com:2181/prod/crawler")
        )
        five = dtab.lookup(four)
        self.assertEqual(
            five, NameTree.read("/zk#/zk.local.twitter.com:2181/prod/crawler")
        )
        six = dtab.lookup(five)
        self.assertEqual(
            six,
            NameTree.read(
                "/$/com.twitter.serverset/zk.local.twitter.com:2181/prod/crawler"
            ),
        )

    def test_lookup_alternate(self):
        dtab = Dtab.read(
            """/zk#    =>      /$/com.twitter.serverset;
                        /zk     =>      /zk#;
                        /s##    =>      /zk/zk.local.twitter.com:2181;
                        /s#     =>      /s##/prod;
                        /s      =>      /s#;
                        /s#     =>      /s##/staging;"""
        )

        one = NameTree.read("/s/crawler")
        two = dtab.lookup(one)
        self.assertEqual(two, NameTree.read("/s#/crawler"))
        alternates = dtab.lookup(two)
        self.assertIsInstance(alternates, NameTree.Alt)
        self.assertEqual(len(alternates), 2)

        self.assertEqual(alternates.trees[0], NameTree.read("/s##/staging/crawler"))
        three0 = dtab.lookup(alternates.trees[0])
        self.assertEqual(
            three0, NameTree.read("/zk/zk.local.twitter.com:2181/staging/crawler")
        )
        four0 = dtab.lookup(three0)
        self.assertEqual(
            four0, NameTree.read("/zk#/zk.local.twitter.com:2181/staging/crawler")
        )
        five0 = dtab.lookup(four0)
        self.assertEqual(
            five0,
            NameTree.read(
                "/$/com.twitter.serverset/zk.local.twitter.com:2181/staging/crawler"
            ),
        )

        self.assertEqual(alternates.trees[1], NameTree.read("/s##/prod/crawler"))
        three1 = dtab.lookup(alternates.trees[1])
        self.assertEqual(
            three1, NameTree.read("/zk/zk.local.twitter.com:2181/prod/crawler")
        )
        four1 = dtab.lookup(three1)
        self.assertEqual(
            four1, NameTree.read("/zk#/zk.local.twitter.com:2181/prod/crawler")
        )
        five1 = dtab.lookup(four1)
        self.assertEqual(
            five1,
            NameTree.read(
                "/$/com.twitter.serverset/zk.local.twitter.com:2181/prod/crawler"
            ),
        )
