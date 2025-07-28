#[cfg(test)]
mod tests {
    use crate::dtab::{Dentry, Dtab};
    use crate::nametree::{NameTree, Weighted};
    use crate::path::{Path, PathElem};

    #[test]
    fn test_path_parsing_and_show() {
        let p = Path::read("/foo/bar").unwrap();
        assert_eq!(p.show(), "/foo/bar");
        let p2 = Path::from_utf8(&["foo", "bar"]);
        assert_eq!(p, p2);
        let p3 = Path::read("/foo/*/baz").unwrap();
        assert_eq!(
            p3.0,
            vec![
                PathElem::Label("foo".into()),
                PathElem::Wildcard,
                PathElem::Label("baz".into())
            ]
        );
    }

    #[test]
    fn test_path_matches() {
        let pat = Path::read("/foo/*/baz").unwrap();
        let val = Path::read("/foo/bar/baz").unwrap();
        assert!(pat.matches(&val));
        let val2 = Path::read("/foo/qux/baz").unwrap();
        assert!(pat.matches(&val2));
        let val3 = Path::read("/foo/baz").unwrap();
        assert!(!pat.matches(&val3));
    }

    #[test]
    fn test_path_operations() {
        let path = Path::read("/foo/bar/baz").unwrap();
        assert_eq!(path.size(), 3);
        assert_eq!(path.take(2), Path::read("/foo/bar").unwrap());
        assert_eq!(path.drop(1), Path::read("/bar/baz").unwrap());

        let path1 = Path::read("/foo").unwrap();
        let path2 = Path::read("/bar").unwrap();
        assert_eq!(path1.concat(&path2), Path::read("/foo/bar").unwrap());
    }

    #[test]
    fn test_dentry_parse() {
        let d = Dentry::parse("/foo/*/baz => /bar").unwrap();
        assert_eq!(d.prefix, Path::read("/foo/*/baz").unwrap());
        assert_eq!(d.target, NameTree::Leaf(Path::read("/bar").unwrap()));
    }

    #[test]
    fn test_dentry_parse_with_fail() {
        let d = Dentry::parse("/=>!").unwrap();
        assert_eq!(d.prefix, Path::empty());
        assert_eq!(d.target, NameTree::Fail);
    }

    #[test]
    fn test_dtab_parse_with_fail() {
        let dtab = Dtab::parse("/=>!").unwrap();
        assert_eq!(dtab.0.len(), 1);
        assert_eq!(dtab.0[0].prefix, Path::empty());
        assert_eq!(dtab.0[0].target, NameTree::Fail);
    }

    #[test]
    fn test_dtab_parse_and_concat() {
        let d1 = Dtab::parse("/foo => /bar").unwrap();
        let d2 = Dtab::parse("/foo=>/biz;/biz=>/inet/0/8080;/bar=>/inet/0/9090").unwrap();
        let d3 = d1.concat(&d2);
        assert_eq!(d3.0.len(), 4);
    }

    #[test]
    fn test_dtab_lookup_simple() {
        let dtab = Dtab::parse("/foo => /bar").unwrap();
        let path = Path::read("/foo/baz").unwrap();
        let result = dtab.lookup(&path).unwrap();
        assert_eq!(result, NameTree::Leaf(Path::read("/bar/baz").unwrap()));
    }

    #[test]
    fn test_dtab_lookup_with_wildcard() {
        let dtab = Dtab::parse("/a/*/c => /d").unwrap();
        let path = Path::read("/a/b/c/e/f").unwrap();
        let result = dtab.lookup(&path).unwrap();
        assert_eq!(result, NameTree::Leaf(Path::read("/d/e/f").unwrap()));
    }

