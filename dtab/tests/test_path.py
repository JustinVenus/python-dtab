from unittest import TestCase

from dtab import Path, DtabError


class PathTest(TestCase):
    def test_show(self):
        self.assertTrue(Path.read("/foo/bar").show == "/foo/bar")

    def test_path_operations(self):
        """Test path operations like take, drop, concat, size."""
        path = Path.read("/foo/bar/baz")
        self.assertEqual(path.size, 3)
        self.assertEqual(path.take(2), Path.read("/foo/bar"))
        self.assertEqual(path.drop(1), Path.read("/bar/baz"))
        
        path1 = Path.read("/foo")
        path2 = Path.read("/bar")
        self.assertEqual(path1.concat(path2), Path.read("/foo/bar"))

    def test_path_matches(self):
        """Test wildcard matching functionality."""
        pat = Path.read("/foo/*/baz")
        val = Path.read("/foo/bar/baz")
        self.assertTrue(pat.matches(val))
        
        val2 = Path.read("/foo/qux/baz")
        self.assertTrue(pat.matches(val2))
        
        val3 = Path.read("/foo/baz")
        self.assertFalse(pat.matches(val3))

    def test_system_paths(self):
        """Test system path handling with /$/ prefix."""
        sys_path = Path.read("/$/com.twitter.serverset")
        self.assertEqual(sys_path.show, "/$/com.twitter.serverset")
        
        # Test system path with additional elements
        sys_path2 = Path.read("/$/inet/0/8080")
        self.assertEqual(sys_path2.show, "/$/inet/0/8080")

    def test_path_parsing_edge_cases(self):
        """Test edge cases in path parsing."""
        # Test empty path
        empty = Path.empty()
        self.assertEqual(empty.show, "")
        
        # Test root path
        root = Path.read("/")
        self.assertEqual(root.show, "/")
        
        # Test path with special characters
        special = Path.read("/foo/bar/baz")
        self.assertEqual(special.show, "/foo/bar/baz")
        
        # Test path with wildcards
        wildcard = Path.read("/foo/*/baz")
        self.assertEqual(wildcard.show, "/foo/*/baz")

    def test_path_error_handling(self):
        """Test error handling for invalid paths."""
        with self.assertRaises(DtabError):
            Path.read("")
        
        with self.assertRaises(DtabError):
            Path.read("invalid")
        
        # Trailing slash should be handled gracefully
        try:
            Path.read("/foo/bar/")
        except DtabError:
            self.fail("Path.read('/foo/bar/') should not raise DtabError")
