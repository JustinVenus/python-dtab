/**
 * @file dtab.h
 * @brief Finagle DTab (Delegation Table) C Language Bindings
 * @version 0.1.0
 *
 * This header provides C language bindings for the Rust DTab library,
 * enabling service discovery and resolution functionality in C applications.
 *
 * The API follows idiomatic C conventions with proper memory management
 * and error handling. All functions that return pointers require explicit
 * cleanup using the corresponding *_free() functions.
 */

#ifndef DTAB_H
#define DTAB_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Error handling
// ============================================================================

/**
 * @brief Result structure for operations that may fail
 */
typedef struct {
  bool success;
  char *error_message;
} dtab_result_t;

/**
 * @brief Free a result structure and its associated memory
 * @param result Pointer to result structure to free
 */
void dtab_result_free(dtab_result_t *result);

// ============================================================================
// Path API
// ============================================================================

/**
 * @brief Path structure representing a hierarchical path
 */
typedef struct {
  char **elements;   /**< Array of path element strings */
  size_t length;     /**< Number of elements in the path */
  bool *is_wildcard; /**< Array indicating which elements are wildcards */
} dtab_path_t;

/**
 * @brief Create a path from a string representation
 * @param path_str String representation of path (e.g., "/foo/bar/wildcard/baz")
 * @return Pointer to dtab_path_t, or NULL on error
 *
 * @note The returned path must be freed with dtab_path_free()
 */
dtab_path_t *dtab_path_create(const char *path_str);

/**
 * @brief Free a path structure and its associated memory
 * @param path Pointer to path structure to free
 */
void dtab_path_free(dtab_path_t *path);

/**
 * @brief Get string representation of a path
 * @param path Pointer to path structure
 * @return String representation, or NULL on error
 *
 * @note The returned string must be freed with dtab_string_free()
 */
char *dtab_path_show(const dtab_path_t *path);

// ============================================================================
// NameTree API
// ============================================================================

/**
 * @brief NameTree type enumeration
 */
typedef enum {
  DTAB_NAMETREE_LEAF,  /**< Leaf node containing a path */
  DTAB_NAMETREE_ALT,   /**< Alternative (OR) node */
  DTAB_NAMETREE_UNION, /**< Union (AND) node */
  DTAB_NAMETREE_FAIL,  /**< Fail node */
  DTAB_NAMETREE_NEG,   /**< Negative node */
  DTAB_NAMETREE_EMPTY  /**< Empty node */
} dtab_nametree_type_t;

/**
 * @brief Weighted tree structure for union nodes
 */
typedef struct {
  double weight;              /**< Weight value */
  struct dtab_nametree *tree; /**< Pointer to weighted tree */
} dtab_weighted_t;

/**
 * @brief NameTree structure representing resolution outcomes
 */
typedef struct dtab_nametree {
  dtab_nametree_type_t tree_type; /**< Type of the tree node */

  // For leaf nodes
  dtab_path_t *leaf_path;

  // For alt nodes
  struct dtab_nametree **alt_trees;
  size_t alt_count;

  // For union nodes
  dtab_weighted_t *union_weighted;
  size_t union_count;
} dtab_nametree_t;

/**
 * @brief Parse a NameTree from string representation
 * @param input String representation (e.g., "/foo | /bar", "/foo & /bar")
 * @return Pointer to dtab_nametree_t, or NULL on error
 *
 * @note The returned NameTree must be freed with dtab_nametree_free()
 */
dtab_nametree_t *dtab_nametree_parse(const char *input);

/**
 * @brief Free a NameTree structure and its associated memory
 * @param tree Pointer to NameTree structure to free
 */
void dtab_nametree_free(dtab_nametree_t *tree);

/**
 * @brief Get string representation of a NameTree
 * @param tree Pointer to NameTree structure
 * @return String representation, or NULL on error
 *
 * @note The returned string must be freed with dtab_string_free()
 */
char *dtab_nametree_show(const dtab_nametree_t *tree);