    #[test]
    fn test_dtab_lookup_multiple_matches() {
        let dtab = Dtab::parse("/s => /s#;/s# => /s##/prod;/s# => /s##/staging").unwrap();
        let path = Path::read("/s/crawler").unwrap();
        let result = dtab.lookup(&path).unwrap();

        // First lookup should return /s#/crawler
        assert_eq!(result, NameTree::Leaf(Path::read("/s#/crawler").unwrap()));

        // Second lookup on the result should return alternatives
        let result2 = dtab.lookup(&Path::read("/s#/crawler").unwrap()).unwrap();

        // Should return alternatives for the multiple /s# matches
        match result2 {
            NameTree::Alt(alternatives) => {
                assert_eq!(alternatives.len(), 2);
                assert!(
                    alternatives
                        .contains(&NameTree::Leaf(Path::read("/s##/prod/crawler").unwrap()))
                );
                assert!(
                    alternatives
                        .contains(&NameTree::Leaf(Path::read("/s##/staging/crawler").unwrap()))
                );
            }
            _ => panic!(
                "Expected Alt with multiple alternatives, got: {:?}",
                result2
            ),
        }
    }

    #[test]
    fn test_dtab_lookup_complex_chain() {
        let dtab_str = "/zk# => /$/com.twitter.serverset;/zk => /zk#;/s## => /zk/zk.local.twitter.com:2181;/s# => /s##/prod;/s => /s#";
        let dtab = Dtab::parse(dtab_str).unwrap();

        let path = Path::read("/s/crawler").unwrap();
        let result = dtab.lookup(&path).unwrap();
        assert_eq!(result, NameTree::Leaf(Path::read("/s#/crawler").unwrap()));

        // Continue the chain
        let result2 = dtab.lookup(&Path::read("/s#/crawler").unwrap()).unwrap();
        assert_eq!(
            result2,
            NameTree::Leaf(Path::read("/s##/prod/crawler").unwrap())
        );

        let result3 = dtab
            .lookup(&Path::read("/s##/prod/crawler").unwrap())
            .unwrap();
        assert_eq!(
            result3,
            NameTree::Leaf(Path::read("/zk/zk.local.twitter.com:2181/prod/crawler").unwrap())
        );
    }

    #[test]
    fn test_dtab_lookup_with_union_target() {
        let dtab = Dtab::parse("/foo => /bar & /baz").unwrap();
        let path = Path::read("/foo/qux").unwrap();
        let result = dtab.lookup(&path).unwrap();

        match result {
            NameTree::Union(weighted_trees) => {
                assert_eq!(weighted_trees.len(), 2);
                assert_eq!(
                    *weighted_trees[0].tree,
                    NameTree::Leaf(Path::read("/bar/qux").unwrap())
                );
                assert_eq!(
                    *weighted_trees[1].tree,
                    NameTree::Leaf(Path::read("/baz/qux").unwrap())
                );
            }
            _ => panic!("Expected Union"),
        }
    }

    #[test]
    fn test_dtab_lookup_no_match() {
        let dtab = Dtab::parse("/foo => /bar").unwrap();
        let path = Path::read("/baz/qux").unwrap();
        let result = dtab.lookup(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_dtab_parse_with_comments() {
        let dtab_str = "# This is a comment
        /foo => /bar;  # another comment
        /biz => /baz; # semicolon comment
        #/ignore=>/me;  # full line comment
        /test => /result";

        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 3); // Should have 3 entries, ignoring comments

        // Check that the entries are parsed correctly
        assert_eq!(dtab.0[0].prefix, Path::read("/foo").unwrap());
        assert_eq!(
            dtab.0[0].target,
            NameTree::Leaf(Path::read("/bar").unwrap())
        );
        assert_eq!(dtab.0[1].prefix, Path::read("/biz").unwrap());
        assert_eq!(
            dtab.0[1].target,
            NameTree::Leaf(Path::read("/baz").unwrap())
        );
        assert_eq!(dtab.0[2].prefix, Path::read("/test").unwrap());
        assert_eq!(
            dtab.0[2].target,
            NameTree::Leaf(Path::read("/result").unwrap())
        );
    }

    #[test]
    fn test_dtab_parse_exact_python_test_case() {
        let dtab_str = r#"# a comment
      /#foo => /biz  # another comment
             | ( /bliz & # yet another comment
                 /bluth ) # duh bluths
             ; #finalmente
      #/ignore=>/me;"#;

        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 1); // Should have 1 entry, ignoring comments

        // Check that the entry is parsed correctly
        assert_eq!(dtab.0[0].prefix, Path::read("/#foo").unwrap());

        // The target should be an Alt with a Union inside
        match &dtab.0[0].target {
            NameTree::Alt(trees) => {
                assert_eq!(trees.len(), 2);
                // First element should be /biz
                assert_eq!(trees[0], NameTree::Leaf(Path::read("/biz").unwrap()));
                // Second element should be a Union
                match &trees[1] {
                    NameTree::Union(weighted) => {
                        assert_eq!(weighted.len(), 2);
                        assert_eq!(
                            weighted[0].tree,
                            Box::new(NameTree::Leaf(Path::read("/bliz").unwrap()))
                        );
                        assert_eq!(
                            weighted[1].tree,
                            Box::new(NameTree::Leaf(Path::read("/bluth").unwrap()))
                        );
                    }
                    _ => panic!("Expected Union as second element of Alt"),
                }
            }
            _ => panic!("Expected Alt as target"),
        }
    }

