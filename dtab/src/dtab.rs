use crate::error::DtabError;
use crate::nametree::{NameTree, Weighted};
use crate::path::{Path, PathElem};
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dentry {
    pub prefix: Path,
    pub target: NameTree,
}

impl fmt::Display for Dentry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} => {}", self.prefix, self.target)
    }
}

impl Dentry {
    pub fn parse(s: &str) -> Result<Self, DtabError> {
        let mut parser = DentryParser::new(s);
        parser.parse_dentry()
    }
}

struct DentryParser {
    input: &'static str,
    index: usize,
}

impl DentryParser {
    fn new(input: &str) -> Self {
        // Convert to static lifetime for simplicity
        let input = Box::leak(input.to_string().into_boxed_str());
        DentryParser { input, index: 0 }
    }

    fn peek_char(&self) -> Option<char> {
        self.input.chars().nth(self.index)
    }

    fn consume_char(&mut self) -> Option<char> {
        let c = self.peek_char();
        if c.is_some() {
            self.index += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.consume_char();
            } else if c == '#' {
                // Skip comments: everything from # to end of line
                while let Some(c) = self.peek_char() {
                    if c == '\n' || c == '\r' {
                        break;
                    }
                    self.consume_char();
                }
            } else {
                break;
            }
        }
    }

    fn eat_char(&mut self, expected: char) -> Result<(), DtabError> {
        match self.peek_char() {
            Some(c) if c == expected => {
                self.consume_char();
                Ok(())
            }
            Some(c) => Err(DtabError::ParseError(format!(
                "Expected '{expected}', found '{c}'"
            ))),
            None => Err(DtabError::ParseError(format!(
                "Expected '{expected}', found end of input"
            ))),
        }
    }

    fn parse_dentry(&mut self) -> Result<Dentry, DtabError> {
        // Parse prefix
        let prefix = self.parse_dentry_prefix()?;

        // Eat whitespace
        self.skip_whitespace();

        // Eat "=>"
        self.eat_char('=')?;
        self.eat_char('>')?;

        // Parse the rest as a NameTree using the existing NameTree parser
        // The NameTree parser should handle comments correctly
        let remaining = &self.input[self.index..];
        let target = NameTree::parse(remaining)?;

        Ok(Dentry { prefix, target })
    }

    fn parse_dentry_prefix(&mut self) -> Result<Path, DtabError> {
        self.skip_whitespace();

        // Find the end of the prefix (before the =>)
        let start_index = self.index;
        let mut paren_count = 0;

        while let Some(c) = self.peek_char() {
            match c {
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                '=' => {
                    if paren_count == 0
                        && self.index + 1 < self.input.len()
                        && self.input.chars().nth(self.index + 1) == Some('>')
                    {
                        break;
                    }
                }
                _ => {}
            }
            self.consume_char();
        }

        // Extract the prefix string and parse it with Path::read
        let prefix_str = &self.input[start_index..self.index];
        println!("DEBUG: extracted prefix string: '{prefix_str}'");
        Path::read(prefix_str.trim())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dtab(pub Vec<Dentry>);

impl fmt::Display for Dtab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(|d| d.to_string()).collect();
        write!(f, "{}", parts.join("; "))
    }
}

impl Dtab {
    pub fn parse(s: &str) -> Result<Self, DtabError> {
        println!("DEBUG: raw input: '{s}'");
        let mut entries = Vec::new();
        let mut parser = DtabParser::new(s);
        parser.parse_dtab(&mut entries)?;
        Ok(Dtab(entries))
    }

    pub fn show(&self) -> String {
        self.to_string()
    }

    /// Returns the debug string representation for FFI compatibility
    pub fn show_debug(&self) -> String {
        let parts: Vec<String> = self
            .0
            .iter()
            .map(|d| format!("{} => {}", d.prefix, d.target.to_string_debug()))
            .collect();
        parts.join("; ")
    }

    pub fn concat(&self, other: &Dtab) -> Dtab {
        let mut combined = self.0.clone();
        combined.extend(other.0.clone());
        Dtab(combined)
    }

