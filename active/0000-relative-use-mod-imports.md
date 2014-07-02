- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

introduce ```use mod ...;``` or ```import ...;``` as a simultaneous module import and namespace 'use', with relative module paths, which have a 1:1 mapping to relative filename paths.

Creates a graph of imports between files as in other module systems, but still mounts modules heriarchically.

```use mod``` brings a module into scope along with the hint: "this module is an actual file"




# Motivation

## avoids repeating information

This idea exploits coherence between the module heirarchy and the filesystem directory tree - but it *assumes* this coherence, instead of relying on the user to manually *create* it with 'mod.rs' files. So the information you give when 'bringing things into scope' should be enough to specify what to load.


## versatility for compile units

Consider moving between the extremes of one compilation unit per file, vs an entire project as a single compilation unit - with the existing use/mod behaviour, you would have to refactor how modules are brought in and how components are referenced.

a faster debug build with less inlining might be possible with smaller translation units; or you want to switch to a single translation unit (like C++ unity builds) for the maximum optimization possible.

Relative paths would allow greater flexibility when wanting to treat project subtrees as seperate libraries, or vica versa. eg. for building examples demonstrating components of an SDK, or a single source tree building a suite of tools.

A build system would be at liberty to cache any appropriate subtree equivalently to a library crate with no pre-planning on the users' part.


## shorter learning curve
The seperate absolute and relative paths, and mod / use statements are a tripping point for new users. Under this scheme, you only see relative paths, and you only need one statement 'use mod '.

eliminates the need to create seperate ```mod.rs``` files within directories. Each file would do the job of mod.rs specifying further files to bring in.

## parallelize --test
This might be useful for compiling tests, eg it would be theoretically possible to start at any individual file and test it, in isolation from the whole-project build. So building for test could be done across more cores.

## tooling
with a project setup this way, a tool can locate definitions starting at any 'current' file and spidering outward. While working on a project, one may have source from different component libraries open; Under the current system, each of which would have different addressing scheme, relative to its own crate root. Under this scheme, a tools needs to know less about the whole project to give consistent help to the user.

# Detailed design

```use mod``` would look for a file relative to the current file.

given some source files: 

    foo.rs
    bar.rs
    baz/qux.rs
    ../qaz.rs

from ```foo.rs,``` the following statements

    use mod   bar;
    use mod   baz::qux;
    use mod   super::qaz;

would add ```foo.rs,bar.rs,qux.rs,qaz.rs ``` to the project (eg, baz::qux is like saying 'load baz/qux.rs'), and make ```bar::,qux::,qaz::``` available as qualifiers to reference symbols of those files, within foo.rs . 

This would work regardless whether ```foo.rs``` was the crate root or further down the tree.

Further ```use``` statements could give shortcuts to individual symbols within foo.rs, and longer paths could be written to access subtrees of these modules.

Each individual file would in turn be able to bring in its own relative files - starting from the project root, the build system would spider outward.

eg if qux.rs contained the statement ```use mod super::super::qaz;``` , ```../qaz.rs``` would be brought into the project, although 'foo.rs' would still need an additional ```use super::qaz``` to reference symbols in ```qaz.rs```.

## use mod between siblings
Symbol paths would always reflect the directory-structure: - when a series of siblings reference eachother, one would not be able to follow this graph to reach symbols.  eg if there is a relationship  a.rs->b.rs->c.rs but they are all in the same directory, there is no path ```a::b::c```, just seperate ```a::  b:: c::```

##submodules wthin files
mod {...} within a file would still be available - this is where the module heirarchy can differ from the file layout, but its assumed every file would be referenced explicityly by a ```use mod``` path. (submodules would be reached with additional ```use```'s

## use vs use mod
if  it wasn't for the existence of submodules, would it be possible to infer load information entirely from relative use directives, and individual qualified symbols ? However this system relies on "use mod" as a hint, "this module is a file"

# Drawbacks

The behaviour of the standard library prelude might not seem as consistent with this scheme.

Replicates functionality available with use, mod and #[path=...] directives, and is a slightly different mentality to the existing system.

Might look more complicated *when used alonside the existing system* (even though its' intended as a replacement, it would require a rolling refactor)

heirachical 'use' paths have their own problems. When moving sources up or down the directory tree, refactoring would still be needed; Rust supposedly already moved from relative to absolute.

If this was to replace the existing use/mod behaviour, one might need references to a long string of ```use mod super::super::super::...::main``` statements to refer to symbols relative to the project root. 

perhaps the tree flattening effect of explicit crate files which are them imported into a project root is desirable.
(under this scheme, *every* source file that wants to refer to a particular crate conveiniently would have some ```use mod super::super..some_major_module_that_would_currently_be_a_crate```)

if modules down the graph did import files earlier in the tree, the tool would have to warn you about this and possibly dissalow when you compile a subtree as a library crate.