    #[test]
    fn test_nametree_parse_basic() {
        // Test basic NameTree parsing
        assert_eq!(NameTree::parse("!").unwrap(), NameTree::Fail);
        assert_eq!(NameTree::parse("~").unwrap(), NameTree::Neg);
        assert_eq!(NameTree::parse("$").unwrap(), NameTree::Empty);
        assert_eq!(
            NameTree::parse("/foo").unwrap(),
            NameTree::Leaf(Path::read("/foo").unwrap())
        );
    }

    #[test]
    fn test_nametree_parse_alt() {
        // Test alternatives (|)
        let tree = NameTree::parse("/foo | /bar").unwrap();
        assert_eq!(
            tree,
            NameTree::Alt(vec![
                NameTree::Leaf(Path::read("/foo").unwrap()),
                NameTree::Leaf(Path::read("/bar").unwrap())
            ])
        );

        let tree2 = NameTree::parse("! | ~ | $").unwrap();
        assert_eq!(
            tree2,
            NameTree::Alt(vec![NameTree::Fail, NameTree::Neg, NameTree::Empty])
        );
    }

    #[test]
    fn test_nametree_parse_union() {
        // Test unions (&)
        let tree = NameTree::parse("/foo & /bar").unwrap();
        assert_eq!(
            tree,
            NameTree::Union(vec![
                Weighted::new(1.0, NameTree::Leaf(Path::read("/foo").unwrap())),
                Weighted::new(1.0, NameTree::Leaf(Path::read("/bar").unwrap()))
            ])
        );
    }

    #[test]
    fn test_nametree_parse_weighted() {
        // Test weighted trees
        let tree = NameTree::parse("2 * /foo").unwrap();
        assert_eq!(
            tree,
            NameTree::Union(vec![Weighted::new(
                2.0,
                NameTree::Leaf(Path::read("/foo").unwrap())
            )])
        );

        let tree2 = NameTree::parse("0.5 * /bar").unwrap();
        assert_eq!(
            tree2,
            NameTree::Union(vec![Weighted::new(
                0.5,
                NameTree::Leaf(Path::read("/bar").unwrap())
            )])
        );
    }

    #[test]
    fn test_nametree_parse_complex() {
        // Test complex expressions
        let tree = NameTree::parse("/foo & /bar | /bar & /baz").unwrap();
        assert_eq!(
            tree,
            NameTree::Alt(vec![
                NameTree::Union(vec![
                    Weighted::new(1.0, NameTree::Leaf(Path::read("/foo").unwrap())),
                    Weighted::new(1.0, NameTree::Leaf(Path::read("/bar").unwrap()))
                ]),
                NameTree::Union(vec![
                    Weighted::new(1.0, NameTree::Leaf(Path::read("/bar").unwrap())),
                    Weighted::new(1.0, NameTree::Leaf(Path::read("/baz").unwrap()))
                ])
            ])
        );

        let tree2 = NameTree::parse("1 * /foo & 2 * /bar | .5 * /bar & .5 * /baz").unwrap();
        assert_eq!(
            tree2,
            NameTree::Alt(vec![
                NameTree::Union(vec![
                    Weighted::new(1.0, NameTree::Leaf(Path::read("/foo").unwrap())),
                    Weighted::new(2.0, NameTree::Leaf(Path::read("/bar").unwrap()))
                ]),
                NameTree::Union(vec![
                    Weighted::new(0.5, NameTree::Leaf(Path::read("/bar").unwrap())),
                    Weighted::new(0.5, NameTree::Leaf(Path::read("/baz").unwrap()))
                ])
            ])
        );
    }

