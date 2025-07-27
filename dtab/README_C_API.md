# DTab C Language Bindings

This directory contains C language bindings for the DTab (Delegation Table) library, providing idiomatic C APIs for service discovery and resolution functionality.

## Features

- **Idiomatic C API**: Follows C conventions with proper memory management
- **Comprehensive Error Handling**: Robust error handling with detailed error messages
- **Memory Safety**: Proper cleanup functions for all allocated resources
- **Full Feature Support**: Complete support for all DTab functionality
- **Cross-Platform**: Works on Linux, macOS, and other Unix-like systems

## API Overview

### Core Types

- `dtab_path_t`: Hierarchical path representation
- `dtab_nametree_t`: Resolution outcome tree
- `dtab_dentry_t`: Delegation rule (prefix → target)
- `dtab_table_t`: Delegation table containing rules

### Key Functions

#### Path Operations
```c
dtab_path_t* dtab_path_create(const char* path_str);
void dtab_path_free(dtab_path_t* path);
char* dtab_path_show(const dtab_path_t* path);
```

#### NameTree Operations
```c
dtab_nametree_t* dtab_nametree_parse(const char* input);
void dtab_nametree_free(dtab_nametree_t* tree);
char* dtab_nametree_show(const dtab_nametree_t* tree);
char* dtab_nametree_show_compact(const dtab_nametree_t* tree);
```

#### Dentry Operations
```c
dtab_dentry_t* dtab_dentry_parse(const char* input);
void dtab_dentry_free(dtab_dentry_t* dentry);
char* dtab_dentry_show(const dtab_dentry_t* dentry);
```

#### Dtab Operations
```c
dtab_table_t* dtab_table_create(const char* input);
void dtab_table_free(dtab_table_t* table);
dtab_nametree_t* dtab_table_lookup(const dtab_table_t* table, const dtab_path_t* path);
char* dtab_table_show(const dtab_table_t* table);
```

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- C compiler (GCC or Clang)
- Make

### Building

1. **Build the Rust library with FFI support:**
   ```bash
   cargo build --features ffi --release
   ```

2. **Build the C example:**
   ```bash
   make
   ```

3. **Run the example:**
   ```bash
   make run
   ```

### System Installation

To install the library system-wide:

```bash
make install
```

This installs:
- Library: `/usr/local/lib/libdtab.a`
- Header: `/usr/local/include/dtab.h`

To uninstall:
```bash
make uninstall
```

## Usage Examples

### Basic Path Operations

```c
#include "dtab.h"

// Create a path
dtab_path_t* path = dtab_path_create("/foo/bar/*/baz");
if (path) {
    // Show the path
    char* path_str = dtab_path_show(path);
    printf("Path: %s\n", path_str);
    dtab_string_free(path_str);
    
    // Cleanup
    dtab_path_free(path);
}
```

### NameTree Parsing

```c
#include "dtab.h"

// Parse complex NameTree expressions
dtab_nametree_t* tree = dtab_nametree_parse("/foo | /bar");
if (tree) {
    // Use the tree...
    dtab_nametree_free(tree);
}

// Parse weighted unions
tree = dtab_nametree_parse("2 * /foo & 3 * /bar");
if (tree) {
    dtab_nametree_free(tree);
}

// Show the difference between toString and show formats
dtab_nametree_t* fail_tree = dtab_nametree_parse("!");
if (fail_tree) {
    char* toString = dtab_nametree_show(fail_tree);
    char* show = dtab_nametree_show_compact(fail_tree);
    printf("toString: %s\n", toString);  // Prints "Fail"
    printf("show: %s\n", show);          // Prints "!"
    dtab_string_free(toString);
    dtab_string_free(show);
    dtab_nametree_free(fail_tree);
}
```

### Dentry Operations

```c
#include "dtab.h"

// Parse individual delegation rules
dtab_dentry_t* dentry = dtab_dentry_parse("/foo=>/bar");
if (dentry) {
    // Show the dentry
    char* dentry_str = dtab_dentry_show(dentry);
    printf("Dentry: %s\n", dentry_str);
    dtab_string_free(dentry_str);
    dtab_dentry_free(dentry);
}

// Parse dentry with wildcards
dentry = dtab_dentry_parse("/foo/*/bar=>/baz");
if (dentry) {
    dtab_dentry_free(dentry);
}
```

### Delegation Table Operations

