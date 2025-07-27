use crate::error::DtabError;
use crate::path::Path;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameTree {
    Leaf(Path),
    Alt(Vec<NameTree>),
    Union(Vec<Weighted>),
    Fail,
    Neg,
    Empty,
}

#[derive(Debug, Clone)]
pub struct Weighted {
    pub weight: f64,
    pub tree: Box<NameTree>,
}

impl Weighted {
    pub fn new(weight: f64, tree: NameTree) -> Self {
        Weighted {
            weight,
            tree: Box::new(tree),
        }
    }
}

impl PartialEq for Weighted {
    fn eq(&self, other: &Self) -> bool {
        // Handle NaN comparison
        if self.weight.is_nan() && other.weight.is_nan() {
            self.tree == other.tree
        } else {
            self.weight == other.weight && self.tree == other.tree
        }
    }
}

impl Eq for Weighted {}

impl Hash for Weighted {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Handle NaN in hash
        if self.weight.is_nan() {
            state.write_u64(0x7FF8000000000000); // NaN bit pattern
        } else {
            self.weight.to_bits().hash(state);
        }
        self.tree.hash(state);
    }
}

impl fmt::Display for NameTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameTree::Leaf(path) => write!(f, "{path}"),
            NameTree::Alt(trees) => {
                let parts: Vec<String> = trees.iter().map(|t| t.to_string()).collect();
                write!(f, "Alt({})", parts.join(", "))
            }
            NameTree::Union(weighted) => {
                let parts: Vec<String> = weighted
                    .iter()
                    .map(|w| format!("{}*{}", w.weight, w.tree))
                    .collect();
                write!(f, "Union({})", parts.join(", "))
            }
            NameTree::Fail => write!(f, "!"),
            NameTree::Neg => write!(f, "~"),
            NameTree::Empty => write!(f, "$"),
        }
    }
}

impl fmt::Display for Weighted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.weight == 1.0 {
            write!(f, "{}", self.tree)
        } else {
            write!(f, "{:.1}*{}", self.weight, self.tree)
        }
    }
}

impl NameTree {
    pub fn parse(input: &str) -> Result<NameTree, DtabError> {
        let mut parser = NameTreeParser::new(input);
        parser.parse_name_tree()
    }

    /// Apply a function to each leaf in the NameTree
    pub fn map<F>(&self, func: F) -> NameTree
    where
        F: Fn(&Path) -> Path + Copy,
    {
        match self {
            NameTree::Leaf(path) => NameTree::Leaf(func(path)),
            NameTree::Alt(trees) => {
                let mut mapped_trees = Vec::with_capacity(trees.len());
                for tree in trees {
                    mapped_trees.push(tree.map(func));
                }
                NameTree::Alt(mapped_trees)
            }
            NameTree::Union(weighted_trees) => {
                let mut mapped_weighted = Vec::with_capacity(weighted_trees.len());
                for weighted in weighted_trees {
                    mapped_weighted.push(Weighted {
                        weight: weighted.weight,
                        tree: Box::new(weighted.tree.map(func)),
                    });
                }
                NameTree::Union(mapped_weighted)
            }
            NameTree::Fail => NameTree::Fail,
            NameTree::Neg => NameTree::Neg,
            NameTree::Empty => NameTree::Empty,
        }
    }

    /// Returns the toString representation (like Scala) for FFI compatibility
    pub fn to_string_debug(&self) -> String {
        match self {
            NameTree::Leaf(path) => path.to_string(),
            NameTree::Alt(trees) => {
                let parts: Vec<String> = trees.iter().map(|t| t.to_string_debug()).collect();
                format!("Alt({})", parts.join(", "))
            }
            NameTree::Union(weighted) => {
                let parts: Vec<String> = weighted
                    .iter()
                    .map(|w| format!("{}*{}", w.weight, w.tree.to_string_debug()))
                    .collect();
                format!("Union({})", parts.join(", "))
            }
            NameTree::Fail => "Fail".to_string(),
            NameTree::Neg => "Neg".to_string(),
            NameTree::Empty => "Empty".to_string(),
        }
    }
}

struct NameTreeParser {
    input: Vec<char>,
    pos: usize,
}

impl NameTreeParser {
    fn new(input: &str) -> Self {
        let input: Vec<char> = input.chars().collect();
        NameTreeParser { input, pos: 0 }
    }

    fn parse_name_tree(&mut self) -> Result<NameTree, DtabError> {
        self.skip_whitespace();
        let tree = self.parse_alt()?;
        self.skip_whitespace();
        if self.pos < self.input.len() {
            return Err(DtabError::ParseError(format!(
                "Unexpected input at position {}",
                self.pos
            )));
        }
        Ok(tree)
    }

    fn parse_alt(&mut self) -> Result<NameTree, DtabError> {
        let mut trees = Vec::new();
        trees.push(self.parse_union()?);

        while {
            self.skip_whitespace();
            let next_char = self.peek_char();
            next_char == Some('|')
        } {
            self.consume_char(); // consume '|'
            self.skip_whitespace();
            trees.push(self.parse_union()?);
        }

        if trees.len() == 1 {
            Ok(trees.remove(0))
        } else {
            Ok(NameTree::Alt(trees))
        }
    }

