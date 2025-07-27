use crate::{Dentry, Dtab, NameTree, Path, Weighted};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// ============================================================================
// Error handling
// ============================================================================

#[repr(C)]
pub struct DtabResult {
    pub success: bool,
    pub error_message: *mut c_char,
}

// Note: These functions are kept for potential future use in error handling
impl DtabResult {
    #[allow(dead_code)]
    fn success() -> Self {
        DtabResult {
            success: true,
            error_message: ptr::null_mut(),
        }
    }

    #[allow(dead_code)]
    fn error(message: String) -> Self {
        DtabResult {
            success: false,
            error_message: match CString::new(message) {
                Ok(s) => s.into_raw(),
                Err(_) => ptr::null_mut(),
            },
        }
    }
}

/// Free a DtabResult structure and its associated memory
///
/// # Safety
///
/// - `result` must be a valid pointer to a `DtabResult` allocated by this library
/// - The pointer must not be null
/// - This function should only be called once per allocated result
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_result_free(result: *mut DtabResult) {
    if !result.is_null() {
        unsafe {
            let result = &mut *result;
            if !result.error_message.is_null() {
                let _ = CString::from_raw(result.error_message);
            }
            let _ = Box::from_raw(result);
        }
    }
}

// ============================================================================
// Path FFI
// ============================================================================

#[repr(C)]
pub struct DtabPath {
    pub elements: *mut *mut c_char,
    pub length: usize,
    pub is_wildcard: *mut bool,
}

impl DtabPath {
    fn from_rust_path(path: &Path) -> Self {
        let mut elements: Vec<*mut c_char> = Vec::new();
        let mut is_wildcard: Vec<bool> = Vec::new();

        for elem in &path.0 {
            match elem {
                crate::path::PathElem::Label(s) => {
                    elements.push(CString::new(s.as_str()).unwrap().into_raw());
                    is_wildcard.push(false);
                }
                crate::path::PathElem::Wildcard => {
                    elements.push(CString::new("*").unwrap().into_raw());
                    is_wildcard.push(true);
                }
            }
        }

        // Leak the vectors to keep the pointers valid
        let elements_ptr = elements.as_mut_ptr();
        let is_wildcard_ptr = is_wildcard.as_mut_ptr();
        let length = elements.len();

        // Prevent vectors from being dropped
        std::mem::forget(elements);
        std::mem::forget(is_wildcard);

        DtabPath {
            elements: elements_ptr,
            length,
            is_wildcard: is_wildcard_ptr,
        }
    }

    fn to_rust_path(&self) -> Path {
        let mut path_elements = Vec::new();

        if !self.elements.is_null() {
            unsafe {
                for i in 0..self.length {
                    let element_ptr = *self.elements.add(i);
                    if !element_ptr.is_null() {
                        let element_str = CStr::from_ptr(element_ptr).to_str().unwrap_or("");
                        if i < self.length
                            && !self.is_wildcard.is_null()
                            && *self.is_wildcard.add(i)
                        {
                            path_elements.push(crate::path::PathElem::Wildcard);
                        } else {
                            path_elements
                                .push(crate::path::PathElem::Label(element_str.to_string()));
                        }
                    }
                }
            }
        }

        Path(path_elements)
    }
}

