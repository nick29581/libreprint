# libreprint

Utility library for reprinting files with changes.

The idea here is that some other tool (Rustfmt, refactoring tool, etc.) wants to
change a file and has a set of changes, and this lib just does the io to make
that happen.
