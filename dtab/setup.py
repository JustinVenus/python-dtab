#!/usr/bin/env python3
"""
Setup script for DTab Cython extension.
"""

import os
import sys
import platform
from setuptools import setup, Extension
from Cython.Build import cythonize

# Get the directory containing this setup.py
current_dir = os.path.dirname(os.path.abspath(__file__))

# Determine architecture-specific compiler flags
if platform.machine() == 'arm64':
    extra_compile_args = ['-Wall', '-Wextra', '-arch', 'arm64']
    extra_link_args = ['-ldtab', '-arch', 'arm64']
else:
    extra_compile_args = ['-Wall', '-Wextra']
    extra_link_args = ['-ldtab']

# Define the Cython extension module
dtab_extension = Extension(
    'dtab_core',
    sources=['dtab_core.pyx'],
    include_dirs=[
        'include',
        'src',
    ],
    library_dirs=['target/release'],
    libraries=['dtab'],
    extra_compile_args=extra_compile_args,
    extra_link_args=extra_link_args,
    define_macros=[('PYTHON_EXTENSION', '1')],
)

setup(
    name='dtab',
    version='0.1.0',
    description='DTab (Delegation Tables) Cython extension',
    author='DTab Team',
    ext_modules=cythonize([dtab_extension]),
    install_requires=["Cython"],
    python_requires='>=3.6',
) 