# cython: language_level=3
# distutils: language=c

"""
DTab Cython extension module.
Wraps the C API from dtab.h to provide Python bindings.
"""

cimport cython
from cpython.ref cimport PyObject
from libc.stdlib cimport free, malloc
from libc.string cimport strdup

# Import the C API
cdef extern from "include/dtab.h":
    # Error handling
    ctypedef struct dtab_result_t:
        bint success
        char* error_message
    
    void dtab_result_free(dtab_result_t* result)
    
    # Path API
    ctypedef struct dtab_path_t:
        char** elements
        size_t length
        bint* is_wildcard
    
    dtab_path_t* dtab_path_create(const char* path_str)
    void dtab_path_free(dtab_path_t* path)
    char* dtab_path_show(const dtab_path_t* path)
    
    # NameTree API
    ctypedef enum dtab_nametree_type_t:
        DTAB_NAMETREE_LEAF
        DTAB_NAMETREE_ALT
        DTAB_NAMETREE_UNION
        DTAB_NAMETREE_FAIL
        DTAB_NAMETREE_NEG
        DTAB_NAMETREE_EMPTY
    
    ctypedef struct dtab_weighted_t:
        double weight
        void* tree
    
    ctypedef struct dtab_nametree_t:
        dtab_nametree_type_t tree_type
        dtab_path_t* leaf_path
        dtab_nametree_t** alt_trees
        size_t alt_count
        dtab_weighted_t* union_weighted
        size_t union_count
    
    dtab_nametree_t* dtab_nametree_parse(const char* nametree_str)
    void dtab_nametree_free(dtab_nametree_t* nametree)
    char* dtab_nametree_show(const dtab_nametree_t* nametree)
    char* dtab_nametree_show_compact(const dtab_nametree_t* nametree)
    
    # Dentry API
    ctypedef struct dtab_dentry_t:
        dtab_path_t* prefix
        dtab_nametree_t* target
    
    dtab_dentry_t* dtab_dentry_parse(const char* dentry_str)
    void dtab_dentry_free(dtab_dentry_t* dentry)
    char* dtab_dentry_show(const dtab_dentry_t* dentry)
    
    # Dtab API
    ctypedef struct dtab_table_t:
        dtab_dentry_t** dentries
        size_t length
    
    dtab_table_t* dtab_table_create(const char* dtab_str)
    void dtab_table_free(dtab_table_t* table)
    char* dtab_table_show(const dtab_table_t* table)
    
    # Lookup API
    dtab_nametree_t* dtab_table_lookup(
        const dtab_table_t* table,
        const dtab_path_t* path
    )
    
    # Utility functions
    void dtab_string_free(char* str)
    char* dtab_version()

# Python imports
import sys
from cpython.unicode cimport PyUnicode_FromString

# Exception class
cdef class DtabError(Exception):
    cdef str message
    
    def __init__(self, str message):
        self.message = message
        super().__init__(message)
    
    def __str__(self):
        return self.message

