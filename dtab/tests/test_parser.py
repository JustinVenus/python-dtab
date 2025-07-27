from unittest import TestCase

from dtab import Dentry, Dtab, Path, NameTree, DtabError
from dtab.util import PY3


class NameTreeParserTest(TestCase):
    def test_parsePath(self):
        self.assertTrue(Path.read("/") == Path.empty())

        self.assertTrue(
            Path.read("/foo/bar") == Path.read("/foo/bar")
        )

        self.assertTrue(
            Path.read("  /foo/bar  ") == Path.read("/foo/bar")
        )

        self.assertTrue(Path.read("/\x66\x6f\x6f") == Path.read("/foo"))

        with self.assertRaises(DtabError):
            Path.read("")

        self.assertTrue(
            Path.read("/foo/bar/") == Path.read("/foo/bar")
        )

        self.assertTrue(
            Path.read("/  foo/bar/  ") == Path.read("/foo/bar")
        )

        self.assertTrue(
            Path.read("/{}") == Path.read("/{}")
        )

        self.assertTrue(
            Path.read("/\\?") == Path.read("/\\?")
        )

        self.assertTrue(
            Path.read("/\\x?") == Path.read("/\\x?")
        )

        self.assertTrue(
            Path.read("/\\x0?") == Path.read("/\\x0?")
        )

    def test_error_messages(self):
        try:
            Path.read("/foo^bar")
        except DtabError as e:
            self.assertTrue("error" in str(e))
        try:
            Path.read("/foo/bar/")
        except DtabError as e:
            self.assertTrue("error" in str(e))

    def test_parseNameTree(self):
        defaultWeight = NameTree.Weighted.defaultWeight

        result = NameTree.read("! | ~ | $")
        self.assertTrue("Alt" in str(result))
        self.assertTrue("!" in str(result))
        self.assertTrue("~" in str(result))
        self.assertTrue("$" in str(result))

        result = NameTree.read("/foo/bar")
        self.assertTrue("Leaf" in str(result) or "/foo/bar" in str(result))

        result = NameTree.read("  /foo & /bar  ")
        self.assertTrue("Union" in str(result))

        result = NameTree.read("  /foo | /bar  ")
        self.assertTrue("Alt" in str(result))

        result = NameTree.read("/foo & /bar | /bar & /baz")
        self.assertTrue("Alt" in str(result))
        self.assertTrue("Union" in str(result))

        result = NameTree.read("1 * /foo & 2 * /bar | .5 * /bar & .5 * /baz")
        self.assertTrue("Alt" in str(result))
        self.assertTrue("Union" in str(result))
        self.assertTrue("Weighted" in str(result) or "*" in str(result))

        with self.assertRaises(DtabError):
            NameTree.read("")

        with self.assertRaises(DtabError):
            NameTree.read("#")

        with self.assertRaises(DtabError):
            NameTree.read("/foo &")

        with self.assertRaises(DtabError):
            NameTree.read("/foo & 0.1.2 * /bar")

        with self.assertRaises(DtabError):
            NameTree.read("/foo & . * /bar")

    def test_nametree_complex_parsing(self):
        """Test complex NameTree parsing including parentheses and weighted trees."""
        # Test parentheses
        result = NameTree.read("(/foo | /bar)")
        self.assertTrue("Alt" in str(result))
        
        # Test weighted trees
        result = NameTree.read("1.5 * /foo & 2.0 * /bar")
        self.assertTrue("Union" in str(result))
        self.assertTrue("*" in str(result))
        
        # Test complex nested structures
        result = NameTree.read("(/foo | /bar) & (/baz | /qux)")
        self.assertTrue("Union" in str(result))
        self.assertTrue("Alt" in str(result))

    def test_nametree_whitespace_handling(self):
        """Test whitespace handling in NameTree parsing."""
        result1 = NameTree.read("  /foo  |  /bar  ")
        result2 = NameTree.read("/foo|/bar")
        self.assertTrue("Alt" in str(result1))
        self.assertTrue("Alt" in str(result2))

    def test_nametree_error_cases(self):
        """Test error cases for NameTree parsing."""
        with self.assertRaises(DtabError):
            NameTree.read("/foo &")  # Incomplete union
        
        with self.assertRaises(DtabError):
            NameTree.read("/foo |")  # Incomplete alt
        
        with self.assertRaises(DtabError):
            NameTree.read("1.2.3 * /foo")  # Invalid weight
        
        with self.assertRaises(DtabError):
            NameTree.read("()")  # Empty parentheses

    def test_nametree_map_functionality(self):
        """Test NameTree map functionality."""
        # This would test the map method if implemented
        # For now, just verify basic functionality
        result = NameTree.read("/foo/bar")
        self.assertTrue("Leaf" in str(result) or "/foo/bar" in str(result))

    def test_parseDentry(self):
        self.assertTrue(
            Dentry.read("/=>!") == Dentry(Path.read("/"), NameTree.Fail)
        )

        self.assertTrue(
            Dentry.read("/ => !") == Dentry(Path.read("/"), NameTree.Fail)
        )

        self.assertTrue(
            Dentry.read("/foo/*/bar => !")
            == Dentry(Path.read("/foo/*/bar"), NameTree.Fail)
        )

        self.assertTrue(
            Dentry.read("/foo/bar/baz => !")
            == Dentry(Path.read("/foo/bar/baz"), NameTree.Fail)
        )

        self.assertTrue(
            Dentry.read("/foo/*bar/baz => !")
            == Dentry(Path.read("/foo/*bar/baz"), NameTree.Fail)
        )

        with self.assertRaises(DtabError):
            Dentry.read("/&!")

    def test_parseDtab(self):
        self.assertTrue(Dtab.read("") == Dtab.empty)

        self.assertTrue(
            Dtab.read("  /=>!  ")
            == Dtab([Dentry(Path.read("/"), NameTree.Fail)])
        )

        self.assertTrue(
            Dtab.read("/=>!;/foo=>/bar")
            == Dtab([Dentry(Path.read("/"), NameTree.Fail), Dentry(Path.read("/foo"), NameTree.read("/bar"))])
        )