    pub fn lookup(&self, path: &Path) -> Result<NameTree, DtabError> {
        let mut matches = Vec::new();

        // Check each dentry for matches
        for entry in &self.0 {
            println!(
                "DEBUG: checking dentry {} => {} against path {}",
                entry.prefix, entry.target, path
            );
            if entry.prefix.matches(path) {
                println!("DEBUG: found match!");
                // Calculate the suffix (remaining path elements after the prefix)
                let suffix = path.drop(entry.prefix.size());
                println!("DEBUG: suffix is {suffix}");

                // Apply path rewriting: add suffix to each leaf in the target NameTree
                let rewritten_tree = entry
                    .target
                    .map(|target_path| target_path.add_suffix(&suffix));

                matches.push(rewritten_tree);
            } else {
                println!("DEBUG: no match");
            }
        }

        // Return appropriate result based on number of matches
        match matches.len() {
            0 => Err(DtabError::LookupError(
                "No matching entry found".to_string(),
            )),
            1 => Ok(matches.remove(0)),
            _ => Ok(NameTree::Alt(matches)),
        }
    }

    pub fn empty() -> Self {
        Dtab(Vec::new())
    }

    pub fn from_dentries(dentries: Vec<Dentry>) -> Self {
        Dtab(dentries)
    }
}

struct DtabParser {
    input: Vec<char>,
    pos: usize,
}