```c
#include "dtab.h"

// Create a delegation table
dtab_table_t* table = dtab_table_create("/foo=>/bar;/baz=>/qux");
if (table) {
    // Create a path to lookup
    dtab_path_t* path = dtab_path_create("/foo/service");
    if (path) {
        // Perform lookup
        dtab_nametree_t* result = dtab_table_lookup(table, path);
        if (result) {
            // Use the result...
            dtab_nametree_free(result);
        }
        dtab_path_free(path);
    }
    dtab_table_free(table);
}

// Show the delegation table
dtab_table_t* table = dtab_table_create("/foo=>/bar;/baz=>/qux");
if (table) {
    char* table_str = dtab_table_show(table);
    printf("Table: %s\n", table_str);
    dtab_string_free(table_str);
    dtab_table_free(table);
}
```

### Complex Delegation Chains

```c
#include "dtab.h"

// Create a complex delegation table
const char* dtab_str = "/zk#=>/$/com.twitter.serverset;"
                       "/zk=>/zk#;"
                       "/s##=>/zk/zk.local.twitter.com:2181;"
                       "/s#=>/s##/prod;"
                       "/s=>/s#";

dtab_table_t* table = dtab_table_create(dtab_str);
if (table) {
    dtab_path_t* path = dtab_path_create("/s/crawler");
    if (path) {
        dtab_nametree_t* result = dtab_table_lookup(table, path);
        if (result) {
            // Result will be the resolved path after delegation chain
            dtab_nametree_free(result);
        }
        dtab_path_free(path);
    }
    dtab_table_free(table);
}
```

## Memory Management

The API follows C conventions for memory management:

- **Allocation**: Functions return pointers to allocated structures
- **Cleanup**: Each allocated structure has a corresponding `*_free()` function
- **Strings**: String functions return `char*` that must be freed with `dtab_string_free()`

### Memory Management Rules

1. **Always free allocated resources**:
   ```c
   dtab_path_t* path = dtab_path_create("/foo");
   // ... use path ...
   dtab_path_free(path);  // Must call this!
   ```

2. **Free strings returned by the library**:
   ```c
   char* str = dtab_path_show(path);
   printf("%s\n", str);
   dtab_string_free(str);  // Must call this!
   ```

3. **Free nested structures**:
   ```c
   dtab_nametree_t* tree = dtab_nametree_parse("/foo | /bar");
   // ... use tree ...
   dtab_nametree_free(tree);  // Frees tree and all nested structures
   ```

## Error Handling

The API uses return values for error handling:

- **NULL return**: Indicates an error occurred
- **Non-NULL return**: Success, but caller must free the result

```c
dtab_path_t* path = dtab_path_create("invalid/path");
if (!path) {
    // Handle error
    fprintf(stderr, "Failed to create path\n");
    return;
}
// Use path...
dtab_path_free(path);
```

### Error Result Structure

For operations that may fail, the API provides a result structure:

```c
typedef struct {
    bool success;
    char* error_message;
} dtab_result_t;
```

**Note:** The `dtab_result_t` structure is available for future use but most functions currently use NULL returns for error indication.

#### `dtab_result_free(dtab_result_t* result)`
Frees a result structure and its associated memory.

**Parameters:**
- `result`: Pointer to result structure to free

## API Reference

### Path API

#### `dtab_path_create(const char* path_str)`
Creates a path from string representation.

**Parameters:**
- `path_str`: String representation (e.g., "/foo/bar/*/baz")

**Returns:**
- `dtab_path_t*`: Path structure, or NULL on error

**Example:**
```c
dtab_path_t* path = dtab_path_create("/foo/bar/*/baz");
```

#### `dtab_path_free(dtab_path_t* path)`
Frees a path structure and its memory.

**Parameters:**
- `path`: Path structure to free

#### `dtab_path_show(const dtab_path_t* path)`
Gets string representation of a path.

**Parameters:**
- `path`: Path structure

**Returns:**
- `char*`: String representation (must be freed with `dtab_string_free()`)

### NameTree API

#### `dtab_nametree_parse(const char* input)`
Parses a NameTree from string representation.

**Parameters:**
- `input`: String representation (e.g., "/foo | /bar", "/foo & /bar")

**Returns:**
- `dtab_nametree_t*`: NameTree structure, or NULL on error

**Supported Syntax:**
- `/path`: Leaf node
- `!`: Fail node
- `~`: Negative node
- `$`: Empty node
- `/foo | /bar`: Alternative (OR)
- `/foo & /bar`: Union (AND)
- `2 * /foo`: Weighted tree
- `(/foo | /bar) & /baz`: Parenthesized expressions

#### `dtab_nametree_free(dtab_nametree_t* tree)`
Frees a NameTree structure and all nested structures.

