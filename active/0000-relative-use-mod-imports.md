- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

introduce ```use mod ...;``` as a fused module import and namespace 'use', using relative filename/module paths

# Motivation

## versatility for compile units

Consider moving between the extremes of one compilation unit per file, and an entire project as a single compilation unit - with the existing use/mod behaviour, you would have to refactor how modules are brought in and how components are referenced when moving between these extremes.

Relative paths would allow greater flexibility when wanting to treat project subtrees as seperate libraries, or vica versa.

A build system would be at liberty to cache any appropriate subtree equivalently to a library crate.

## learning curve
The seperate absolute and relative paths, and mod / use statements are a tripping point for new users

## tooling
with a project setup this way, a tool can locate definitions starting at the 'current' file and spidering outward. While working on a project, one may have code in different component libraries open,needing changes; Under the current system, each of which would have different absolute paths.

## simpler imports
eliminate the need for seperate ```mod.rs``` files within directories. Each file would do the job of mod.rs specifying further files to bring in.

# Detailed design

use mod would look for a file relative to the current locatoin

given some source files in 
foo.rs
bar.rs
baz/qux.rs
../qaz.rs

from foo.rs, the following statements
```use mod bar;```
```use mod baz::qux;```
```use mod super::qaz;```

would add these files to the project, and make ```bar::,qux::,qaz::``` available as qualifiers to reference symbols of those files within foo.rs . Further ```use``` could shortcut more individual symbols, and longer paths could be written to access subtrees of these modules.

Each individual file would in turn be able to bring in its own relative files.

eg if qux.rs contained the statement ```use mod super::super::qaz;``` , ```../qaz.rs``` would be brought into the project, although 'foo.rs' would still need an additional ```use super::qaz``` to reference symbols in ```qaz.rs```.

item paths would still reflect the directory-structure: - when a series of siblings reference eachother, one would not be able to follow this graph to reach symbols.

# Drawbacks

Replicates functionality available with use, mod and #[path=...] directives, and is a slightly different mentality to the existing system.

heirachical 'use' paths have their own problems. When moving sources up or down the directory tree, refactoring would still be needed;

one might need references to a long string of ```super::super::..``` statements to refer to symbols in the project root.

The behaviour of the standard prelude would not seem as consistent with this scheme.