impl DtabParser {
    fn new(input: &str) -> Self {
        DtabParser {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn parse_dtab(&mut self, entries: &mut Vec<Dentry>) -> Result<(), DtabError> {
        loop {
            self.eat_whitespace();
            if self.at_end() {
                break;
            }
            let dentry = self.parse_dentry()?;
            entries.push(dentry);
            self.eat_whitespace();
            if self.at_end() {
                break;
            }
            // Check if we have a semicolon (optional between dentries)
            if self.peek() == ';' {
                // Consume all consecutive semicolons
                while self.peek() == ';' {
                    self.next();
                }
            }
            // If no semicolon, continue to next dentry (more lenient approach)
        }
        Ok(())
    }

    fn parse_dentry(&mut self) -> Result<Dentry, DtabError> {
        println!("DEBUG: parse_dentry starting at pos {}", self.pos);
        let prefix = self.parse_dentry_prefix()?;

        self.eat_whitespace();
        self.eat('=')?;
        self.eat('>')?;

        // Parse the target tree using the standard parse_tree method
        let tree = self.parse_tree()?;

        // Check if target is empty (like NameTree::Empty)
        if matches!(tree, NameTree::Empty) {
            return Err(DtabError::ParseError(
                "Empty target not allowed".to_string(),
            ));
        }

        println!("DEBUG: parse_dentry finished at pos {}", self.pos);
        Ok(Dentry {
            prefix,
            target: tree,
        })
    }

    fn parse_dentry_prefix(&mut self) -> Result<Path, DtabError> {
        self.eat_whitespace();
        self.eat('/')?;

        if !self.is_dentry_prefix_elem_char(self.peek()) {
            return Ok(Path::empty());
        }

        let mut elems = Vec::new();
        loop {
            elems.push(self.parse_dentry_prefix_elem()?);
            if !self.maybe_eat('/') {
                break;
            }
        }

        let path_elements: Vec<PathElem> = elems
            .into_iter()
            .map(|s| {
                if s == "*" {
                    PathElem::Wildcard
                } else {
                    PathElem::Label(s)
                }
            })
            .collect();

        println!("DEBUG: parsed path elements: {path_elements:?}");
        Ok(Path(path_elements))
    }

    fn parse_dentry_prefix_elem(&mut self) -> Result<String, DtabError> {
        if self.peek() == '*' {
            self.next();
            return Ok("*".to_string());
        }
        let result = self.parse_label()?;
        println!("DEBUG: parsed label: '{result}'");
        Ok(result)
    }

    fn parse_label(&mut self) -> Result<String, DtabError> {
        let mut result = String::new();
        loop {
            let c = self.peek();
            if self.is_label_char(c) && c != '/' {
                self.next();
                result.push(c);
            } else {
                break;
            }
        }
        Ok(result)
    }

    fn parse_tree(&mut self) -> Result<NameTree, DtabError> {
        let mut trees = Vec::new();
        loop {
            trees.push(self.parse_tree1()?);
            self.eat_whitespace();
            if !self.maybe_eat('|') {
                break;
            }
        }

        if trees.len() > 1 {
            Ok(NameTree::Alt(trees))
        } else {
            Ok(trees.remove(0))
        }
    }

    fn parse_tree1(&mut self) -> Result<NameTree, DtabError> {
        let mut weighted_trees = Vec::new();
        loop {
            weighted_trees.push(self.parse_weighted()?);
            self.eat_whitespace();
            if !self.maybe_eat('&') {
                break;
            }
        }

        if weighted_trees.len() > 1 {
            Ok(NameTree::Union(weighted_trees))
        } else {
            let weighted = weighted_trees.remove(0);
            if weighted.weight == 1.0 {
                Ok(*weighted.tree)
            } else {
                Ok(NameTree::Union(vec![weighted]))
            }
        }
    }

    fn parse_weighted(&mut self) -> Result<Weighted, DtabError> {
        self.eat_whitespace();

        let weight = if !self.is_number_char(self.peek()) {
            1.0 // default weight
        } else {
            let weight = self.parse_number()?;
            self.eat_whitespace();
            self.eat('*')?;
            self.eat_whitespace();
            weight
        };

        let tree = self.parse_simple()?;
        Ok(Weighted::new(weight, tree))
    }

    fn parse_simple(&mut self) -> Result<NameTree, DtabError> {
        self.eat_whitespace();

        match self.peek() {
            '(' => {
                self.next();
                let tree = self.parse_tree()?;
                self.eat_whitespace();
                self.eat(')')?;
                Ok(tree)
            }
            '/' => Ok(NameTree::Leaf(self.parse_path()?)),
            '!' => {
                self.next();
                Ok(NameTree::Fail)
            }
            '~' => {
                self.next();
                Ok(NameTree::Neg)
            }
            '$' => {
                self.next();
                Ok(NameTree::Empty)
            }
            c => Err(DtabError::ParseError(format!("Unexpected character: {c}"))),
        }
    }

    fn parse_path(&mut self) -> Result<Path, DtabError> {
        self.eat_whitespace();
        self.eat('/')?;

        if !self.is_label_char(self.peek()) {
            return Ok(Path::empty());
        }

        let mut labels = Vec::new();
        loop {
            labels.push(self.parse_label()?);
            if !self.maybe_eat('/') {
                break;
            }
        }

        let path_elements: Vec<PathElem> = labels
            .into_iter()
            .map(|s| {
                if s == "*" {
                    PathElem::Wildcard
                } else {
                    PathElem::Label(s)
                }
            })
            .collect();

        Ok(Path(path_elements))
    }

    fn parse_number(&mut self) -> Result<f64, DtabError> {
        let mut result = String::new();
        let mut seen_dot = false;

        while self.is_number_char(self.peek()) {
            let c = self.peek();
            if c == '.' {
                if seen_dot {
                    return Err(DtabError::ParseError("Invalid number".to_string()));
                }
                seen_dot = true;
            }
            result.push(c);
            self.next();
        }

        if result.len() == 1 && result == "." {
            return Err(DtabError::ParseError("Invalid number".to_string()));
        }

        result
            .parse()
            .map_err(|_| DtabError::ParseError("Invalid number".to_string()))
    }

    // Helper methods
    fn peek(&self) -> char {
        if self.at_end() {
            '\0'
        } else {
            self.input[self.pos]
        }
    }

    fn next(&mut self) {
        if !self.at_end() {
            self.pos += 1;
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn eat(&mut self, expected: char) -> Result<(), DtabError> {
        if self.peek() == expected {
            self.next();
            Ok(())
        } else {
            Err(DtabError::ParseError(format!(
                "Expected '{}', found '{}'",
                expected,
                self.peek()
            )))
        }
    }

    fn maybe_eat(&mut self, expected: char) -> bool {
        if self.peek() == expected {
            self.next();
            true
        } else {
            false
        }
    }

    fn eat_whitespace(&mut self) {
        while !self.at_end() && (self.peek().is_whitespace() || self.peek() == '#') {
            if self.peek() == '#' {
                self.eat_line();
            } else {
                self.next();
            }
        }
    }

    fn eat_line(&mut self) {
        while !self.at_end() && self.peek() != '\n' && self.peek() != '\r' {
            self.next();
        }
        if !self.at_end() && (self.peek() == '\n' || self.peek() == '\r') {
            self.next(); // consume newline
        }
    }

    fn is_label_char(&self, c: char) -> bool {
        c.is_alphanumeric()
            || c == '_'
            || c == '-'
            || c == '*'
            || c == '#'
            || c == '$'
            || c == '.'
            || c == ':'
            || c == '/'
    }

    fn is_dentry_prefix_elem_char(&self, c: char) -> bool {
        self.is_label_char(c) || c == '*'
    }

    fn is_number_char(&self, c: char) -> bool {
        c.is_ascii_digit() || c == '.'
    }
}