# Path class
cdef class Path:
    cdef dtab_path_t* _path
    cdef list _elems  # Python list for compatibility
    
    def __init__(self, *elems):
        self._path = NULL
        self._elems = []
        for elem in elems:
            self.append(elem)
    
    def __dealloc__(self):
        if self._path != NULL:
            dtab_path_free(self._path)
    
    @classmethod
    def read(cls, str path_str):
        """Read a path from string."""
        cdef Path path = cls()
        cdef bytes path_bytes = path_str.encode('utf-8')
        cdef const char* c_path_str = path_bytes
        
        path._path = dtab_path_create(c_path_str)
        if path._path == NULL:
            raise DtabError("Failed to create path")
        
        # Parse the path string to populate _elems
        if path_str == '/':
            path._elems = []
        else:
            path._elems = path_str.strip('/').split('/')
        
        return path
    
    @classmethod
    def Utf8(cls, *elems):
        """Create a path with UTF-8 elements."""
        return cls(*elems)
    
    @classmethod
    def empty(cls):
        """Get empty path."""
        return cls()
    
    @classmethod
    def is_showable(cls, char_val):
        """Check if character is showable."""
        showable_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_:.#$%-"
        if isinstance(char_val, int):
            char_val = chr(char_val)
        return all(c in showable_chars for c in char_val)
    
    def append(self, value):
        """Append element to path."""
        if isinstance(value, Path):
            self._elems.extend(value.elems)
        else:
            self._elems.append(str(value))
    
    @property
    def elems(self):
        """Get path elements."""
        return self._elems
    
    @property
    def size(self):
        """Get number of elements in path."""
        try:
            return len(self.elems)
        except:
            return 0
    
    def take(self, n):
        """Take first n elements."""
        if hasattr(self, '_elems'):
            return Path(*self._elems[:n])
        return Path()
    
    def drop(self, n):
        """Drop first n elements."""
        if hasattr(self, '_elems'):
            return Path(*self._elems[n:])
        return Path()
    
    def concat(self, other):
        """Concatenate with another path."""
        if hasattr(self, '_elems') and hasattr(other, '_elems'):
            return Path(*(self._elems + other._elems))
        return Path()
    
    def matches(self, other):
        """Check if this path matches another (with wildcard support)."""
        try:
            self_elems = self.elems
            other_elems = other.elems
        except:
            return False
        
        if len(self_elems) != len(other_elems):
            return False
        
        for i, elem in enumerate(self_elems):
            if elem == '*' or elem == other_elems[i]:
                continue
            return False
        return True
    
    def startswith(self, other):
        """Check if path starts with other path."""
        return self.show.startswith(other.show)
    
    @property
    def is_empty(self):
        """Check if path is empty."""
        return self.size == 0
    
    @property
    def show(self):
        """Get string representation of path."""
        if self._path == NULL:
            # Fallback to Python implementation
            return "" if self.is_empty else "/" + "/".join(self._elems)
        
        cdef char* path_str = dtab_path_show(self._path)
        if path_str == NULL:
            raise DtabError("Failed to get path string")
        
        try:
            return path_str.decode('utf-8')
        finally:
            dtab_string_free(path_str)
    
    def __eq__(self, other):
        """Check equality."""
        return str(self) == str(other)
    
    def __ne__(self, other):
        """Check inequality."""
        return not self.__eq__(other)
    
    def __add__(self, other):
        """Concatenate paths."""
        if isinstance(other, self.__class__):
            args = self.elems + other.elems
            return self.__class__(*args)
        if isinstance(other, (list, tuple)):
            args = self.elems + list(other)
            return self.__class__.Utf8(*args)
        return object.__add__(self, other)
    
    def __str__(self):
        """String representation."""
        try:
            return f"Path({self.show()})"
        except:
            return "Path(<error>)"
    
    def __repr__(self):
        """Representation."""
        return self.__str__()

# NameTree class
cdef class NameTree:
    cdef dtab_nametree_t* _nametree
    cdef str _input_str  # Store the original input string for display
    
    def __init__(self, path=None):
        self._nametree = NULL
        self._input_str = ""
        
        if path:
            if isinstance(path, str):
                self._input_str = path
            else:
                self._input_str = str(path)
    
    def __dealloc__(self):
        if self._nametree != NULL:
            dtab_nametree_free(self._nametree)
    
    @classmethod
    def read(cls, str nametree_str):
        """Read a NameTree from string."""
        cdef NameTree nametree = cls()
        nametree._input_str = nametree_str
        
        cdef bytes nametree_bytes = nametree_str.encode('utf-8')
        cdef const char* c_nametree_str = nametree_bytes
        
        nametree._nametree = dtab_nametree_parse(c_nametree_str)
        if nametree._nametree == NULL:
            raise DtabError("Failed to parse NameTree")
        
        return nametree
    
    def lookup(self, path):
        """Lookup a path in the nametree."""
        # For now, return Neg for all lookups
        # This will be implemented using the Rust FFI
        return Neg()
    
    def map(self, func):
        """Map a function over the nametree."""
        # For now, return self
        # This will be implemented using the Rust FFI
        return self
    
    @property
    def show(self):
        """Get string representation of NameTree."""
        if self._nametree == NULL:
            return self._input_str if self._input_str else ""
        
        cdef char* nametree_str = dtab_nametree_show_compact(self._nametree)
        if nametree_str == NULL:
            return "Error"
        
        try:
            return nametree_str.decode('utf-8')
        finally:
            dtab_string_free(nametree_str)
    
    def __repr__(self):
        if self._nametree == NULL:
            return f"NameTree({self._input_str})"
        else:
            return f"NameTree({self.show})"
    
    def __str__(self):
        """String representation."""
        return self.__repr__()
    
    def __eq__(self, other):
        """Check equality based on string representation."""
        if not isinstance(other, NameTree):
            return False
        return self.show == other.show
    
    def __ne__(self, other):
        """Check inequality."""
        return not self.__eq__(other)
    
    def __hash__(self):
        """Hash based on string representation."""
        return hash(self.show)

# NameTree singletons
cdef class Empty(NameTree):
    def show(self):
        return "Empty"
    
    def __str__(self):
        return "NameTree.Empty"

cdef class Fail(NameTree):
    def show(self):
        return "Fail"
    
    def __str__(self):
        return "NameTree.Fail"

cdef class Neg(NameTree):
    def show(self):
        return "Neg"
    
    def __str__(self):
        return "NameTree.Neg"

