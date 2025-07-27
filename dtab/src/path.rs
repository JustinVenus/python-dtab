use crate::error::DtabError;
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathElem {
    Label(String),
    Wildcard,
}

impl fmt::Display for PathElem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathElem::Label(label) => write!(f, "{label}"),
            PathElem::Wildcard => write!(f, "*"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(pub Vec<PathElem>);

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "/")
        } else {
            let elements: Vec<String> = self.0.iter().map(|e| e.to_string()).collect();
            write!(f, "/{}", elements.join("/"))
        }
    }
}

impl Path {
    pub fn empty() -> Path {
        Path(vec![])
    }

    pub fn from_utf8(elements: &[&str]) -> Path {
        Path(
            elements
                .iter()
                .map(|s| PathElem::Label(s.to_string()))
                .collect(),
        )
    }

    pub fn read(s: &str) -> Result<Self, DtabError> {
        let s = s.trim();
        if !s.starts_with('/') {
            return Err(DtabError::InvalidPath("Path must start with /".to_string()));
        }

        if s == "/" {
            return Ok(Path::empty());
        }

        // Split on / and filter out empty strings
        let elements: Vec<&str> = s.split('/').filter(|&x| !x.is_empty()).collect();
        let mut path_elements = Vec::new();

        for elem in elements {
            if elem == "*" {
                path_elements.push(PathElem::Wildcard);
            } else {
                path_elements.push(PathElem::Label(elem.to_string()));
            }
        }

        Ok(Path(path_elements))
    }

    pub fn show(&self) -> String {
        if self.0.is_empty() {
            "/".to_string()
        } else {
            let mut result = String::new();
            result.push('/');

            for (i, elem) in self.0.iter().enumerate() {
                if i > 0 {
                    result.push('/');
                }
                match elem {
                    PathElem::Label(s) => result.push_str(s),
                    PathElem::Wildcard => result.push('*'),
                }
            }
            result
        }
    }

    pub fn matches(&self, other: &Path) -> bool {
        println!("DEBUG: matching {} against {}", self.show(), other.show());
        // Check if this path is a prefix of the other path
        if self.0.len() > other.0.len() {
            println!(
                "DEBUG: self len {} > other len {}, returning false",
                self.0.len(),
                other.0.len()
            );
            return false;
        }

        for (i, self_elem) in self.0.iter().enumerate() {
            let other_elem = &other.0[i];
            println!("DEBUG: comparing {self_elem} with {other_elem}");
            match (self_elem, other_elem) {
                (PathElem::Wildcard, _) => {
                    println!("DEBUG: wildcard matches anything");
                    continue;
                }
                (PathElem::Label(s1), PathElem::Label(s2)) if s1 == s2 => {
                    println!("DEBUG: labels match");
                    continue;
                }
                _ => {
                    println!("DEBUG: no match, returning false");
                    return false;
                }
            }
        }
        println!("DEBUG: all elements matched, returning true");
        true
    }

    /// Get the number of elements in the path
    pub fn size(&self) -> usize {
        self.0.len()
    }

    /// Take the first n elements of the path
    pub fn take(&self, n: usize) -> Path {
        let n = n.min(self.0.len());
        Path(self.0[..n].to_vec())
    }

    /// Drop the first n elements and return the remainder
    pub fn drop(&self, n: usize) -> Path {
        let n = n.min(self.0.len());
        Path(self.0[n..].to_vec())
    }

    /// Concatenate this path with another path
    pub fn concat(&self, other: &Path) -> Path {
        let mut combined = Vec::with_capacity(self.0.len() + other.0.len());
        combined.extend_from_slice(&self.0);
        combined.extend_from_slice(&other.0);
        Path(combined)
    }

    /// Add a suffix to this path
    pub fn add_suffix(&self, suffix: &Path) -> Path {
        self.concat(suffix)
    }
}
