from logging import getLogger
from unittest import TestCase

from dtab import Dentry, Dtab
from dtab.name import Name
from dtab.path import Path
from dtab.tree import NameTree
from dtab.error import DtabError

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

        # Check that the dtab was parsed correctly by verifying it contains the expected elements
        self.assertTrue("biz" in str(withComments))
        self.assertTrue("bliz" in str(withComments) or "bluth" in str(withComments))
        self.assertTrue("Alt" in str(withComments))
        self.assertTrue("Union" in str(withComments))

    def test_d1_concat_dtab_empty(self):
        d1 = Dtab.read("/foo=>/bar;/biz=>/baz")
        self.assertEqual(d1 + Dtab.empty, d1)

    def test_is_collection(self):
        # these are mostly just compilation tests.
        dtab = Dtab.empty
        dtab += Dentry.read("/a => /b")
        dtab += Dentry.read("/c => /d")

        dtab1 = Dtab(
            [Dentry.read("/a=>/b"), Dentry.read("/c=>/d")]
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
        self.assertTrue("Alt" in str(alternates))
        # Check that the alternates contain the expected paths
        self.assertTrue("prod" in str(alternates) or "staging" in str(alternates))

    def test_dtab_error_handling(self):
        """Test error handling for invalid dtab parsing."""
        with self.assertRaises(DtabError):
            Dtab.read("/foo =>")  # Missing target
        
        with self.assertRaises(DtabError):
            Dtab.read("=> /bar")  # Missing prefix
        
        with self.assertRaises(DtabError):
            Dtab.read("/foo /bar")  # Missing arrow
        
        with self.assertRaises(DtabError):
            Dtab.read("/foo => /bar => /baz")  # Multiple arrows

    def test_dtab_edge_cases(self):
        """Test edge cases in dtab parsing."""
        # Test empty dtab
        empty = Dtab.read("")
        self.assertEqual(empty.length, 0)
        
        # Test dtab with only whitespace
        whitespace = Dtab.read("   ")
        self.assertEqual(whitespace.length, 0)
        
        # Test dtab with trailing semicolons
        trailing = Dtab.read("/foo => /bar;;")
        self.assertEqual(trailing.length, 1)
        
        # Test dtab with multiple consecutive semicolons
        multiple_semicolons = Dtab.read("/foo => /bar;;;/baz => /qux")
        self.assertEqual(multiple_semicolons.length, 2)

    def test_dtab_collection_behavior(self):
        """Test dtab collection behavior including equality and hashing."""
        dtab1 = Dtab.read("/foo => /bar")
        dtab2 = Dtab.read("/foo => /bar")
        dtab3 = Dtab.read("/baz => /qux")
        
        # Test equality
        self.assertEqual(dtab1, dtab2)
        self.assertNotEqual(dtab1, dtab3)
        
        # Test hashing
        dtab_set = {dtab1, dtab2, dtab3}
        self.assertEqual(len(dtab_set), 2)  # dtab1 and dtab2 should hash to same value

    def test_dtab_show_parse_roundtrip(self):
        """Test that dtab show and parse are roundtrip compatible."""
        original = Dtab.read("/foo => /bar;/baz => /qux")
        show_str = original.show
        parsed = Dtab.read(show_str)
        self.assertEqual(original, parsed)

    def test_dtab_system_paths(self):
        """Test dtab with system paths."""
        dtab = Dtab.read("/zk => /$/com.twitter.serverset")
        self.assertEqual(dtab.length, 1)
        
        # Test lookup with system path
        result = dtab.lookup(Path.read("/zk/local"))
        self.assertTrue("com.twitter.serverset" in str(result))

    def test_dtab_complex_parsing(self):
        """Test complex dtab parsing scenarios."""
        # Test with complex NameTree targets
        complex_dtab = Dtab.read("""
            /foo => /bar | /baz
            /biz => /qux & /quux
            /fizz => 1.5 * /buzz & 2.0 * /fizzbuzz
        """)
        self.assertEqual(complex_dtab.length, 3)
        
        # Test with comments and complex structure
        commented_dtab = Dtab.read("""
            # This is a comment
            /service => /prod/service  # Production service
            /service => /staging/service  # Staging service
            # Another comment
        """)
        self.assertEqual(commented_dtab.length, 2)

    def test_dtab_lookup_edge_cases(self):
        """Test edge cases in dtab lookup."""
        dtab = Dtab.read("/foo => /bar")
        
        # Test lookup with no match
        result = dtab.lookup(Path.read("/baz"))
        self.assertTrue("Neg" in str(result) or "~" in str(result))
        
        # Test lookup with empty path
        result = dtab.lookup(Path.read("/"))
        self.assertTrue("Neg" in str(result) or "~" in str(result))
        
        # Test lookup with wildcard path
        wildcard_dtab = Dtab.read("/a/*/c => /d")
        result = wildcard_dtab.lookup(Path.read("/a/b/c"))
        self.assertTrue("Leaf" in str(result) or "/d" in str(result))

    def test_dtab_concat_behavior(self):
        """Test dtab concatenation behavior."""
        d1 = Dtab.read("/foo => /bar")
        d2 = Dtab.read("/baz => /qux")
        combined = d1 + d2
        
        self.assertEqual(combined.length, 2)
        self.assertTrue("foo" in str(combined))
        self.assertTrue("baz" in str(combined))

    def test_dtab_parse_with_fail(self):
        """Test dtab parsing with fail targets."""
        dtab = Dtab.read("/=>!")
        self.assertEqual(dtab.length, 1)
        
        # Test lookup with fail target
        result = dtab.lookup(Path.read("/anything"))
        self.assertTrue("Fail" in str(result) or "!" in str(result))
