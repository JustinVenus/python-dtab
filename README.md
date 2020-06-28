# python-dtab
Python DTab Parser (Delegation Table)

This is a direct port of Finagle's [DTab implementation](http://twitter.github.io/finagle/guide/Names.html).

## What works
* parser (see tests for usage)

## What doesn't work
* namers aren't implemented

## Repo Tooling
This repo is using the [pants build tool](https://github.com/pantsbuild/pants/).

## Building Wheel
```sh
./pants --pants-config-files=bdist.toml setup-py dtab:dtab --setup-py-run="bdist_wheel --universal -d ../"
```
The wheelfile will be written to the `dist` directory at the root of the repo.

## Running tests
```sh
$ ./pants test dtab/tests:tests
```

## Linting and Formatting
```sh
$ ./pants lint dtab::
$ ./pants fmt dtab::
```

## REPL
```sh
$ ./pants repl dtab:dtab
Python 3.6.4 (default, Dec  1 2019, 11:49:07)
[GCC 4.2.1 Compatible Apple LLVM 11.0.0 (clang-1100.0.33.8)] on darwin
Type "help", "copyright", "credits" or "license" for more information.
(InteractiveConsole)
>>> from dtab import Dtab
>>> Dtab.read("/s/foo=>/$/something/different").show
'Label(s),Label(foo)=>Path(/$/something/different)'
>>> quit()
```
