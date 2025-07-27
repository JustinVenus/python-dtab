/**
 * @file basic_usage.c
 * @brief Basic usage example for DTab C API
 *
 * This example demonstrates how to use the DTab C language bindings
 * for service discovery and resolution.
 */

#include "../include/dtab.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * @brief Print a NameTree structure recursively
 */
void print_nametree(const dtab_nametree_t *tree, int indent) {
  if (!tree)
    return;

  // Print indentation
  for (int i = 0; i < indent; i++) {
    printf("  ");
  }

  switch (tree->tree_type) {
  case DTAB_NAMETREE_LEAF:
    if (tree->leaf_path) {
      char *path_str = dtab_path_show(tree->leaf_path);
      printf("Leaf: %s\n", path_str ? path_str : "NULL");
      dtab_string_free(path_str);
    } else {
      printf("Leaf: NULL\n");
    }
    break;

  case DTAB_NAMETREE_ALT:
    printf("Alt (%zu alternatives):\n", tree->alt_count);
    for (size_t i = 0; i < tree->alt_count; i++) {
      if (tree->alt_trees[i]) {
        print_nametree(tree->alt_trees[i], indent + 1);
      }
    }
    break;

  case DTAB_NAMETREE_UNION:
    printf("Union (%zu weighted trees):\n", tree->union_count);
    for (size_t i = 0; i < tree->union_count; i++) {
      for (int j = 0; j < indent + 1; j++) {
        printf("  ");
      }
      printf("Weight: %.2f\n", tree->union_weighted[i].weight);
      if (tree->union_weighted[i].tree) {
        print_nametree(tree->union_weighted[i].tree, indent + 2);
      }
    }
    break;

  case DTAB_NAMETREE_FAIL:
    printf("Fail\n");
    break;

  case DTAB_NAMETREE_NEG:
    printf("Neg\n");
    break;

  case DTAB_NAMETREE_EMPTY:
    printf("Empty\n");
    break;
  }
}

/**
 * @brief Example: Basic path creation and manipulation
 */
void example_path_operations(void) {
  printf("=== Path Operations Example ===\n");

  // Create a path
  dtab_path_t *path = dtab_path_create("/foo/bar/*/baz");
  if (!path) {
    printf("Failed to create path\n");
    return;
  }

  // Show the path
  char *path_str = dtab_path_show(path);
  printf("Path: %s\n", path_str ? path_str : "NULL");
  dtab_string_free(path_str);

  // Print path details
  printf("Path has %zu elements:\n", path->length);
  for (size_t i = 0; i < path->length; i++) {
    printf("  Element %zu: %s%s\n", i,
           path->elements[i] ? path->elements[i] : "NULL",
           path->is_wildcard[i] ? " (wildcard)" : "");
  }

  dtab_path_free(path);
  printf("\n");
}

/**
 * @brief Example: NameTree parsing and display
 */
void example_nametree_parsing(void) {
  printf("=== NameTree Parsing Example ===\n");

  // Test various NameTree expressions
  const char *expressions[] = {"/foo",        "/foo | /bar",
                               "/foo & /bar", "2 * /foo & 3 * /bar",
                               "! | ~ | $",   "(/foo | /bar) & /baz"};

  for (size_t i = 0; i < sizeof(expressions) / sizeof(expressions[0]); i++) {
    printf("Parsing: %s\n", expressions[i]);

    dtab_nametree_t *tree = dtab_nametree_parse(expressions[i]);
    if (tree) {
      print_nametree(tree, 1);
      dtab_nametree_free(tree);
    } else {
      printf("  Failed to parse\n");
    }
    printf("\n");
  }
}

/**
 * @brief Example: Dentry parsing
 */