#### `dtab_nametree_show(const dtab_nametree_t* tree)`
Gets string representation of a NameTree (toString format).

**Parameters:**
- `tree`: NameTree structure

**Returns:**
- `char*`: String representation (must be freed with `dtab_string_free()`)

**Example:**
```c
dtab_nametree_t* tree = dtab_nametree_parse("/foo | /bar");
char* str = dtab_nametree_show(tree);
printf("Tree: %s\n", str);
dtab_string_free(str);
dtab_nametree_free(tree);
```

#### `dtab_nametree_show_compact(const dtab_nametree_t* tree)`
Gets compact string representation of a NameTree (show format).

**Parameters:**
- `tree`: NameTree structure

**Returns:**
- `char*`: Compact string representation (must be freed with `dtab_string_free()`)

**Note:** This returns the show format (e.g., "!" for Fail) rather than toString format (e.g., "Fail").

**Example:**
```c
dtab_nametree_t* tree = dtab_nametree_parse("!");
char* str = dtab_nametree_show_compact(tree);
printf("Compact: %s\n", str);  // Prints "!"
dtab_string_free(str);
dtab_nametree_free(tree);
```

### Dentry API

#### `dtab_dentry_parse(const char* input)`
Parses a delegation rule (prefix → target) from string representation.

**Parameters:**
- `input`: String representation (e.g., "/foo=>/bar")

**Returns:**
- `dtab_dentry_t*`: Dentry structure, or NULL on error

**Example:**
```c
dtab_dentry_t* dentry = dtab_dentry_parse("/foo=>/bar");
```

#### `dtab_dentry_free(dtab_dentry_t* dentry)`
Frees a dentry structure and its memory.

#### `dtab_dentry_show(const dtab_dentry_t* dentry)`
Gets string representation of a dentry.

**Parameters:**
- `dentry`: Dentry structure

**Returns:**
- `char*`: String representation (must be freed with `dtab_string_free()`)

### Dtab API

#### `dtab_table_create(const char* input)`
Creates a delegation table from string representation.

**Parameters:**
- `input`: String representation (e.g., "/foo=>/bar;/baz=>/qux")

**Returns:**
- `dtab_table_t*`: Delegation table, or NULL on error

#### `dtab_table_lookup(const dtab_table_t* table, const dtab_path_t* path)`
Looks up a path in the delegation table.

**Parameters:**
- `table`: Delegation table
- `path`: Path to lookup

**Returns:**
- `dtab_nametree_t*`: Resulting NameTree, or NULL if no match

#### `dtab_table_free(dtab_table_t* table)`
Frees a delegation table and all its entries.

#### `dtab_table_show(const dtab_table_t* table)`
Gets string representation of a delegation table.

**Parameters:**
- `table`: Delegation table

**Returns:**
- `char*`: String representation (must be freed with `dtab_string_free()`)

### Utility Functions

#### `dtab_string_free(char* s)`
Frees a string returned by the library.

#### `dtab_version()`
Gets the library version string.

**Returns:**
- `char*`: Version string (must be freed with `dtab_string_free()`)

## Building Your Own Applications

### Compilation

```bash
gcc -Wall -std=c99 -Iinclude -Ltarget/release -ldtab -lpthread -ldl -lm your_program.c -o your_program
```

### CMake Integration

```cmake
find_library(DTAB_LIB dtab PATHS target/release)
include_directories(include)

add_executable(your_program your_program.c)
target_link_libraries(your_program ${DTAB_LIB} pthread dl m)
```

### pkg-config Integration

Create `dtab.pc`:
```
prefix=/usr/local
exec_prefix=${prefix}
libdir=${exec_prefix}/lib
includedir=${prefix}/include

Name: dtab
Description: DTab C API
Version: 0.1.0
Libs: -L${libdir} -ldtab -lpthread -ldl -lm
Cflags: -I${includedir}
```

## Testing

### Run Rust Tests
```bash
make test
```

### Memory Leak Detection
```bash
make memcheck
```

### Code Quality
```bash
make lint
make fmt
```

## Examples

See `examples/basic_usage.c` for a comprehensive example demonstrating:

- Path creation and manipulation
- NameTree parsing and display
- Dentry parsing
- Dtab table operations
- Complex delegation chains
- Memory management

Run the example:
```bash
make run
```

## Contributing

1. Follow C coding standards
2. Add tests for new functionality
3. Update documentation
4. Run memory leak detection
5. Ensure proper error handling

## License

Same as the main DTab library. 