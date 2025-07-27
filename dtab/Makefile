# Makefile for DTab C API
# Builds the Rust library and C example

# Configuration
RUST_LIB_RLIB = target/release/libdtab.rlib
RUST_LIB_STATIC = target/release/libdtab.a
PYTHON_EXTENSION = dtab_core.so

# Detect OS for dynamic library naming
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    RUST_LIB_DYNAMIC = target/release/libdtab.dylib
    DYNAMIC_EXT = dylib
else
    RUST_LIB_DYNAMIC = target/release/libdtab.so
    DYNAMIC_EXT = so
endif

C_HEADER = include/dtab.h
C_EXAMPLE = examples/basic_usage.c
C_EXAMPLE_BIN = examples/basic_usage

# Rust build flags
RUST_FEATURES = --features ffi
RUST_PROFILE = release

# Installation directories (configurable)
DESTDIR ?=
LIBDIR ?= /usr/local/lib
INCLUDEDIR ?= /usr/local/include

# C compiler flags
CC = gcc
CFLAGS = -Wall -Wextra -std=c99 -Iinclude
LDFLAGS_STATIC = -Ltarget/release -ldtab -lpthread -ldl -lm
LDFLAGS_DYNAMIC = -Ltarget/release -ldtab -lpthread -ldl -lm

# Default target
all: $(C_EXAMPLE_BIN) $(PYTHON_EXTENSION)

# Build Python/Cython extension
$(PYTHON_EXTENSION): $(RUST_LIB_STATIC) setup.py dtab_core.pyx
	@echo "Building Python/Cython extension..."
	/usr/bin/python3 setup.py build_ext --inplace

# Build just the Python extension
python-ext: $(PYTHON_EXTENSION)

# Build Rust library (rlib)
$(RUST_LIB_RLIB):
	cargo build $(RUST_FEATURES) --$(RUST_PROFILE)

# Build Rust library (static) - create .a from .rlib
$(RUST_LIB_STATIC): $(RUST_LIB_RLIB)
	# Extract static library from rlib
	ar x $(RUST_LIB_RLIB)
	ar rcs $@ *.o
	rm -f *.o

# Build Rust library (dynamic) - create shared object from .rlib
$(RUST_LIB_DYNAMIC): $(RUST_LIB_RLIB)
	# Dynamic library is created automatically by cargo with cdylib crate-type
	# File extension is determined by OS (.dylib on macOS, .so on Linux)

# Build C example (static linking)
$(C_EXAMPLE_BIN): $(C_EXAMPLE) $(RUST_LIB_STATIC) $(C_HEADER)
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS_STATIC)

# Run the example
run: $(C_EXAMPLE_BIN)
	./$(C_EXAMPLE_BIN)

# Clean build artifacts
clean:
	cargo clean
	rm -f $(C_EXAMPLE_BIN)
	rm -f $(RUST_LIB_DYNAMIC)
	rm -f $(PYTHON_EXTENSION)
	rm -rf build/
	rm -rf *.egg-info/
	rm -f dtab_core.c

# Install library and headers (system-wide)
install: $(RUST_LIB_STATIC) $(RUST_LIB_DYNAMIC)
	sudo mkdir -p $(DESTDIR)$(LIBDIR) $(DESTDIR)$(INCLUDEDIR)
	sudo cp $(RUST_LIB_STATIC) $(DESTDIR)$(LIBDIR)/
	sudo cp $(RUST_LIB_DYNAMIC) $(DESTDIR)$(LIBDIR)/
	sudo cp $(C_HEADER) $(DESTDIR)$(INCLUDEDIR)/
	sudo ldconfig

# Uninstall
uninstall:
	sudo rm -f $(DESTDIR)$(LIBDIR)/libdtab.a
	sudo rm -f $(DESTDIR)$(LIBDIR)/libdtab.$(DYNAMIC_EXT)
	sudo rm -f $(DESTDIR)$(INCLUDEDIR)/dtab.h
	sudo ldconfig

# Test the library
test: test-rust test-py

test-rust:
	cargo test $(RUST_FEATURES)
	@echo "Running Python/Cython tests..."

test-py: python-ext
	@cd .. && /usr/bin/python3 -m pytest dtab/tests

# Check for memory leaks (with macOS compatibility)
memcheck: $(C_EXAMPLE_BIN)
	@echo "Running memory check..."
	@if command -v valgrind >/dev/null 2>&1; then \
		echo "Using Valgrind for memory checking..."; \
		valgrind --leak-check=full --show-leak-kinds=all --suppressions=/dev/null ./$(C_EXAMPLE_BIN) 2>&1 | grep -E "(definitely lost|indirectly lost|possibly lost|still reachable)" || echo "No memory leaks detected"; \
	else \
		echo "Valgrind not available. Running basic memory check..."; \
		echo "Note: For comprehensive memory checking on macOS, consider using:"; \
		echo "  - AddressSanitizer: RUSTFLAGS='-Z sanitizer=address' cargo run"; \
		echo "  - Instruments.app (Xcode): Run with Leaks instrument"; \
		echo "  - Leaks command: leaks -atExit -- ./$(C_EXAMPLE_BIN)"; \
		./$(C_EXAMPLE_BIN) && echo "Basic execution completed successfully"; \
	fi

# macOS-specific memory leak detection using native tools
macos-memcheck: $(C_EXAMPLE_BIN)
	@echo "Running macOS memory leak detection..."
	@if command -v leaks >/dev/null 2>&1; then \
		echo "Using macOS leaks command..."; \
		leaks -atExit -- ./$(C_EXAMPLE_BIN) 2>&1 | grep -E "(leak|Leak)" || echo "No memory leaks detected"; \
	else \
		echo "macOS leaks command not available. Running basic execution..."; \
		./$(C_EXAMPLE_BIN) && echo "Basic execution completed successfully"; \
	fi

# Documentation
docs:
	cargo doc --open

# Format code
fmt:
	cargo fmt
	clang-format -i $(C_EXAMPLE) $(C_HEADER)

# Lint code
lint:
	cargo clippy $(RUST_FEATURES)
	@echo "Note: clang-tidy not available, skipping C linting"

# Show help
help:
	@echo "Available targets:"
	@echo "  all        - Build everything (C example + Python extension)"
	@echo "  run        - Build and run the C example"
	@echo "  python-ext - Build just the Python/Cython extension"
	@echo "  test       - Run Rust and Python tests"
	@echo "  clean      - Remove build artifacts"
	@echo "  install    - Install library system-wide (static + dynamic)"
	@echo "  uninstall  - Remove installed library"
	@echo ""
	@echo "Installation directories (configurable):"
	@echo "  DESTDIR=$(DESTDIR)"
	@echo "  LIBDIR=$(LIBDIR)"
	@echo "  INCLUDEDIR=$(INCLUDEDIR)"
	@echo "  memcheck   - Run with valgrind memory checker"
	@echo "  macos-memcheck - Run with macOS native memory leak detection"
	@echo "  docs       - Generate documentation"
	@echo "  fmt        - Format code"
	@echo "  lint       - Lint code"
	@echo "  help       - Show this help"
	@echo ""
	@echo "Library files:"
	@echo "  $(RUST_LIB_STATIC)  - Static library"
	@echo "  $(RUST_LIB_DYNAMIC) - Dynamic shared object library ($(UNAME_S))"
	@echo "  $(PYTHON_EXTENSION) - Python/Cython extension module"

.PHONY: all run test clean install uninstall memcheck docs fmt lint help python-ext 