# Dentry class
cdef class Dentry:
    cdef dtab_dentry_t* _dentry
    cdef object _prefix
    cdef object _nametree
    cdef str _input_str
    
    def __init__(self, prefix, nametree):
        self._dentry = NULL
        self._prefix = prefix
        self._nametree = nametree
        
        # Create a string representation for the dentry
        if prefix and nametree:
            prefix_str = prefix.show if hasattr(prefix, 'show') else str(prefix)
            nametree_str = nametree.show if hasattr(nametree, 'show') else str(nametree)
            self._input_str = f"{prefix_str} => {nametree_str}"
        else:
            self._input_str = ""
    
    def __dealloc__(self):
        if self._dentry != NULL:
            dtab_dentry_free(self._dentry)
    
    @classmethod
    def read(cls, str dentry_str):
        """Read a Dentry from string."""
        cdef Dentry dentry = cls(None, None)
        cdef bytes dentry_bytes = dentry_str.encode('utf-8')
        cdef const char* c_dentry_str = dentry_bytes
        
        dentry._dentry = dtab_dentry_parse(c_dentry_str)
        if dentry._dentry == NULL:
            raise DtabError("Failed to parse Dentry")
        
        return dentry
    
    @property
    def prefix(self):
        """Get prefix."""
        return self._prefix
    
    @property
    def nametree(self):
        """Get nametree."""
        return self._nametree
    
    @property
    def show(self):
        """Get string representation."""
        if self._dentry == NULL:
            result = self._input_str if self._input_str else "Dentry()"
            return result
        
        cdef char* dentry_str = dtab_dentry_show(self._dentry)
        if dentry_str == NULL:
            result = self._input_str if self._input_str else "Dentry()"
            return result
        
        try:
            result = dentry_str.decode('utf-8')
            return result
        finally:
            dtab_string_free(dentry_str)
    
    def __str__(self):
        return f"Dentry({self.show})"
    
    def __eq__(self, other):
        """Check equality."""
        if not isinstance(other, Dentry):
            return False
        return str(self) == str(other)
    
    def __ne__(self, other):
        """Check inequality."""
        return not self.__eq__(other)
    
    def __hash__(self):
        """Hash based on string representation."""
        return hash(str(self))
    
    def __repr__(self):
        return self.__str__()