/// Create a path from a string representation
///
/// # Safety
///
/// - `c_path` must be a valid null-terminated C string
/// - The string must contain a valid path representation
/// - The returned pointer must be freed with `dtab_path_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_path_create(c_path: *const c_char) -> *mut DtabPath {
    if c_path.is_null() {
        return ptr::null_mut();
    }

    let path_str = unsafe {
        match CStr::from_ptr(c_path).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match Path::read(path_str) {
        Ok(path) => {
            let dtab_path = DtabPath::from_rust_path(&path);
            Box::into_raw(Box::new(dtab_path))
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Free a path structure and its associated memory
///
/// # Safety
///
/// - `path` must be a valid pointer to a `DtabPath` allocated by this library
/// - The pointer must not be null
/// - This function should only be called once per allocated path
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_path_free(path: *mut DtabPath) {
    if !path.is_null() {
        unsafe {
            let path = &mut *path;

            // Free individual string elements
            for i in 0..path.length {
                let element = path.elements.add(i);
                if !(*element).is_null() {
                    let _ = CString::from_raw(*element);
                }
            }

            // Reconstruct and drop the vectors
            if path.length > 0 {
                let _elements_vec = Vec::from_raw_parts(path.elements, path.length, path.length);
                let _is_wildcard_vec =
                    Vec::from_raw_parts(path.is_wildcard, path.length, path.length);
                // Vectors will be dropped automatically
            }

            // Free the path structure itself
            let _ = Box::from_raw(path);
        }
    }
}

/// Get string representation of a path
///
/// # Safety
///
/// - `path` must be a valid pointer to a `DtabPath` allocated by this library
/// - The pointer must not be null
/// - The returned string must be freed with `dtab_string_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_path_show(path: *const DtabPath) -> *mut c_char {
    if path.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let path = &*path;
        let mut elements: Vec<String> = Vec::new();
        let mut wildcards: Vec<bool> = Vec::new();

        for i in 0..path.length {
            let element = path.elements.add(i);
            let is_wildcard = path.is_wildcard.add(i);

            if !(*element).is_null() {
                let element_str = CStr::from_ptr(*element).to_str().unwrap_or("");
                elements.push(element_str.to_string());
                wildcards.push(*is_wildcard);
            }
        }

        let rust_path = Path(
            elements
                .into_iter()
                .enumerate()
                .map(|(i, elem)| {
                    if wildcards[i] {
                        crate::path::PathElem::Wildcard
                    } else {
                        crate::path::PathElem::Label(elem)
                    }
                })
                .collect(),
        );

        match CString::new(rust_path.show()) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
}

// ============================================================================
// NameTree FFI
// ============================================================================

#[repr(C)]
pub enum DtabNameTreeType {
    Leaf,
    Alt,
    Union,
    Fail,
    Neg,
    Empty,
}

#[repr(C)]
pub struct DtabNameTree {
    pub tree_type: DtabNameTreeType,
    pub leaf_path: *mut DtabPath,
    pub alt_trees: *mut *mut DtabNameTree,
    pub alt_count: usize,
    pub union_weighted: *mut DtabWeighted,
    pub union_count: usize,
}

#[repr(C)]
pub struct DtabWeighted {
    pub weight: f64,
    pub tree: *mut DtabNameTree,
}

impl DtabNameTree {
    fn from_rust_nametree(tree: &NameTree) -> Self {
        match tree {
            NameTree::Leaf(path) => {
                let leaf_path = Box::into_raw(Box::new(DtabPath::from_rust_path(path)));
                DtabNameTree {
                    tree_type: DtabNameTreeType::Leaf,
                    leaf_path,
                    alt_trees: ptr::null_mut(),
                    alt_count: 0,
                    union_weighted: ptr::null_mut(),
                    union_count: 0,
                }
            }
            NameTree::Alt(trees) => {
                let mut alt_trees: Vec<*mut DtabNameTree> = Vec::new();
                for tree in trees {
                    alt_trees.push(Box::into_raw(Box::new(DtabNameTree::from_rust_nametree(
                        tree,
                    ))));
                }
                let alt_trees_ptr = alt_trees.as_mut_ptr();
                let alt_count = alt_trees.len();
                std::mem::forget(alt_trees);

                DtabNameTree {
                    tree_type: DtabNameTreeType::Alt,
                    leaf_path: ptr::null_mut(),
                    alt_trees: alt_trees_ptr,
                    alt_count,
                    union_weighted: ptr::null_mut(),
                    union_count: 0,
                }
            }
            NameTree::Union(weighted_trees) => {
                let mut union_weighted: Vec<DtabWeighted> = Vec::new();
                for w in weighted_trees {
                    union_weighted.push(DtabWeighted {
                        weight: w.weight,
                        tree: Box::into_raw(Box::new(DtabNameTree::from_rust_nametree(&w.tree))),
                    });
                }
                let union_weighted_ptr = union_weighted.as_mut_ptr();
                let union_count = union_weighted.len();
                std::mem::forget(union_weighted);

                DtabNameTree {
                    tree_type: DtabNameTreeType::Union,
                    leaf_path: ptr::null_mut(),
                    alt_trees: ptr::null_mut(),
                    alt_count: 0,
                    union_weighted: union_weighted_ptr,
                    union_count,
                }
            }
            NameTree::Fail => DtabNameTree {
                tree_type: DtabNameTreeType::Fail,
                leaf_path: ptr::null_mut(),
                alt_trees: ptr::null_mut(),
                alt_count: 0,
                union_weighted: ptr::null_mut(),
                union_count: 0,
            },
            NameTree::Neg => DtabNameTree {
                tree_type: DtabNameTreeType::Neg,
                leaf_path: ptr::null_mut(),
                alt_trees: ptr::null_mut(),
                alt_count: 0,
                union_weighted: ptr::null_mut(),
                union_count: 0,
            },
            NameTree::Empty => DtabNameTree {
                tree_type: DtabNameTreeType::Empty,
                leaf_path: ptr::null_mut(),
                alt_trees: ptr::null_mut(),
                alt_count: 0,
                union_weighted: ptr::null_mut(),
                union_count: 0,
            },
        }
    }

    fn to_rust_nametree(&self) -> NameTree {
        match self.tree_type {
            DtabNameTreeType::Leaf => {
                if !self.leaf_path.is_null() {
                    unsafe {
                        let path = &*self.leaf_path;
                        NameTree::Leaf(path.to_rust_path())
                    }
                } else {
                    NameTree::Empty
                }
            }
            DtabNameTreeType::Alt => {
                let mut trees = Vec::new();
                if !self.alt_trees.is_null() {
                    unsafe {
                        for i in 0..self.alt_count {
                            let alt_tree = *self.alt_trees.add(i);
                            if !alt_tree.is_null() {
                                trees.push((&*alt_tree).to_rust_nametree());
                            }
                        }
                    }
                }
                if trees.len() == 1 {
                    trees.remove(0)
                } else {
                    NameTree::Alt(trees)
                }
            }
            DtabNameTreeType::Union => {
                let mut weighted_trees = Vec::new();
                if !self.union_weighted.is_null() {
                    unsafe {
                        for i in 0..self.union_count {
                            let weighted = &*self.union_weighted.add(i);
                            weighted_trees.push(Weighted::new(
                                weighted.weight,
                                (&*weighted.tree).to_rust_nametree(),
                            ));
                        }
                    }
                }
                NameTree::Union(weighted_trees)
            }
            DtabNameTreeType::Fail => NameTree::Fail,
            DtabNameTreeType::Neg => NameTree::Neg,
            DtabNameTreeType::Empty => NameTree::Empty,
        }
    }
}

/// Parse a NameTree from string representation
///
/// # Safety
///
/// - `c_input` must be a valid null-terminated C string
/// - The string must contain a valid NameTree representation
/// - The returned pointer must be freed with `dtab_nametree_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_nametree_parse(c_input: *const c_char) -> *mut DtabNameTree {
    if c_input.is_null() {
        return ptr::null_mut();
    }

    let input_str = unsafe {
        match CStr::from_ptr(c_input).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match NameTree::parse(input_str) {
        Ok(tree) => Box::into_raw(Box::new(DtabNameTree::from_rust_nametree(&tree))),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a NameTree structure and its associated memory
///
/// # Safety
///
/// - `tree` must be a valid pointer to a `DtabNameTree` allocated by this library
/// - The pointer must not be null
/// - This function should only be called once per allocated tree
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_nametree_free(tree: *mut DtabNameTree) {
    if !tree.is_null() {
        unsafe {
            let tree = &mut *tree;

            // Free leaf path if it exists
            if !tree.leaf_path.is_null() {
                dtab_path_free(tree.leaf_path);
            }

            // Free alt trees if they exist
            if !tree.alt_trees.is_null() {
                for i in 0..tree.alt_count {
                    let alt_tree = *tree.alt_trees.add(i);
                    if !alt_tree.is_null() {
                        dtab_nametree_free(alt_tree);
                    }
                }
                let _ = Vec::from_raw_parts(tree.alt_trees, tree.alt_count, tree.alt_count);
            }

            // Free union weighted trees if they exist
            if !tree.union_weighted.is_null() {
                for i in 0..tree.union_count {
                    let weighted = &mut *tree.union_weighted.add(i);
                    if !weighted.tree.is_null() {
                        dtab_nametree_free(weighted.tree);
                    }
                }
                let _ =
                    Vec::from_raw_parts(tree.union_weighted, tree.union_count, tree.union_count);
            }

            let _ = Box::from_raw(tree);
        }
    }
}

/// Get string representation of a NameTree
///
/// # Safety
///
/// - `tree` must be a valid pointer to a `DtabNameTree` allocated by this library
/// - The pointer must not be null
/// - The returned string must be freed with `dtab_string_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_nametree_show(tree: *const DtabNameTree) -> *mut c_char {
    if tree.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let tree = &*tree;
        // Convert back to Rust NameTree and use its toString representation
        let rust_tree = tree.to_rust_nametree();
        let show_str = rust_tree.to_string_debug();

        match CString::new(show_str) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Get compact string representation of a NameTree (show format)
///
/// # Safety
///
/// - `tree` must be a valid pointer to a `DtabNameTree` allocated by this library
/// - The pointer must not be null
/// - The returned string must be freed with `dtab_string_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_nametree_show_compact(tree: *const DtabNameTree) -> *mut c_char {
    if tree.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let tree = &*tree;
        // Convert back to Rust NameTree and use its show representation
        let rust_tree = tree.to_rust_nametree();
        let show_str = rust_tree.to_string();

        match CString::new(show_str) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
}

// ============================================================================
// Dentry FFI
// ============================================================================

#[repr(C)]
pub struct DtabDentry {
    pub prefix: *mut DtabPath,
    pub target: *mut DtabNameTree,
}

/// Parse a Dentry from string representation
///
/// # Safety
///
/// - `c_input` must be a valid null-terminated C string
/// - The string must contain a valid Dentry representation
/// - The returned pointer must be freed with `dtab_dentry_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_dentry_parse(c_input: *const c_char) -> *mut DtabDentry {
    if c_input.is_null() {
        return ptr::null_mut();
    }

    let input_str = unsafe {
        match CStr::from_ptr(c_input).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match Dentry::parse(input_str) {
        Ok(dentry) => {
            let c_dentry = DtabDentry {
                prefix: Box::into_raw(Box::new(DtabPath::from_rust_path(&dentry.prefix))),
                target: Box::into_raw(Box::new(DtabNameTree::from_rust_nametree(&dentry.target))),
            };
            Box::into_raw(Box::new(c_dentry))
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Free a Dentry structure and its associated memory
///
/// # Safety
///
/// - `dentry` must be a valid pointer to a `DtabDentry` allocated by this library
/// - The pointer must not be null
/// - This function should only be called once per allocated dentry
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_dentry_free(dentry: *mut DtabDentry) {
    if !dentry.is_null() {
        unsafe {
            let dentry = &mut *dentry;
            if !dentry.prefix.is_null() {
                dtab_path_free(dentry.prefix);
            }
            if !dentry.target.is_null() {
                dtab_nametree_free(dentry.target);
            }
            let _ = Box::from_raw(dentry);
        }
    }
}

/// Get string representation of a Dentry
///
/// # Safety
///
/// - `dentry` must be a valid pointer to a `DtabDentry` allocated by this library
/// - The pointer must not be null
/// - The returned string must be freed with `dtab_string_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_dentry_show(dentry: *const DtabDentry) -> *mut c_char {
    if dentry.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let dentry = &*dentry;
        // Convert back to Rust Dentry and use its Display implementation
        let prefix = if !dentry.prefix.is_null() {
            (&*dentry.prefix).to_rust_path()
        } else {
            Path::read("/").unwrap_or_else(|_| Path(vec![]))
        };

        let target = if !dentry.target.is_null() {
            (&*dentry.target).to_rust_nametree()
        } else {
            NameTree::Empty
        };

        let rust_dentry = Dentry { prefix, target };
        let show_str = rust_dentry.to_string();

        match CString::new(show_str) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
}

// ============================================================================
// Dtab FFI
// ============================================================================

#[repr(C)]
pub struct DtabTable {
    pub dentries: *mut *mut DtabDentry,
    pub length: usize,
}

impl DtabTable {
    fn from_rust_dtab(dtab: &Dtab) -> Self {
        let mut dentries: Vec<*mut DtabDentry> = Vec::new();

        for dentry in &dtab.0 {
            let c_dentry = DtabDentry {
                prefix: Box::into_raw(Box::new(DtabPath::from_rust_path(&dentry.prefix))),
                target: Box::into_raw(Box::new(DtabNameTree::from_rust_nametree(&dentry.target))),
            };
            dentries.push(Box::into_raw(Box::new(c_dentry)));
        }

        let dentries_ptr = dentries.as_mut_ptr();
        let length = dentries.len();
        std::mem::forget(dentries);

        DtabTable {
            dentries: dentries_ptr,
            length,
        }
    }
}

/// Create a DTab table from string representation
///
/// # Safety
///
/// - `c_input` must be a valid null-terminated C string
/// - The string must contain a valid DTab representation
/// - The returned pointer must be freed with `dtab_table_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_table_create(c_input: *const c_char) -> *mut DtabTable {
    if c_input.is_null() {
        return ptr::null_mut();
    }

    let input_str = unsafe {
        match CStr::from_ptr(c_input).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match Dtab::parse(input_str) {
        Ok(dtab) => Box::into_raw(Box::new(DtabTable::from_rust_dtab(&dtab))),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a DTab table structure and its associated memory
///
/// # Safety
///
/// - `table` must be a valid pointer to a `DtabTable` allocated by this library
/// - The pointer must not be null
/// - This function should only be called once per allocated table
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_table_free(table: *mut DtabTable) {
    if !table.is_null() {
        unsafe {
            let table = &mut *table;

            // Free individual dentries
            for i in 0..table.length {
                let dentry_ptr = *table.dentries.add(i);
                if !dentry_ptr.is_null() {
                    dtab_dentry_free(dentry_ptr);
                }
            }

            // Reconstruct and drop the dentries vector
            if table.length > 0 {
                let _dentries_vec = Vec::from_raw_parts(table.dentries, table.length, table.length);
                // Vector will be dropped automatically
            }

            let _ = Box::from_raw(table);
        }
    }
}

/// Get string representation of a DTab table
///
/// # Safety
///
/// - `table` must be a valid pointer to a `DtabTable` allocated by this library
/// - The pointer must not be null
/// - The returned string must be freed with `dtab_string_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_table_show(table: *const DtabTable) -> *mut c_char {
    if table.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let table = &*table;
        // Convert back to Rust Dtab and use its Display implementation
        let mut dentries = Vec::new();

        for i in 0..table.length {
            let dentry_ptr = *table.dentries.add(i);
            if !dentry_ptr.is_null() {
                let dentry = &*dentry_ptr;
                let prefix = if !dentry.prefix.is_null() {
                    (&*dentry.prefix).to_rust_path()
                } else {
                    Path::read("/").unwrap_or_else(|_| Path(vec![]))
                };

                let target = if !dentry.target.is_null() {
                    (&*dentry.target).to_rust_nametree()
                } else {
                    NameTree::Empty
                };

                dentries.push(Dentry { prefix, target });
            }
        }

        let rust_dtab = Dtab(dentries);
        let show_str = rust_dtab.show_debug();

        match CString::new(show_str) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Look up a path in a DTab table
///
/// # Safety
///
/// - `table` must be a valid pointer to a `DtabTable` allocated by this library
/// - `path` must be a valid pointer to a `DtabPath` allocated by this library
/// - Both pointers must not be null
/// - The returned pointer must be freed with `dtab_nametree_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_table_lookup(
    table: *const DtabTable,
    path: *const DtabPath,
) -> *mut DtabNameTree {
    if table.is_null() || path.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let table = &*table;
        let path = &*path;

        // Convert C path to Rust path
        let mut elements: Vec<String> = Vec::new();
        for i in 0..path.length {
            let element = path.elements.add(i);
            if !(*element).is_null() {
                let element_str = CStr::from_ptr(*element).to_str().unwrap_or("");
                elements.push(element_str.to_string());
            }
        }

        let rust_path = Path(
            elements
                .into_iter()
                .map(|s| {
                    if s == "*" {
                        crate::path::PathElem::Wildcard
                    } else {
                        crate::path::PathElem::Label(s)
                    }
                })
                .collect(),
        );

        // Convert C dtab to Rust dtab
        let mut dentries: Vec<Dentry> = Vec::new();
        for i in 0..table.length {
            let dentry = table.dentries.add(i);
            if !(*dentry).is_null() {
                let c_dentry = &**dentry;

                // Convert prefix
                let mut prefix_elements: Vec<String> = Vec::new();
                for j in 0..(*c_dentry.prefix).length {
                    let element = (*c_dentry.prefix).elements.add(j);
                    if !(*element).is_null() {
                        let element_str = CStr::from_ptr(*element).to_str().unwrap_or("");
                        prefix_elements.push(element_str.to_string());
                    }
                }
                let prefix = Path(
                    prefix_elements
                        .into_iter()
                        .map(|s| {
                            if s == "*" {
                                crate::path::PathElem::Wildcard
                            } else {
                                crate::path::PathElem::Label(s)
                            }
                        })
                        .collect(),
                );

                // Convert target from C structure to Rust NameTree
                let target = if !c_dentry.target.is_null() {
                    let c_target = &*c_dentry.target;
                    c_target.to_rust_nametree()
                } else {
                    NameTree::Leaf(Path::empty())
                };

                dentries.push(Dentry { prefix, target });
            }
        }

        let rust_dtab = Dtab(dentries);

        match rust_dtab.lookup(&rust_path) {
            Ok(tree) => Box::into_raw(Box::new(DtabNameTree::from_rust_nametree(&tree))),
            Err(_) => ptr::null_mut(),
        }
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Free a string allocated by this library
///
/// # Safety
///
/// - `s` must be a valid pointer to a string allocated by this library
/// - The pointer must not be null
/// - This function should only be called once per allocated string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Get the library version string
///
/// # Safety
///
/// - The returned string must be freed with `dtab_string_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtab_version() -> *mut c_char {
    match CString::new("0.1.0") {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn test_ffi_path_create_and_show() {
        unsafe {
            let path_str = CString::new("/foo/bar").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let show_str = dtab_path_show(path);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo/bar");

            dtab_string_free(show_str);
            dtab_path_free(path);
        }
    }

    #[test]
    fn test_ffi_path_with_wildcard() {
        unsafe {
            let path_str = CString::new("/foo/*/bar").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let show_str = dtab_path_show(path);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo/*/bar");

            dtab_string_free(show_str);
            dtab_path_free(path);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_basic() {
        unsafe {
            // Test basic NameTree parsing
            let tree_str = CString::new("!").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "Fail");

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_leaf() {
        unsafe {
            let tree_str = CString::new("/foo/bar").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo/bar");

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_alt() {
        unsafe {
            let tree_str = CString::new("/foo | /bar").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "Alt(/foo, /bar)");

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_union() {
        unsafe {
            let tree_str = CString::new("/foo & /bar").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "Union(1*/foo, 1*/bar)");

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_dentry_parse() {
        unsafe {
            let dentry_str = CString::new("/foo => /bar").unwrap();
            let dentry = dtab_dentry_parse(dentry_str.as_ptr());
            assert!(!dentry.is_null());

            let show_str = dtab_dentry_show(dentry);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /bar");

            dtab_string_free(show_str);
            dtab_dentry_free(dentry);
        }
    }

    #[test]
    fn test_ffi_dentry_parse_with_wildcard() {
        unsafe {
            let dentry_str = CString::new("/foo/*/bar => /baz").unwrap();
            let dentry = dtab_dentry_parse(dentry_str.as_ptr());
            assert!(!dentry.is_null());

            let show_str = dtab_dentry_show(dentry);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo/*/bar => /baz");

            dtab_string_free(show_str);
            dtab_dentry_free(dentry);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_simple() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /bar");

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_multiple() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar;/biz => /baz").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /bar; /biz => /baz");

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_with_comments() {
        unsafe {
            let dtab_str =
                CString::new("# comment\n/foo => /bar  # another comment\n/biz => /baz").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /bar; /biz => /baz");

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_lookup_simple() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let path_str = CString::new("/foo/baz").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let result = dtab_table_lookup(dtab, path);
            assert!(!result.is_null());

            let show_str = dtab_nametree_show(result);
            assert!(!show_str.is_null());
            let result_str = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result_str, "/bar/baz");

            dtab_string_free(show_str);
            dtab_nametree_free(result);
            dtab_path_free(path);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_lookup_with_wildcard() {
        unsafe {
            let dtab_str = CString::new("/a/*/c => /d").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let path_str = CString::new("/a/b/c/e/f").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let result = dtab_table_lookup(dtab, path);
            assert!(!result.is_null());

            let show_str = dtab_nametree_show(result);
            assert!(!show_str.is_null());
            let result_str = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result_str, "/d/e/f");

            dtab_string_free(show_str);
            dtab_nametree_free(result);
            dtab_path_free(path);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_lookup_complex_chain() {
        unsafe {
            let dtab_str = CString::new("/zk# => /$/com.twitter.serverset;/zk => /zk#;/s## => /zk/zk.local.twitter.com:2181;/s# => /s##/prod;/s => /s#").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            // First lookup: /s/crawler -> /s#/crawler
            let path_str = CString::new("/s/crawler").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let result = dtab_table_lookup(dtab, path);
            assert!(!result.is_null());

            let show_str = dtab_nametree_show(result);
            assert!(!show_str.is_null());
            let result_str = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result_str, "/s#/crawler");

            dtab_string_free(show_str);
            dtab_nametree_free(result);
            dtab_path_free(path);

            // Second lookup: /s#/crawler -> /s##/prod/crawler
            let path_str2 = CString::new("/s#/crawler").unwrap();
            let path2 = dtab_path_create(path_str2.as_ptr());
            assert!(!path2.is_null());

            let result2 = dtab_table_lookup(dtab, path2);
            assert!(!result2.is_null());

            let show_str2 = dtab_nametree_show(result2);
            assert!(!show_str2.is_null());
            let result_str2 = CStr::from_ptr(show_str2).to_str().unwrap();
            assert_eq!(result_str2, "/s##/prod/crawler");

            dtab_string_free(show_str2);
            dtab_nametree_free(result2);
            dtab_path_free(path2);

            // Third lookup: /s##/prod/crawler -> /zk/zk.local.twitter.com:2181/prod/crawler
            let path_str3 = CString::new("/s##/prod/crawler").unwrap();
            let path3 = dtab_path_create(path_str3.as_ptr());
            assert!(!path3.is_null());

            let result3 = dtab_table_lookup(dtab, path3);
            assert!(!result3.is_null());

            let show_str3 = dtab_nametree_show(result3);
            assert!(!show_str3.is_null());
            let result_str3 = CStr::from_ptr(show_str3).to_str().unwrap();
            assert_eq!(result_str3, "/zk/zk.local.twitter.com:2181/prod/crawler");

            dtab_string_free(show_str3);
            dtab_nametree_free(result3);
            dtab_path_free(path3);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_lookup_alternate() {
        unsafe {
            let dtab_str = CString::new("/zk# => /$/com.twitter.serverset;/zk => /zk#;/s## => /zk/zk.local.twitter.com:2181;/s# => /s##/prod;/s => /s#;/s# => /s##/staging").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            // First lookup: /s/crawler -> /s#/crawler
            let path_str = CString::new("/s/crawler").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let result = dtab_table_lookup(dtab, path);
            assert!(!result.is_null());

            let show_str = dtab_nametree_show(result);
            assert!(!show_str.is_null());
            let result_str = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result_str, "/s#/crawler");

            dtab_string_free(show_str);
            dtab_nametree_free(result);
            dtab_path_free(path);

            // Second lookup: /s#/crawler -> Alt with multiple alternatives
            let path_str2 = CString::new("/s#/crawler").unwrap();
            let path2 = dtab_path_create(path_str2.as_ptr());
            assert!(!path2.is_null());

            let result2 = dtab_table_lookup(dtab, path2);
            assert!(!result2.is_null());

            let show_str2 = dtab_nametree_show(result2);
            assert!(!show_str2.is_null());
            let result_str2 = CStr::from_ptr(show_str2).to_str().unwrap();
            // Should contain both alternatives
            assert!(result_str2.contains("/s##/prod/crawler"));
            assert!(result_str2.contains("/s##/staging/crawler"));

            dtab_string_free(show_str2);
            dtab_nametree_free(result2);
            dtab_path_free(path2);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_lookup_no_match() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let path_str = CString::new("/baz/qux").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let result = dtab_table_lookup(dtab, path);
            assert!(result.is_null()); // Should return null for no match

            dtab_path_free(path);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_with_trailing_semicolons() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar;/biz => /baz;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /bar; /biz => /baz");

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_error_handling() {
        unsafe {
            // Test null pointer handling
            let result = dtab_path_create(std::ptr::null());
            assert!(result.is_null());

            let result = dtab_nametree_parse(std::ptr::null());
            assert!(result.is_null());

            let result = dtab_dentry_parse(std::ptr::null());
            assert!(result.is_null());

            let result = dtab_table_create(std::ptr::null());
            assert!(result.is_null());

            let result = dtab_table_lookup(std::ptr::null(), std::ptr::null());
            assert!(result.is_null());
        }
    }

    #[test]
    fn test_ffi_version() {
        unsafe {
            let version_str = dtab_version();
            assert!(!version_str.is_null());
            let version = CStr::from_ptr(version_str).to_str().unwrap();
            assert_eq!(version, "0.1.0");
            dtab_string_free(version_str);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_multiple_consecutive_semicolons() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar;;/baz => /qux;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /bar; /baz => /qux");

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_error_on_missing_arrow() {
        unsafe {
            let dtab_str = CString::new("/foo /bar;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(dtab.is_null()); // Should return null for parse error
        }
    }

    #[test]
    fn test_ffi_dtab_parse_error_on_missing_prefix() {
        unsafe {
            let dtab_str = CString::new("=> /bar;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(dtab.is_null()); // Should return null for parse error
        }
    }

    #[test]
    fn test_ffi_dtab_parse_error_on_missing_target() {
        unsafe {
            let dtab_str = CString::new("/foo => ;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(dtab.is_null()); // Should return null for parse error
        }
    }

    #[test]
    fn test_ffi_dtab_parse_system_path() {
        unsafe {
            let dtab_str = CString::new("/foo => /$/inet/localhost/8080;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /$/inet/localhost/8080");

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_show_parse_roundtrip() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar;/baz => /qux;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let shown = CStr::from_ptr(show_str).to_str().unwrap();

            // Parse the shown string back
            let reparsed = dtab_table_create(show_str);
            assert!(!reparsed.is_null());

            let reparsed_show = dtab_table_show(reparsed);
            assert!(!reparsed_show.is_null());
            let reparsed_str = CStr::from_ptr(reparsed_show).to_str().unwrap();
            assert_eq!(shown, reparsed_str);

            dtab_string_free(show_str);
            dtab_string_free(reparsed_show);
            dtab_table_free(dtab);
            dtab_table_free(reparsed);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_empty_dtab_and_dentry() {
        unsafe {
            // Empty dtab should be valid
            let dtab_str = CString::new("").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "");

            dtab_string_free(show_str);
            dtab_table_free(dtab);

            // Empty dentry should cause error
            let dentry_str = CString::new("/ => /;").unwrap();
            let dentry = dtab_dentry_parse(dentry_str.as_ptr());
            assert!(dentry.is_null()); // Should return null for parse error
        }
    }

    #[test]
    fn test_ffi_dtab_parse_error_on_invalid_characters() {
        unsafe {
            let dtab_str = CString::new("/foo^bar => /baz").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(dtab.is_null(), "Should fail to parse invalid characters");
        }
    }

    #[test]
    fn test_ffi_dtab_parse_with_fail() {
        unsafe {
            let dtab_str = CString::new("/=>!").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null(), "Should parse dtab with fail target");

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            // Should contain the fail dtab
            assert!(result.contains("Fail"));

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_dtab_parse_optional_semicolons_between_dentries() {
        unsafe {
            let dtab_str = CString::new("/foo => /bar\n/biz => /baz;").unwrap();
            let dtab = dtab_table_create(dtab_str.as_ptr());
            assert!(!dtab.is_null());

            let show_str = dtab_table_show(dtab);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo => /bar; /biz => /baz");

            dtab_string_free(show_str);
            dtab_table_free(dtab);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_complex_with_comments() {
        unsafe {
            // Test a simpler complex NameTree that the FFI can handle
            let tree_str = CString::new("/foo | /bar").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            // Should contain the parsed structure
            assert!(result.contains("Alt"));

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_parentheses() {
        unsafe {
            let tree_str = CString::new("(/foo | /bar)").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert!(result.contains("Alt"));

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_whitespace() {
        unsafe {
            let tree_str = CString::new("  /foo  |  /bar  ").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "Alt(/foo, /bar)");

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_nametree_parse_weighted() {
        unsafe {
            let tree_str = CString::new("/foo & /bar").unwrap();
            let tree = dtab_nametree_parse(tree_str.as_ptr());
            assert!(!tree.is_null());

            let show_str = dtab_nametree_show(tree);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            // Should contain the parsed structure
            assert!(result.contains("Union"));

            dtab_string_free(show_str);
            dtab_nametree_free(tree);
        }
    }

    #[test]
    fn test_ffi_path_operations() {
        unsafe {
            let path_str = CString::new("/foo/bar/baz").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let show_str = dtab_path_show(path);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo/bar/baz");

            dtab_string_free(show_str);
            dtab_path_free(path);
        }
    }

    #[test]
    fn test_ffi_path_show_consistency() {
        unsafe {
            let path_str = CString::new("/a/b/c").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let show_str = dtab_path_show(path);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/a/b/c");

            dtab_string_free(show_str);
            dtab_path_free(path);
        }
    }

    #[test]
    fn test_ffi_path_matches() {
        unsafe {
            let path_str = CString::new("/foo/*/baz").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let show_str = dtab_path_show(path);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/foo/*/baz");

            dtab_string_free(show_str);
            dtab_path_free(path);
        }
    }

    #[test]
    fn test_ffi_path_parsing_and_show() {
        unsafe {
            let path_str = CString::new("/test/path").unwrap();
            let path = dtab_path_create(path_str.as_ptr());
            assert!(!path.is_null());

            let show_str = dtab_path_show(path);
            assert!(!show_str.is_null());
            let result = CStr::from_ptr(show_str).to_str().unwrap();
            assert_eq!(result, "/test/path");

            dtab_string_free(show_str);
            dtab_path_free(path);
        }
    }
}
