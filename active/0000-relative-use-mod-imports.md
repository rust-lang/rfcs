- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

introduce ```use mod ...;``` as a fused module import and namespace 'use', using relative module paths, which are also relative filename paths.

This system exploits the coherence between the module heirarchy and the filesystem directory tree - but it *assumes* this coherence, instead of relying on the user to manually create it with 'mod.rs' files. So the information of 'bringing things into scope' should be enough to specify what to load.



# Motivation

## versatility for compile units

Consider moving between the extremes of one compilation unit per file, and an entire project as a single compilation unit - with the existing use/mod behaviour, you would have to refactor how modules are brought in and how components are referenced when moving between these extremes.

Relative paths would allow greater flexibility when wanting to treat project subtrees as seperate libraries, or vica versa. eg. for building examples demonstrating components of an SDK, or a single source tree building a suite of tools.

A build system would be at liberty to cache any appropriate subtree equivalently to a library crate.

## shorter learning curve
The seperate absolute and relative paths, and mod / use statements are a tripping point for new users. Under this scheme, you only see relative paths, and you only need one statement 'use mod '.

## tooling
with a project setup this way, a tool can locate definitions starting at the 'current' file and spidering outward. While working on a project, one may have code in different component libraries open,needing changes; Under the current system, each of which would have different absolute paths. So a tool needs to have seen less of the whole project structure. 

## simpler description of source-tree
eliminate the need for seperate ```mod.rs``` files within directories. Each file would do the job of mod.rs specifying further files to bring in.

# Detailed design

```use mod``` would look for a file relative to the current file.

given some source files in 
./foo.rs
./bar.rs
./baz/qux.rs
../qaz.rs

from ```foo.rs,``` the following statements
```use mod bar;```
```use mod baz::qux;```
```use mod super::qaz;```

would add those files to the project (eg, baz::qux is like saying 'load baz/qux.rs'), and make ```bar::,qux::,qaz::``` available as qualifiers to reference symbols of those files within foo.rs . 

Further ```use``` statements could give shortcuts to individual symbols, and longer paths could be written to access subtrees of these modules.

Each individual file would in turn be able to bring in its own relative files - starting from the project root, the build system would spider outward.

eg if qux.rs contained the statement ```use mod super::super::qaz;``` , ```../qaz.rs``` would be brought into the project, although 'foo.rs' would still need an additional ```use super::qaz``` to reference symbols in ```qaz.rs```.

## use mod between siblings
Symbol paths would always reflect the directory-structure: - when a series of siblings reference eachother, one would not be able to follow this graph to reach symbols.  eg if there is a relationship  a.rs->b.rs->c.rs but they are all in the same directory, there is no path ```a::b::c```, just seperate ```a::  b:: c::```

##submodules wthin files
mod {...} within a file would still be available - this is where the module heirarchy can differ from the file layout, but its assumed every file would be referenced explicityly by a ```use mod``` path. (submodules would be reached with additional ```use```'s

# Drawbacks

Replicates functionality available with use, mod and #[path=...] directives, and is a slightly different mentality to the existing system.

Might look more complicated *when used alonside the existing system* (even though its' intended as a replacement, it would require a rolling refactor)

heirachical 'use' paths have their own problems. When moving sources up or down the directory tree, refactoring would still be needed;

If this was to replace the existing use/mod behaviour, one might need references to a long string of ```use mod super::super::..::main``` statements to refer to symbols relative to the project root. 

perhaps the tree flattening effect of explicit crate files which are them imported into a project root is desirable.
(under this scheme, *every* source file that wants to refer to a particular crate conveiniently would have some ```use mod super::super..some_major_module_that_would_currently_be_a_crate```)

The behaviour of the standard library prelude might not seem as consistent with this scheme.