/**
 * @brief Get compact string representation of a NameTree (show format)
 * @param tree Pointer to NameTree structure
 * @return Compact string representation, or NULL on error
 *
 * @note The returned string must be freed with dtab_string_free()
 * @note This returns the show format (e.g., "!" for Fail) rather than toString
 * format
 */
char *dtab_nametree_show_compact(const dtab_nametree_t *tree);

// ============================================================================
// Dentry API
// ============================================================================

/**
 * @brief Dentry structure representing a delegation rule
 */
typedef struct {
  dtab_path_t *prefix;     /**< Path prefix to match */
  dtab_nametree_t *target; /**< Target NameTree */
} dtab_dentry_t;

/**
 * @brief Parse a Dentry from string representation
 * @param input String representation (e.g., "/foo => /bar")
 * @return Pointer to dtab_dentry_t, or NULL on error
 *
 * @note The returned Dentry must be freed with dtab_dentry_free()
 */
dtab_dentry_t *dtab_dentry_parse(const char *input);

/**
 * @brief Free a Dentry structure and its associated memory
 * @param dentry Pointer to Dentry structure to free
 */
void dtab_dentry_free(dtab_dentry_t *dentry);

/**
 * @brief Get string representation of a Dentry
 * @param dentry Pointer to Dentry structure
 * @return String representation, or NULL on error
 *
 * @note The returned string must be freed with dtab_string_free()
 */
char *dtab_dentry_show(const dtab_dentry_t *dentry);

// ============================================================================
// Dtab API
// ============================================================================

/**
 * @brief Dtab structure representing a delegation table
 */
typedef struct {
  dtab_dentry_t **dentries; /**< Array of delegation entries */
  size_t length;            /**< Number of entries */
} dtab_table_t;

/**
 * @brief Create a Dtab from string representation
 * @param input String representation (e.g., "/foo=>/bar;/baz=>/qux")
 * @return Pointer to dtab_table_t, or NULL on error
 *
 * @note The returned Dtab must be freed with dtab_table_free()
 */
dtab_table_t *dtab_table_create(const char *input);

/**
 * @brief Free a DTab table structure and its associated memory
 * @param table Pointer to DTab table structure to free
 */
void dtab_table_free(dtab_table_t *table);

/**
 * @brief Get string representation of a DTab table
 * @param table Pointer to DTab table structure
 * @return String representation, or NULL on error
 *
 * @note The returned string must be freed with dtab_string_free()
 */
char *dtab_table_show(const dtab_table_t *table);

/**
 * @brief Lookup a path in the delegation table
 * @param table Pointer to Dtab structure
 * @param path Pointer to path to lookup
 * @return Pointer to resulting NameTree, or NULL on error/no match
 *
 * @note The returned NameTree must be freed with dtab_nametree_free()
 */
dtab_nametree_t *dtab_table_lookup(const dtab_table_t *table,
                                   const dtab_path_t *path);

// ============================================================================
// Utility functions
// ============================================================================

/**
 * @brief Free a string returned by the library
 * @param s Pointer to string to free
 */
void dtab_string_free(char *s);

/**
 * @brief Get library version string
 * @return Version string (e.g., "0.1.0")
 *
 * @note The returned string must be freed with dtab_string_free()
 */
char *dtab_version(void);

// ============================================================================
// Example usage
// ============================================================================

/**
 * @example basic_usage.c
 * Basic usage example:
 * @code
 * // Create a delegation table
 * dtab_table_t* table = dtab_table_create("/foo=>/bar;/baz=>/qux");
 * if (!table) {
 *     // Handle error
 *     return;
 * }
 *
 * // Create a path to lookup
 * dtab_path_t* path = dtab_path_create("/foo/service");
 * if (!path) {
 *     dtab_table_free(table);
 *     return;
 * }
 *
 * // Perform lookup
 * dtab_nametree_t* result = dtab_table_lookup(table, path);
 * if (result) {
 *     // Use result...
 *     dtab_nametree_free(result);
 * }
 *
 * // Cleanup
 * dtab_path_free(path);
 * dtab_table_free(table);
 * @endcode
 */

#ifdef __cplusplus
}
#endif

#endif // DTAB_H