void example_dentry_parsing(void) {
  printf("=== Dentry Parsing Example ===\n");

  const char *dentries[] = {"/foo => /bar", "/foo/*/baz => /bar", "/s => /s#",
                            "/s# => /s##/prod"};

  for (size_t i = 0; i < sizeof(dentries) / sizeof(dentries[0]); i++) {
    printf("Parsing dentry: %s\n", dentries[i]);

    dtab_dentry_t *dentry = dtab_dentry_parse(dentries[i]);
    if (dentry) {
      // Show prefix
      char *prefix_str = dtab_path_show(dentry->prefix);
      printf("  Prefix: %s\n", prefix_str ? prefix_str : "NULL");
      dtab_string_free(prefix_str);

      // Show target
      printf("  Target:\n");
      print_nametree(dentry->target, 2);

      dtab_dentry_free(dentry);
    } else {
      printf("  Failed to parse\n");
    }
    printf("\n");
  }
}

/**
 * @brief Example: Dtab table operations
 */
void example_dtab_operations(void) {
  printf("=== Dtab Operations Example ===\n");

  // Create a delegation table
  const char *dtab_str = "/foo=>/bar;/baz=>/qux;/s=>/s#;/s#=>/s##/prod";
  printf("Creating dtab: %s\n", dtab_str);

  dtab_table_t *table = dtab_table_create(dtab_str);
  if (!table) {
    printf("Failed to create dtab table\n");
    return;
  }

  printf("Dtab has %zu entries\n", table->length);

  // Test some lookups
  const char *test_paths[] = {"/foo/service", "/baz/api", "/s/crawler",
                              "/nonexistent/path"};

  for (size_t i = 0; i < sizeof(test_paths) / sizeof(test_paths[0]); i++) {
    printf("Looking up: %s\n", test_paths[i]);

    dtab_path_t *path = dtab_path_create(test_paths[i]);
    if (!path) {
      printf("  Failed to create path\n");
      continue;
    }

    dtab_nametree_t *result = dtab_table_lookup(table, path);
    if (result) {
      printf("  Result:\n");
      print_nametree(result, 2);
      dtab_nametree_free(result);
    } else {
      printf("  No match found\n");
    }

    dtab_path_free(path);
    printf("\n");
  }

  dtab_table_free(table);
}

/**
 * @brief Example: Complex delegation chain
 */
void example_complex_chain(void) {
  printf("=== Complex Delegation Chain Example ===\n");

  // Create a complex delegation table similar to the Python examples
  const char *dtab_str = "/zk#=>/$/com.twitter.serverset;/zk=>/zk#;/s##=>/zk/"
                         "zk.local.twitter.com:2181;/s#=>/s##/prod;/s=>/s#";
  printf("Creating complex dtab: %s\n", dtab_str);

  dtab_table_t *table = dtab_table_create(dtab_str);
  if (!table) {
    printf("Failed to create dtab table\n");
    return;
  }

  // Test the delegation chain
  const char *test_path = "/s/crawler";
  printf("Looking up: %s\n", test_path);

  dtab_path_t *path = dtab_path_create(test_path);
  if (!path) {
    printf("Failed to create path\n");
    dtab_table_free(table);
    return;
  }

  dtab_nametree_t *result = dtab_table_lookup(table, path);
  if (result) {
    printf("Result:\n");
    print_nametree(result, 1);
    dtab_nametree_free(result);
  } else {
    printf("No match found\n");
  }

  dtab_path_free(path);
  dtab_table_free(table);
}

/**
 * @brief Example: Memory management demonstration
 */
void example_memory_management(void) {
  printf("=== Memory Management Example ===\n");

  // Demonstrate proper memory management
  dtab_path_t *path = dtab_path_create("/test/path");
  if (path) {
    char *path_str = dtab_path_show(path);
    printf("Created path: %s\n", path_str ? path_str : "NULL");

    // Free the string
    dtab_string_free(path_str);

    // Free the path
    dtab_path_free(path);
    printf("Path freed successfully\n");
  }

  // Test error handling
  dtab_path_t *invalid_path = dtab_path_create("invalid/path");
  if (!invalid_path) {
    printf("Correctly rejected invalid path\n");
  }

  printf("\n");
}

int main(void) {
  printf("DTab C API Example Program\n");
  printf("Version: %s\n\n", dtab_version());

  // Run examples
  example_path_operations();
  example_nametree_parsing();
  example_dentry_parsing();
  example_dtab_operations();
  example_complex_chain();
  example_memory_management();

  printf("All examples completed successfully!\n");
  return 0;
}