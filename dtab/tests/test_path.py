from unittest import TestCase

from dtab.parser import NameTreeParsers


class PathTest(TestCase):
    def test_show(self):
        self.assertTrue(NameTreeParsers.parsePath("/foo/bar").show == "/foo/bar")