    #[test]
    fn test_nametree_parse_parentheses() {
        // Test parenthesized expressions
        let tree = NameTree::parse("(/foo | /bar) & /baz").unwrap();
        assert_eq!(
            tree,
            NameTree::Union(vec![
                Weighted::new(
                    1.0,
                    NameTree::Alt(vec![
                        NameTree::Leaf(Path::read("/foo").unwrap()),
                        NameTree::Leaf(Path::read("/bar").unwrap())
                    ])
                ),
                Weighted::new(1.0, NameTree::Leaf(Path::read("/baz").unwrap()))
            ])
        );
    }

    #[test]
    fn test_nametree_parse_whitespace() {
        // Test whitespace handling
        let tree1 = NameTree::parse("  /foo & /bar  ").unwrap();
        let tree2 = NameTree::parse("/foo&/bar").unwrap();
        assert_eq!(tree1, tree2);
    }

    #[test]
    fn test_nametree_map() {
        // Test the map functionality
        let tree = NameTree::parse("/foo | /bar").unwrap();
        let mapped = tree.map(|path| path.add_suffix(&Path::read("/suffix").unwrap()));

        match mapped {
            NameTree::Alt(trees) => {
                assert_eq!(trees.len(), 2);
                assert_eq!(trees[0], NameTree::Leaf(Path::read("/foo/suffix").unwrap()));
                assert_eq!(trees[1], NameTree::Leaf(Path::read("/bar/suffix").unwrap()));
            }
            _ => panic!("Expected Alt"),
        }
    }

    #[test]
    fn test_dtab_error_handling() {
        assert!(Dtab::parse("/foo").is_err());
        assert!(Dentry::parse("/foo =>").is_err());
        assert!(Path::read("foo/bar").is_err());

        // Test NameTree error cases
        assert!(NameTree::parse("").is_err());
        assert!(NameTree::parse("/foo &").is_err());
        assert!(NameTree::parse("2 *").is_err());
        assert!(NameTree::parse("(/foo").is_err());
        assert!(NameTree::parse("/foo |").is_err());
    }

    #[test]
    fn test_dtab_trailing_semicolon() {
        // Test that trailing semicolons are allowed
        let dtab_str = "/b => /c;/a => /b;";
        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 2);

