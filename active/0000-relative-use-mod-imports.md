- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

introduce ```use mod ...``` as a fused relative module import and namespace 'use'

# Motivation

## versatility for compile units
consider moving between the extremes of one compilation unit per file, and an entire project as one compilation unit - with the existing use/mod behaviour, you must refactor how modules are brought in and how components are referenced when moving between these extremes.

Relative paths would allow greater flexibility when wanting to treat project subtrees as seperate libraries.

A build system would be at liberty to cache any appropriate subtree equivalently to a crate under the current system.

## learning curve
The seperate absolute and relative paths, and mod / use statements are a tripping point for new users

## tooling
with a project setup this way, a tool can locate definitions starting at the 'current' file and spidering outward. While working on a project, one may have code in different component libraries open,needing changes; Under the current system, each of which would have different absolute paths.

## simpler imports
eliminate the need for seperate ```mod.rs``` files within directories

# Detailed design

use mod would look for a file relative to the current locatoin

given some source files in 
foo.rs
bar.rs
baz/qux.rs
../qaz.rs

from foo.rs,
```use mod bar```
```use mod baz::qux```
```use mod super::qaz```

would add these files to the project, and make bar,baz,quz available as qualifiers to reference symbols in these files. ```use``` statements would bring more individual symbols in, or longer paths could be written to access subtrees of these modules.

Each file referenced would in turn be able to bring more files in.

from qux.rs,
```use mod super::super::qaz``` 
would make ```qaz``` available.

item paths would still reflect the directory-structure: - when a series of siblings reference eachother, one would not be able to follow this graph to reach symbols.

# Drawbacks

heirachical 'use' paths have their own problems. When moving sources up or down the directory tree, refactoring would still be needed;

one might need references to a long string of ```super::super::..``` statements to refer to symbols in the project root.

The behaviour of the standard prelude would not seem as consistent with this scheme.