# Dtab class
cdef class Dtab:
    cdef dtab_table_t* _dtab
    cdef str _input_str  # Store the original input string for display
    
    # Declare variables for lookup method
    cdef dtab_path_t* _lookup_path
    cdef dtab_nametree_t* _lookup_result
    cdef NameTree _lookup_nametree
    
    def __init__(self, dtab_input=None):
        self._dtab = NULL
        self._input_str = ""
        self._lookup_path = NULL
        self._lookup_result = NULL
        self._lookup_nametree = None
        
        if dtab_input:
            if isinstance(dtab_input, str):
                self._input_str = dtab_input
            elif isinstance(dtab_input, (list, tuple)):
                # Convert list of dentries to string representation
                dentry_strings = []
                for dentry in dtab_input:
                    if hasattr(dentry, 'show'):
                        dentry_strings.append(dentry.show)
                    else:
                        dentry_strings.append(str(dentry))
                self._input_str = "; ".join(dentry_strings)
            else:
                self._input_str = str(dtab_input)
            
            # Parse the input string to populate _dtab
            if self._input_str:
                dtab_bytes = self._input_str.encode('utf-8')
                c_dtab_str = dtab_bytes
                self._dtab = dtab_table_create(c_dtab_str)
                if self._dtab == NULL:
                    raise DtabError("Failed to create Dtab")
    
    def __dealloc__(self):
        if self._dtab != NULL:
            dtab_table_free(self._dtab)
    
    @classmethod
    def read(cls, str dtab_str):
        """Read a Dtab from string."""
        cdef Dtab dtab = cls()
        dtab._input_str = dtab_str
        
        cdef bytes dtab_bytes = dtab_str.encode('utf-8')
        cdef const char* c_dtab_str = dtab_bytes
        
        dtab._dtab = dtab_table_create(c_dtab_str)
        if dtab._dtab == NULL:
            raise DtabError("Failed to create Dtab")
        
        return dtab

    @property
    def dentries(self):
        """Get dentries."""
        # For now, return empty list to avoid segfaults
        # The Rust implementation handles the actual dentries
        return []
    
    @property
    def length(self):
        """Get number of dentries."""
        if self._dtab == NULL:
            return 0
        return self._dtab.length
    
    @property
    def is_empty(self):
        """Check if dtab is empty."""
        return self.length == 0

    @classmethod
    def empty(cls):
        """Get empty dtab."""
        cdef Dtab dtab = cls()
        dtab._input_str = ""
        return dtab
    
    @classmethod
    def fail(cls):
        """Get failing dtab."""
        cdef Dtab dtab = cls()
        dtab._input_str = "/=>!"
        return dtab

    def _convert_alt_format(self, str alt_str):
        """Convert Alt(...) format to | format for parsing."""
        if alt_str.startswith('Alt(') and alt_str.endswith(')'):
            # Extract the content between Alt( and )
            content = alt_str[4:-1]
            # Split by ', ' and join with ' | '
            parts = content.split(', ')
            return ' | '.join(parts)
        return alt_str

    def lookup(self, path):
        """Lookup a path in the dtab."""
        cdef dtab_path_t* c_path
        cdef dtab_nametree_t* result
        cdef char* result_str
        cdef str result_py_str
        
        if self._dtab == NULL:
            raise ValueError("Dtab not initialized")
        
        # Handle both Path and NameTree inputs
        if isinstance(path, Path):
            # Convert Path to C path structure
            c_path = dtab_path_create(path.show.encode('utf-8'))
            if c_path == NULL:
                raise DtabError("Failed to create path for lookup")
            
            try:
                # Use the FFI lookup function
                result = dtab_table_lookup(self._dtab, c_path)
                if result == NULL:
                    return Neg()
                
                # Convert result back to Python NameTree
                result_str = dtab_nametree_show_compact(result)
                if result_str == NULL:
                    dtab_nametree_free(result)
                    return Neg()
                
                try:
                    result_py_str = result_str.decode('utf-8')
                    print(f"DEBUG: FFI returned string: '{result_py_str}'")
                    # Convert Alt format to pipe format if needed
                    converted_str = self._convert_alt_format(result_py_str)
                    print(f"DEBUG: Converted string: '{converted_str}'")
                    return NameTree.read(converted_str)
                finally:
                    dtab_string_free(result_str)
                    dtab_nametree_free(result)
            finally:
                dtab_path_free(c_path)
        elif isinstance(path, NameTree):
            # For NameTree inputs, extract the path and lookup
            if hasattr(path, 'show'):
                path_str = path.show
                if path_str.startswith('/'):
                    # Extract the path part
                    return self.lookup(Path.read(path_str))
            return Neg()
        else:
            raise TypeError("Input must be a Path or NameTree")
    
    def __iter__(self):
        """Iterate over dentries."""
        # For now, return empty iterator
        return iter([])
    
    def __add__(self, other):
        """Add dentries or dtabs."""
        if isinstance(other, Dentry):
            # Convert dentry to string and concatenate
            return self.copy(dentry=other)
        elif isinstance(other, Dtab):
            # Concatenate the string representations
            self_str = getattr(self, '_input_str', self.show)
            other_str = getattr(other, '_input_str', other.show)
            combined_str = f"{self_str}; {other_str}".strip("; ")
            return Dtab.read(combined_str)
        raise TypeError(f"unsupported operand type(s) for +: '{type(self).__name__}' and '{type(other).__name__}'")
    
    def __eq__(self, other):
        """Check equality."""
        if not isinstance(other, Dtab):
            return False
        self_str = getattr(self, '_input_str', self.show)
        other_str = getattr(other, '_input_str', other.show)
        return self_str == other_str
    
    def __ne__(self, other):
        """Check inequality."""
        return not self.__eq__(other)
    
    def __hash__(self):
        """Hash based on string representation."""
        return hash(str(self))
    
    def copy(self, dentry=None):
        """Copy dtab with optional dentry appended."""
        if dentry is None:
            return Dtab.read(self._input_str)
        else:
            # Append dentry to string representation
            new_str = f"{self._input_str}; {dentry.show}".strip("; ")
            return Dtab.read(new_str)
    
    @property
    def show(self):
        """Get string representation."""
        if self._dtab == NULL:
            return self._input_str
        
        # Use the C API to get the string representation
        cdef char* dtab_str = dtab_table_show(self._dtab)
        if dtab_str == NULL:
            return self._input_str
        
        try:
            return dtab_str.decode('utf-8')
        finally:
            dtab_string_free(dtab_str)
    
    def __str__(self):
        return f"Dtab({self.show})"
    
    def __repr__(self):
        return self.__str__()
    
    def pretty_print(self):
        """Pretty print the dtab."""
        print(f"Dtab({self.length})")
        for dentry in self.dentries:
            print(f" {dentry.prefix.show} => {dentry.nametree}")

# Create singleton instances
_empty = Empty()
_fail = Fail()
_neg = Neg()

# Export singletons at module level
__all__ = ['Path', 'NameTree', 'Dentry', 'Dtab', 'DtabError', 'Empty', 'Fail', 'Neg'] 