        // Test with whitespace around semicolons
        let dtab_str2 = "/b => /c ; /a => /b ; ";
        let dtab2 = Dtab::parse(dtab_str2).unwrap();
        assert_eq!(dtab2.0.len(), 2);
    }

    #[test]
    fn test_dtab_collection_behavior() {
        // Test that Dtab behaves like a collection
        let dtab = Dtab::parse("/foo => /bar").unwrap();

        // Test concatenation with empty dtab
        let empty = Dtab::parse("").unwrap();
        let result = dtab.concat(&empty);
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0].prefix, Path::read("/foo").unwrap());
        assert_eq!(
            result.0[0].target,
            NameTree::Leaf(Path::read("/bar").unwrap())
        );

        // Test concatenation with another dtab
        let dtab2 = Dtab::parse("/biz => /baz").unwrap();
        let combined = dtab.concat(&dtab2);
        assert_eq!(combined.0.len(), 2);
    }

    #[test]
    fn test_dtab_lookup_alternate() {
        // Test lookup with multiple alternatives (Alt)
        let dtab_str = "/zk# => /$/com.twitter.serverset;/zk => /zk#;/s## => /zk/zk.local.twitter.com:2181;/s# => /s##/prod;/s => /s#;/s# => /s##/staging";
        let dtab = Dtab::parse(dtab_str).unwrap();

        let path = Path::read("/s/crawler").unwrap();
        let result = dtab.lookup(&path).unwrap();
        assert_eq!(result, NameTree::Leaf(Path::read("/s#/crawler").unwrap()));

        // Second lookup should return alternatives
        let result2 = dtab.lookup(&Path::read("/s#/crawler").unwrap()).unwrap();

        match result2 {
            NameTree::Alt(alternatives) => {
                assert_eq!(alternatives.len(), 2);
                assert!(
                    alternatives
                        .contains(&NameTree::Leaf(Path::read("/s##/prod/crawler").unwrap()))
                );
                assert!(
                    alternatives
                        .contains(&NameTree::Leaf(Path::read("/s##/staging/crawler").unwrap()))
                );
            }
            _ => panic!("Expected Alt with multiple alternatives"),
        }
    }

    #[test]
    fn test_dtab_lookup_complete_chain() {
        // Test the complete lookup chain as in Python test_lookup_simple
        let dtab_str = "/zk# => /$/com.twitter.serverset;/zk => /zk#;/s## => /zk/zk.local.twitter.com:2181;/s# => /s##/prod;/s => /s#";
        let dtab = Dtab::parse(dtab_str).unwrap();

        // First lookup
        let one = Path::read("/s/crawler").unwrap();
        let two = dtab.lookup(&one).unwrap();
        assert_eq!(two, NameTree::Leaf(Path::read("/s#/crawler").unwrap()));

        // Second lookup
        let three = dtab.lookup(&Path::read("/s#/crawler").unwrap()).unwrap();
        assert_eq!(
            three,
            NameTree::Leaf(Path::read("/s##/prod/crawler").unwrap())
        );

        // Third lookup
        let four = dtab
            .lookup(&Path::read("/s##/prod/crawler").unwrap())
            .unwrap();
        assert_eq!(
            four,
            NameTree::Leaf(Path::read("/zk/zk.local.twitter.com:2181/prod/crawler").unwrap())
        );

        // Fourth lookup
        let five = dtab
            .lookup(&Path::read("/zk/zk.local.twitter.com:2181/prod/crawler").unwrap())
            .unwrap();
        assert_eq!(
            five,
            NameTree::Leaf(Path::read("/zk#/zk.local.twitter.com:2181/prod/crawler").unwrap())
        );

        // Fifth lookup
        let six = dtab
            .lookup(&Path::read("/zk#/zk.local.twitter.com:2181/prod/crawler").unwrap())
            .unwrap();
        assert_eq!(
            six,
            NameTree::Leaf(
                Path::read("/$/com.twitter.serverset/zk.local.twitter.com:2181/prod/crawler")
                    .unwrap()
            )
        );
    }

    #[test]
    fn test_path_show_consistency() {
        // Test that Path.show() returns the expected format
        let path = Path::read("/foo/bar").unwrap();
        assert_eq!(path.show(), "/foo/bar");

        let empty_path = Path::empty();
        assert_eq!(empty_path.show(), "/");

        let wildcard_path = Path::read("/foo/*/bar").unwrap();
        assert_eq!(wildcard_path.show(), "/foo/*/bar");
    }

    #[test]
    fn test_nametree_parse_complex_with_comments() {
        // Test complex NameTree parsing with comments (matching Python test)
        let tree_str = "/biz | ( /bliz & /bluth )";
        let tree = NameTree::parse(tree_str).unwrap();

        match tree {
            NameTree::Alt(trees) => {
                assert_eq!(trees.len(), 2);
                // First element should be /biz
                assert_eq!(trees[0], NameTree::Leaf(Path::read("/biz").unwrap()));
                // Second element should be a Union
                match &trees[1] {
                    NameTree::Union(weighted) => {
                        assert_eq!(weighted.len(), 2);
                        assert_eq!(
                            weighted[0].tree,
                            Box::new(NameTree::Leaf(Path::read("/bliz").unwrap()))
                        );
                        assert_eq!(
                            weighted[1].tree,
                            Box::new(NameTree::Leaf(Path::read("/bluth").unwrap()))
                        );
                    }
                    _ => panic!("Expected Union as second element of Alt"),
                }
            }
            _ => panic!("Expected Alt as result"),
        }
    }

    #[test]
    fn test_dtab_parse_with_trailing_semicolons() {
        // Test parsing with various semicolon configurations
        let dtab_str = "/foo => /bar;/biz => /baz;";
        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 2);

        let dtab_str2 = "/foo => /bar ; /biz => /baz ; ";
        let dtab2 = Dtab::parse(dtab_str2).unwrap();
        assert_eq!(dtab2.0.len(), 2);
    }

    #[test]
    fn test_dtab_concat_behavior() {
        // Test Dtab concatenation behavior
        let d1 = Dtab::parse("/foo => /bar").unwrap();
        let d2 = Dtab::parse("/foo=>/biz;/biz=>/inet/0/8080;/bar=>/inet/0/9090").unwrap();
        let combined = d1.concat(&d2);
        assert_eq!(combined.0.len(), 4);

        // Test concatenation with empty dtab
        let empty = Dtab::parse("").unwrap();
        let result = d1.concat(&empty);
        assert_eq!(result.0.len(), 1);
    }

    #[test]
    fn test_dtab_parse_simple_comment() {
        // Test the simplest case that's failing
        let dtab_str = "/foo => /bar  # comment";
        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 1);
        assert_eq!(dtab.0[0].prefix, Path::read("/foo").unwrap());
        assert_eq!(
            dtab.0[0].target,
            NameTree::Leaf(Path::read("/bar").unwrap())
        );
    }

    #[test]
    fn test_dtab_parse_exact_failing_input() {
        // Test the exact input that's failing
        let dtab_str = "# This is a comment
        /foo => /bar  # another comment
        /biz => /baz; # semicolon comment
        #/ignore=>/me;  # full line comment
        /test => /result";

        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 3); // Should have 3 entries, ignoring comments

        // Check that the entries are parsed correctly
        assert_eq!(dtab.0[0].prefix, Path::read("/foo").unwrap());
        assert_eq!(
            dtab.0[0].target,
            NameTree::Leaf(Path::read("/bar").unwrap())
        );
        assert_eq!(dtab.0[1].prefix, Path::read("/biz").unwrap());
        assert_eq!(
            dtab.0[1].target,
            NameTree::Leaf(Path::read("/baz").unwrap())
        );
        assert_eq!(dtab.0[2].prefix, Path::read("/test").unwrap());
        assert_eq!(
            dtab.0[2].target,
            NameTree::Leaf(Path::read("/result").unwrap())
        );
    }

    #[test]
    fn test_debug_hash_prefix() {
        // Test the specific case that's failing
        let dentry_str = "/#foo => /biz";
        let dentry = Dentry::parse(dentry_str).unwrap();
        assert_eq!(dentry.prefix, Path::read("/#foo").unwrap());
        assert_eq!(dentry.target, NameTree::Leaf(Path::read("/biz").unwrap()));
    }

    #[test]
    fn test_dtab_parse_optional_semicolons_between_dentries() {
        // Semicolons are optional between dentries according to Scala reference
        let dtab_str = "/foo => /bar
/biz => /baz;";
        let result = Dtab::parse(dtab_str);
        assert!(
            result.is_ok(),
            "Parser should accept dentries without semicolons between them"
        );

        let dtab = result.unwrap();
        assert_eq!(dtab.0.len(), 2);
        assert_eq!(dtab.0[0].prefix, Path::read("/foo").unwrap());
        assert_eq!(
            dtab.0[0].target,
            NameTree::Leaf(Path::read("/bar").unwrap())
        );
        assert_eq!(dtab.0[1].prefix, Path::read("/biz").unwrap());
        assert_eq!(
            dtab.0[1].target,
            NameTree::Leaf(Path::read("/baz").unwrap())
        );
    }

    #[test]
    fn test_dtab_parse_multiple_consecutive_semicolons() {
        // Multiple consecutive semicolons should be ignored
        let dtab_str = "/foo => /bar;;/baz => /qux;";
        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 2);
        assert_eq!(dtab.0[0].prefix, Path::read("/foo").unwrap());
        assert_eq!(
            dtab.0[0].target,
            NameTree::Leaf(Path::read("/bar").unwrap())
        );
        assert_eq!(dtab.0[1].prefix, Path::read("/baz").unwrap());
        assert_eq!(
            dtab.0[1].target,
            NameTree::Leaf(Path::read("/qux").unwrap())
        );
    }

    #[test]
    fn test_dtab_parse_error_on_missing_arrow() {
        // Missing => should cause a parse error
        let dtab_str = "/foo /bar;";
        let result = Dtab::parse(dtab_str);
        assert!(result.is_err(), "Parser should error if '=>' is missing");
    }

    #[test]
    fn test_dtab_parse_error_on_missing_prefix() {
        // Missing prefix should cause a parse error
        let dtab_str = "=> /bar;";
        let result = Dtab::parse(dtab_str);
        assert!(result.is_err(), "Parser should error if prefix is missing");
    }

    #[test]
    fn test_dtab_parse_error_on_missing_target() {
        // Missing target should cause a parse error
        let dtab_str = "/foo => ;";
        let result = Dtab::parse(dtab_str);
        assert!(result.is_err(), "Parser should error if target is missing");
    }

    #[test]
    fn test_dtab_parse_system_path() {
        // System path parsing
        let dtab_str = "/foo => /$/inet/localhost/8080;";
        let dtab = Dtab::parse(dtab_str).unwrap();
        assert_eq!(dtab.0.len(), 1);
        assert_eq!(dtab.0[0].prefix, Path::read("/foo").unwrap());
        assert_eq!(
            dtab.0[0].target,
            NameTree::Leaf(Path::read("/$/inet/localhost/8080").unwrap())
        );
    }

    #[test]
    fn test_dtab_show_parse_roundtrip() {
        // Show/parse roundtrip
        let dtab_str = "/foo => /bar;/baz => /qux;";
        let dtab = Dtab::parse(dtab_str).unwrap();
        let shown = dtab.show();
        let reparsed = Dtab::parse(&shown).unwrap();
        assert_eq!(dtab, reparsed);
    }

    #[test]
    fn test_dtab_equality_and_hash() {
        // Dtab equality and hash
        let dtab1 = Dtab::parse("/foo => /bar;/baz => /qux;").unwrap();
        let dtab2 = Dtab::parse("/foo => /bar;/baz => /qux;").unwrap();
        assert_eq!(dtab1, dtab2);
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher1 = DefaultHasher::new();
        dtab1.hash(&mut hasher1);
        let mut hasher2 = DefaultHasher::new();
        dtab2.hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_dtab_parse_error_on_invalid_characters() {
        let result = Dtab::parse("/foo^bar => /baz");
        assert!(result.is_err());
    }

    #[test]
    fn test_dtab_collection_behavior_python_equivalent() {
        // Test equivalent to Python's test_is_collection
        let mut dtab = Dtab::empty();
        let dentry1 = Dentry::parse("/a => /b").unwrap();
        let dentry2 = Dentry::parse("/c => /d").unwrap();
        dtab.0.push(dentry1);
        dtab.0.push(dentry2);

        assert_eq!(dtab.0.len(), 2);

        // Test creating dtab from list of dentries
        let dentries = vec![
            Dentry::parse("/a => /b").unwrap(),
            Dentry::parse("/c => /d").unwrap(),
        ];
        let dtab_from_list = Dtab::from_dentries(dentries);
        assert_eq!(dtab_from_list.0.len(), 2);
    }

    #[test]
    fn test_dtab_concat_d1_d2_python_equivalent() {
        // Test equivalent to Python's test_concat_d1_d2
        let d1 = Dtab::parse("/foo => /bar").unwrap();
        let d2 = Dtab::parse("/foo=>/biz;/biz=>/$/inet/0/8080;/bar=>/$/inet/0/9090").unwrap();
        let combined = d1.concat(&d2);

        assert_eq!(combined.0.len(), 4);
        assert!(
            combined
                .0
                .iter()
                .any(|d| d.prefix.show() == "/foo" && d.target.to_string().contains("bar"))
        );
        assert!(
            combined
                .0
                .iter()
                .any(|d| d.prefix.show() == "/foo" && d.target.to_string().contains("biz"))
        );
        assert!(
            combined
                .0
                .iter()
                .any(|d| d.prefix.show() == "/biz" && d.target.to_string().contains("inet"))
        );
        assert!(
            combined
                .0
                .iter()
                .any(|d| d.prefix.show() == "/bar" && d.target.to_string().contains("inet"))
        );
    }

    #[test]
    fn test_dtab_d1_concat_dtab_empty_python_equivalent() {
        // Test equivalent to Python's test_d1_concat_dtab_empty
        let d1 = Dtab::parse("/foo=>/bar;/biz=>/baz").unwrap();
        let empty = Dtab::empty();
        let result = d1.concat(&empty);

        assert_eq!(result, d1);
    }

    #[test]
    fn test_dtab_allows_trailing_semicolon_python_equivalent() {
        // Test equivalent to Python's test_allows_trailing_semicolon
        let dtab = Dtab::parse("/b => /c;/a => /b;").unwrap();
        assert_eq!(dtab.0.len(), 2);
    }

    #[test]
    fn test_dtab_rewrites_with_wildcard_python_equivalent() {
        // Test equivalent to Python's test_dtab_rewrites_with_wildcard
        let dtab = Dtab::parse("/a/*/c => /d").unwrap();
        let path = Path::read("/a/b/c/e/f").unwrap();
        let result = dtab.lookup(&path).unwrap();

        // Should rewrite to /d/e/f
        assert_eq!(result, NameTree::Leaf(Path::read("/d/e/f").unwrap()));
    }

    #[test]
    fn test_dtab_lookup_simple_python_equivalent() {
        // Test equivalent to Python's test_lookup_simple
        let dtab = Dtab::parse("/zk# => /$/com.twitter.serverset;/zk => /zk#;/s## => /zk/zk.local.twitter.com:2181;/s# => /s##/prod;/s => /s#").unwrap();

        let two = dtab.lookup(&Path::read("/s/crawler").unwrap()).unwrap();
        assert_eq!(two, NameTree::Leaf(Path::read("/s#/crawler").unwrap()));

        let three = dtab.lookup(&Path::read("/s#/crawler").unwrap()).unwrap();
        assert_eq!(
            three,
            NameTree::Leaf(Path::read("/s##/prod/crawler").unwrap())
        );

        let four = dtab
            .lookup(&Path::read("/s##/prod/crawler").unwrap())
            .unwrap();
        assert_eq!(
            four,
            NameTree::Leaf(Path::read("/zk/zk.local.twitter.com:2181/prod/crawler").unwrap())
        );

        let five = dtab
            .lookup(&Path::read("/zk/zk.local.twitter.com:2181/prod/crawler").unwrap())
            .unwrap();
        assert_eq!(
            five,
            NameTree::Leaf(Path::read("/zk#/zk.local.twitter.com:2181/prod/crawler").unwrap())
        );

        let six = dtab
            .lookup(&Path::read("/zk#/zk.local.twitter.com:2181/prod/crawler").unwrap())
            .unwrap();
        assert_eq!(
            six,
            NameTree::Leaf(
                Path::read("/$/com.twitter.serverset/zk.local.twitter.com:2181/prod/crawler")
                    .unwrap()
            )
        );
    }

    #[test]
    fn test_dtab_lookup_alternate_python_equivalent() {
        // Test equivalent to Python's test_lookup_alternate
        let dtab = Dtab::parse("/zk# => /$/com.twitter.serverset;/zk => /zk#;/s## => /zk/zk.local.twitter.com:2181;/s# => /s##/prod;/s => /s#;/s# => /s##/staging").unwrap();

        let two = dtab.lookup(&Path::read("/s/crawler").unwrap()).unwrap();
        assert_eq!(two, NameTree::Leaf(Path::read("/s#/crawler").unwrap()));

        let alternates = dtab.lookup(&Path::read("/s#/crawler").unwrap()).unwrap();

        // Should return alternatives for the multiple /s# matches
        match alternates {
            NameTree::Alt(alternatives) => {
                assert_eq!(alternatives.len(), 2);
                // Check that both prod and staging alternatives are present
                let alt_strings: Vec<String> = alternatives.iter().map(|a| a.to_string()).collect();
                assert!(alt_strings.iter().any(|s| s.contains("prod")));
                assert!(alt_strings.iter().any(|s| s.contains("staging")));
            }
            _ => panic!("Expected Alt variant"),
        }
    }
}