    fn parse_union(&mut self) -> Result<NameTree, DtabError> {
        let mut weighted_trees = Vec::new();
        weighted_trees.push(self.parse_weighted()?);

        while {
            self.skip_whitespace();
            self.peek_char() == Some('&')
        } {
            self.consume_char(); // consume '&'
            self.skip_whitespace();
            weighted_trees.push(self.parse_weighted()?);
        }

        if weighted_trees.len() == 1 {
            // If there's only one weighted tree and it has default weight, unwrap it
            let weighted = weighted_trees.remove(0);
            if weighted.weight == 1.0 {
                Ok(*weighted.tree)
            } else {
                Ok(NameTree::Union(vec![weighted]))
            }
        } else {
            Ok(NameTree::Union(weighted_trees))
        }
    }

    fn parse_weighted(&mut self) -> Result<Weighted, DtabError> {
        self.skip_whitespace();

        let weight = if self.is_digit_or_dot() {
            let weight = self.parse_float()?;
            self.skip_whitespace();
            if self.peek_char() == Some('*') {
                self.consume_char(); // consume '*'
                self.skip_whitespace();
            } else {
                return Err(DtabError::ParseError(
                    "Expected '*' after weight".to_string(),
                ));
            }
            weight
        } else {
            1.0 // default weight
        };

        let tree = self.parse_simple()?;
        Ok(Weighted::new(weight, tree))
    }

    fn parse_simple(&mut self) -> Result<NameTree, DtabError> {
        self.skip_whitespace();

        match self.peek_char() {
            Some('!') => {
                self.consume_char();
                Ok(NameTree::Fail)
            }
            Some('~') => {
                self.consume_char();
                Ok(NameTree::Neg)
            }
            Some('$') => {
                // Check if this is part of a path (like /$) or standalone $
                if self.pos > 0 && self.input[self.pos - 1] == '/' {
                    // This is part of a path, not standalone $
                    let path_str = self.parse_path_string()?;
                    match Path::read(&path_str) {
                        Ok(path) => Ok(NameTree::Leaf(path)),
                        Err(_) => Err(DtabError::ParseError(format!("Invalid path: {path_str}"))),
                    }
                } else {
                    // Standalone $ token
                    self.consume_char();
                    Ok(NameTree::Empty)
                }
            }
            Some('(') => {
                self.consume_char(); // consume '('
                self.skip_whitespace();
                let tree = self.parse_alt()?;
                self.skip_whitespace();
                if self.peek_char() == Some(')') {
                    self.consume_char(); // consume ')'
                    Ok(tree)
                } else {
                    Err(DtabError::ParseError("Expected ')'".to_string()))
                }
            }
            Some('/') => {
                let path_str = self.parse_path_string()?;
                match Path::read(&path_str) {
                    Ok(path) => Ok(NameTree::Leaf(path)),
                    Err(_) => Err(DtabError::ParseError(format!("Invalid path: {path_str}"))),
                }
            }
            Some(c) => Err(DtabError::ParseError(format!("Unexpected character: {c}"))),
            None => Err(DtabError::ParseError("Unexpected end of input".to_string())),
        }
    }

    fn parse_path_string(&mut self) -> Result<String, DtabError> {
        let mut path = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == '*'
                || c == '#'
                || c == '/'
                || c == '$'
                || c == '.'
                || c == ':'
            {
                path.push(self.consume_char().unwrap());
            } else {
                break;
            }
        }
        if path.is_empty() {
            Err(DtabError::ParseError("Empty path".to_string()))
        } else {
            Ok(path)
        }
    }

    fn parse_float(&mut self) -> Result<f64, DtabError> {
        let mut num_str = String::new();

        // Parse optional leading dot
        if self.peek_char() == Some('.') {
            num_str.push(self.consume_char().unwrap());
        }

        // Parse digits
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                num_str.push(self.consume_char().unwrap());
            } else {
                break;
            }
        }

        // Parse optional decimal part
        if self.peek_char() == Some('.') {
            num_str.push(self.consume_char().unwrap());
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() {
                    num_str.push(self.consume_char().unwrap());
                } else {
                    break;
                }
            }
        }

        num_str
            .parse::<f64>()
            .map_err(|_| DtabError::ParseError(format!("Invalid number: {num_str}")))
    }

    fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.consume_char();
                skipped = true;
            } else if c == '#' {
                // Skip comments: consume everything from # to end of line
                self.consume_char(); // consume the '#'
                while let Some(c) = self.peek_char() {
                    if c == '\n' || c == '\r' {
                        break;
                    }
                    self.consume_char();
                }
                skipped = true;
            } else {
                break;
            }
        }
        skipped
    }

    fn peek_char(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn consume_char(&mut self) -> Option<char> {
        if self.pos < self.input.len() {
            let c = self.input[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    fn is_digit_or_dot(&self) -> bool {
        if let Some(c) = self.peek_char() {
            c.is_ascii_digit() || c == '.'
        } else {
            false
        }
    }
}
