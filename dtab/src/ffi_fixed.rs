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
        
        let rust_path = Path(elements.into_iter().map(|s| {
            if s == "*" {
                crate::path::PathElem::Wildcard
            } else {
                crate::path::PathElem::Label(s)
            }
        }).collect());

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
                let prefix = Path(prefix_elements.into_iter().map(|s| {
                    if s == "*" {
                        crate::path::PathElem::Wildcard
                    } else {
                        crate::path::PathElem::Label(s)
                    }
                }).collect());

                // Convert target from C structure to Rust NameTree
                let target = if !(*c_dentry.target).is_null() {
                    let c_target = &**(*c_dentry.target);
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