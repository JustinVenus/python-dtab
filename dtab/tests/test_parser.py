from unittest import TestCase

from dtab import Dentry, Dtab
from dtab.error import IllegalArgumentException
from dtab.parser import NameTreeParsers
from dtab.path import Path
from dtab.tree import NameTree
from dtab.util import PY3


class NameTreeParserTest(TestCase):
    def test_parsePath(self):
        self.assertTrue(NameTreeParsers.parsePath("/") == Path.empty)

        self.assertTrue(
            NameTreeParsers.parsePath("/foo/bar") == Path.Utf8("foo", "bar")
        )

        self.assertTrue(
            NameTreeParsers.parsePath("  /foo/bar  ") == Path.Utf8("foo", "bar")
        )

        self.assertTrue(NameTreeParsers.parsePath("/\x66\x6f\x6f") == Path.Utf8("foo"))

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parsePath("")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parsePath("/foo/bar/")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parsePath("/  foo/bar/  ")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parsePath("/{}")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parsePath("/\\?")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parsePath("/\\x?")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parsePath("/\\x0?")

    def test_error_messages(self):
        try:
            NameTreeParsers.parsePath("/foo^bar")
        except IllegalArgumentException as e:
            if PY3:
                self.assertTrue("'/foo[^]bar'" in str(e))
            else:
                self.assertTrue("'/foo[^]bar'" in e.message)
        try:
            NameTreeParsers.parsePath("/foo/bar/")
        except IllegalArgumentException as e:
            if PY3:
                self.assertTrue("'/foo/bar/[]'" in str(e))
            else:
                self.assertTrue("'/foo/bar/[]'" in e.message)

    def test_parseNameTree(self):
        defaultWeight = NameTree.Weighted.defaultWeight

        self.assertTrue(
            NameTreeParsers.parseNameTree("! | ~ | $")
            == NameTree.Alt(NameTree.Fail, NameTree.Neg, NameTree.Empty)
        )

        self.assertTrue(
            NameTreeParsers.parseNameTree("/foo/bar")
            == NameTree.Leaf(Path.Utf8("foo", "bar"))
        )

        self.assertTrue(
            NameTreeParsers.parseNameTree("  /foo & /bar  ")
            == NameTree.Union(
                NameTree.Weighted(defaultWeight, NameTree.Leaf(Path.Utf8("foo"))),
                NameTree.Weighted(defaultWeight, NameTree.Leaf(Path.Utf8("bar"))),
            )
        )

        self.assertTrue(
            NameTreeParsers.parseNameTree("  /foo | /bar  ")
            == NameTree.Alt(
                NameTree.Leaf(Path.Utf8("foo")), NameTree.Leaf(Path.Utf8("bar"))
            )
        )

        self.assertTrue(
            NameTreeParsers.parseNameTree("/foo & /bar | /bar & /baz")
            == NameTree.Alt(
                NameTree.Union(
                    NameTree.Weighted(defaultWeight, NameTree.Leaf(Path.Utf8("foo"))),
                    NameTree.Weighted(defaultWeight, NameTree.Leaf(Path.Utf8("bar"))),
                ),
                NameTree.Union(
                    NameTree.Weighted(defaultWeight, NameTree.Leaf(Path.Utf8("bar"))),
                    NameTree.Weighted(defaultWeight, NameTree.Leaf(Path.Utf8("baz"))),
                ),
            )
        )

        self.assertTrue(
            NameTreeParsers.parseNameTree("1 * /foo & 2 * /bar | .5 * /bar & .5 * /baz")
            == NameTree.Alt(
                NameTree.Union(
                    NameTree.Weighted(1, NameTree.Leaf(Path.Utf8("foo"))),
                    NameTree.Weighted(2, NameTree.Leaf(Path.Utf8("bar"))),
                ),
                NameTree.Union(
                    NameTree.Weighted(0.5, NameTree.Leaf(Path.Utf8("bar"))),
                    NameTree.Weighted(0.5, NameTree.Leaf(Path.Utf8("baz"))),
                ),
            )
        )

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parseNameTree("")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parseNameTree("#")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parseNameTree("/foo &")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parseNameTree("/foo & 0.1.2 * /bar")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parseNameTree("/foo & . * /bar")

    def test_parseDentry(self):
        self.assertTrue(
            NameTreeParsers.parseDentry("/=>!") == Dentry(Path.empty, NameTree.Fail)
        )

        self.assertTrue(
            NameTreeParsers.parseDentry("/ => !") == Dentry(Path.empty, NameTree.Fail)
        )

        self.assertTrue(
            NameTreeParsers.parseDentry("/foo/*/bar => !")
            == Dentry(
                Dentry.Prefix(
                    Dentry.Prefix.Label("foo"),
                    Dentry.Prefix.AnyElem,
                    Dentry.Prefix.Label("bar"),
                ),
                NameTree.Fail,
            )
        )

        self.assertTrue(
            NameTreeParsers.parseDentry("/foo/bar/baz => !")
            == Dentry(
                Dentry.Prefix(
                    Dentry.Prefix.Label("foo"),
                    Dentry.Prefix.Label("bar"),
                    Dentry.Prefix.Label("baz"),
                ),
                NameTree.Fail,
            )
        )

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parseDentry("/foo/*bar/baz => !")

        with self.assertRaises(IllegalArgumentException):
            NameTreeParsers.parseDentry("/&!")

    def test_parseDtab(self):
        self.assertTrue(NameTreeParsers.parseDtab("") == Dtab.empty)

        self.assertTrue(
            NameTreeParsers.parseDtab("  /=>!  ")
            == Dtab([Dentry(Path.empty, NameTree.Fail)])
        )

        self.assertTrue(
            NameTreeParsers.parseDtab("/=>!;")
            == Dtab([Dentry(Path.empty, NameTree.Fail)])
        )

        self.assertTrue(
            NameTreeParsers.parseDtab("/=>!;/foo=>/bar")
            == Dtab(
                [
                    Dentry(Path.empty, NameTree.Fail),
                    Dentry(Path.Utf8("foo"), NameTree.Leaf(Path.Utf8("bar"))),
                ]
            )
        )
