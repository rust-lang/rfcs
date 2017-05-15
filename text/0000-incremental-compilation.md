- Start Date: 2015-01-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
This RFC proposes an incremental compilation strategy for `rustc` that allows for translation, codegen, and parts of static analysis to be done in an incremental fashion, without precluding the option of later expanding incrementality to parsing, macro expansion, and resolution.

In the C world, source code is split up into source files and header files, where source files are the unit of (re)compilation and header files contain 'interface' information that is needed to compile source files independently of each other. This RFC proposes an algorithm that could be described as (figuratively) splitting a Rust codebase into a set of source files, computing the minimal 'header/interface' information for each virtual source file and then using this information to determine if the object code for a given virtual source file (cached from a previous compiler invocation) can safely be re-used without having to recompile the source file.
(Note: This is just a metaphor, no actual header or source files are generated)

# Motivation
At the moment `rustc` takes a long time to compile anything but trivial programs.

# Detailed design
For making compilation incremental, the compiler must be changed in two ways:

1. The compilation process must produce artifacts that can be (re)compiled and cached independently of each other.
2. The compiler must track dependencies between the items in a program, so it can infer which artifacts to recompile and which to re-use from a previous compilation.

## Independent Compilation Artifacts
In order for the compiler to be able to re-use compilation artifacts from previous runs, these artifacts must be as independent of each other as possible. I propose to make the unit of (re)compilation individual functions and globals, that is, everything that ends up as a symbol in the output binary. At least the LLVM IR and object code of each function and global is cached, so the cache has two entries for every symbol, one in the form of object code, one in the form of LLVM IR.

Why object code? Because it is the final output of the compiler and can be passed immediately to the linker without having to re-run the costliest compiler passes: trans and codegen.

Why LLVM IR? Because if a fully optimized crate build is needed, it allows to generate an LLVM IR version of the crate without re-running parts of type-checking and trans. The individual IR fragments can be concatenated and LLVM can do its optimization passes on the whole crate then. 

It might also make sense to cache other things in between compiler runs (e.g. important lookup tables) but the two above are certainly the most important ones. I will refer to the cached data for some symbol as `compilation artifact` in the remainder of the document.

## Dependency Tracking
First, lets define some terms that are useful for talking about the topic:

- A `program item` is any kind of function-, type-, or global variable definition. This includes functions, types, and statics defined locally within other functions and associated type assignments in `impl` blocks. It does not include modules, local variables, statements or expressions.
- A `program item interface` is that part of a `program item` that may be relevant for compiling *other* `program items`. Types are 'all interface' while for functions it's only the signature and for statics it's their type.
- A `program item body` is the full definition of a `program item`, i.e. the full functions definition, including the function body, respectively the whole global variable definition, including the initializer.

`Program items` can *depend* on each other. Let's define what that means more formally:

- A program item `A` *depends* on a program item `B` iff a change to `B` means that `A` needs to be re-compiled.

With these terms in place we can state that compiling a single `program item` will thus depend 

1. on its own `program item body`, and
2. the `program item interfaces` of any `program items` (types, traits, functions, methods, globals) it **transitively** references

The dependency structure of a `program item` can be modeled as a directed graph:

* For the `program item body` there is a node in the graph. Note that `program items` that don't have a body (structs, enums, traits, ...) are only represented by their 'interface node' in the graph.
* For each `program item interface` there is one node in the graph.
* If a `program item interface` `A` directly references another `program item` `B` then there is an edge from `A`'s graph node to `B`'s interface node. That is, whenever the name of a type, function, or global occurs, there is an edge to the interface node of that type, function, or global.
* If a `program item body` `A` references another `program item` `B` then there is an edge from `A`'s graph node to `B`'s interface node.
* There's always an edge from a `program items` body node to its interface node.

Let's illustrate this with an example:

```rust
struct Kid {
  name: &'static str
}

struct Tiger {
  name: &'static str
}

struct Dinosaur {
  name: &'static str,
  stomach_contents: Gastropod
}

struct Gastropod {
  name: &'static str,
  height_in_stories: u32
}

fn transmogrify(kid: Kid) -> Tiger {
    let intermediate_dinosaur = trans_internal(kid);
    Tiger { name: intermediate_dinosaur.name }
}

fn trans_internal(kid: Kid) -> Dinosaur {
    Dinosaur {
        name: kid.name,
        stomach_contents: Gastropod {
            name: "Larry".to_string(),
            height_in_stories: 500
        }
    }
}

fn main() {
    let calvin = Kid { name: "Calvin" };
    let hobbes = Tiger { name: "Hobbes" };

    let calvin = transmogrify(calvin);

    assert!(calvin < hobbes);
}
```

The dependency graph of the above program looks like this:

```                
              Gastropod <--- Dinosaur  +--> Kid  Tiger  
                 ^               ^     |     ^     ^     
                 |               |     |     |     |     
                 |               |     |     |     |     
                 |               |     |     |     |     
                 |   trans_internal ---+   transmogrify   main
INTERFACES       |       ^    ^                ^     ^       ^
-----------      |       |    |                |     |       |
BODIES           |       |    +---------+      |     +----+  |
                 |       |              |      |          |  | 
               trans_internal'        transmogrify'       main'

```

Note that, for readability, I've omitted some redundant edges in the above graph. An edge between two nodes can be omitted if there is another path between those nodes (as in `trans_internal' --> Kid`). More formally, two dependency graphs are equivalent for our purposes if their transitive closures are equal.

This dependency graph can be queried for finding all `program item interfaces` that a `program item` `A` depends on: Start at the node corresponding to `A` and collect all transitively reachable nodes. In the above example this means that `main()` needs to be recompiled if `Kid`, `Tiger` or the interface of `transmogrify` change (because these are reachable from `main`'s body node) but not if `Dinosaur`, `Gastropod` or any of the other function's body changes (because their nodes are not reachable from `main`'s body).

## Generic Program Items
So far the dependency graph has only described non-generic `program items`. For generic definitions the situation is a bit more complicated since their dependencies are only fully defined once all type parameters are substituted for concrete arguments. Consider the following example:

```rust
trait Transmogrifiable<T> {
    fn transmogrify(self) -> T;
}

impl Transmogrifiable<Tiger> for Kid {
    fn transmogrify(self) -> Tiger { ... }
}

impl Transmogrifiable<Gastropod> for Dinosaur {
    fn transmogrify(self) -> Gastropod { ... }
}

fn transmogrify<TFrom, TTo>(val: TFrom) -> TTo 
    where TFrom: Transmogrifiable<TTo> 
{
    val.transmogrify()        
}
```

In this example `transmogrify<Kid, Tiger>` will have a different dependency graph than `transmogrify<Dinosaur, Gastropod>`. In other words, the monomorphized implementation of `transmogrify<Kid, Tiger>` is not affected if the definition of `Dinosaur` or `Gastropod` changes and the dependency graph should reflect this.

One way to model this behavior is by not creating dependency graph nodes for generic `program items` but `node templates`, which---like generic items---have type parameters and yield a concrete, monomorphic dependency graph node, if all type parameters are substituted for concrete arguments. When the need arises to check if a particular monomorphized function implementation from the cache can be re-used, the dependency graph for the function can be constructed on demand from the given `node template` and parameter substitutions.

For consistency it also makes sense to treat non-generic `program items` as generic `program items` that happen to have zero type parameters. We thus obtain a direct correspondence between `program items` and `node templates` on the one hand and monomorphized `program item` instances and dependency graph nodes on the other.

## Program Item and Compilation Artifact Identifiers
`Program items` and their corresponding `node templates` must be identifiable in a way that is stable across multiple invocations of the compiler. The `DefId` type that is currently used for cross-crate item identification does not fit this requirement unfortunately, since it contains the rather unstable AST `NodeId`. Adding a single AST node currently can invalidate all `NodeId`s in the codebase due to the sequential Node ID assignment strategy.

One straightforward way of creating stable identifiers would be to use the `program item`'s path within the AST, while generating local integer IDs for anonymous blocks:

```rust
// ID: "M1"
mod M1 { 

    // ID: "M1::f1"
    fn f1() {

        // ID: "M1::f1::S1"
        struct S1;

        {
            // ID: "M1::f1::0::S2"
            struct S2;

        }

        {
            // ID: "M1::f1::1::E1"
            enum E1;
        }
    }

    impl SomeTrait for SomeType {
        fn something() { // ID <SomeType as M1::SomeTrait>::something
            ...
        }
    }
}
```

The dependency tracking system as described above contains `node templates` for `program item` definitions on a syntactic level, that is, for each `struct`, `enum`, `type`, `trait`, there is one `node template`, for each `fn`, `static`, and `const` there are two (one for the interface, one for the body). However, as seen in the section on generics, the codebase can refer to monomorphized instances of program items that cannot be identified by a single identifier as described above. A reference like `Option<String>` is a composite of multiple `program item` IDs, a tree of program item IDs in the general case:

```rust

Option<Result<u32,GenericError<String>>>

            Option
              |
            Result
            /   \
         u32     GenericError
                     |
                  String 
```

Incidentally, it is also such a composite ID that is needed for identifying `compilation artifacts` within the cache, since the cache only stores monomorphized instances of generic functions. For want of a better name I'll call it `mono-id` as in "identifier for a monomorphized type, function, trait, ...". Non-generic `program items` are just a special case of generic ones and need no special treatment.

## Determining If A Cache Entry Is Still Valid
This is the central question we want answered from the dependency tracking system: Given the need to include the object code of some (monomorphized) `program item` in the output binary, can we just reuse the object code already stored in the cache? This question can be answered by constructing the monomorphized dependency graph from the given `mono-id` and check each node for source code changes between cached and current version. In pseudo-rust:

```rust

trait MonoId {
    fn definition_id(&self) -> ProgramItemId;
    fn type_arguments(&self) -> &Map<TypeParam, MonoId>;  
}

trait NodeTemplate {
    // A fingerprint of the relevant parts of the resolved AST
    fn description(&self) -> Fingerprint;
    fn instantiate(&self, type_arguments: &Map<TypeParam, MonoId>) -> &Node;
}

trait Node {
    fn dependencies(&self) -> &[MonoId];
}

fn has_interface_changed(
    id: MonoId,
    interface_node_templates: &Map<ProgramItemId, NodeTemplate>,
    cache: &Cache)
 -> bool {
    
    let node_template = interface_node_templates.get(id.definition_id);

    if node_template.description() != 
       cache.load_interface_description(id.definition_id()) {
        return true;
    }

    let node = node_template.instantiate(id.type_arguments());
    for dependency_id in node.dependencies() {
        if has_interface_changed(dependency_id, node_templates, cache) {
            return true;
        }
    }

    return false;
}

fn is_cached_implementation_still_valid(
    id: MonoId,
    interface_node_templates: &Map<ProgramItemId, NodeTemplate>,
    body_node_templates: &Map<ProgramItemId, NodeTemplate>,
    cache: &Cache)
 -> bool {
    
    let node_template = body_node_templates.get(id.definition_id());
    
    if node_template.description() != 
       cache.load_body_description(id.definition_id()) {
        return false;
    }

    let node = node_template.instantiate(id.type_arguments());

    for dependency_id in node.dependencies() {
        if has_interface_changed(dependency_id, node_templates, cache) {
            return false;
        }
    }

    return true;
}

// Handling of cycles in dependency graph has been omitted for clarity.
```


## Incremental Compilation Algorithm
With these things in place, we can construct an algorithm for building a crate incrementally. 

```
(1) Parse, Expand, Resolve whole crate
(2) Run type inference on all program items
(3) Build NodeTemplate map 
(4) Add all monomorphic program items to set of needed compilation
    artifacts
(5) Ensure that all needed compilation artifacts are in the cache:
    (1) Take mono-id from list of needed compilation artifacts
    (3) Construct dependency graph to determine whether cache entry
        can be reused
    (4) If not, compile item
    (5) Add item fingerprint and compilation artifacts to cache
    (6) If new monomorphized item instances are discovered during
        compilation, add them to list of needed compilation artifacts
(6) Clear unreferenced entries from the cache
(7) Create output binary
    (a) Link cached object files into output binary, or
    (b) Concatenate cached IR fragments, do optimization and codegen,
        then link
```


## Miscellaneous Aspects

### More fine-grained dependency tracking
It would be possible to make dependency tracking aware of the kind of reference one item makes to another. If an item `A` mentions another item `B` only via some reference type (e.g. `&T`), then item `A` only needs to be updated if `B` is removed or `B` changes its 'sized-ness'. This is comparable to how forward declarations in C are handled. In the dependency graph this would mean that there are different kinds of edges that trigger for different kinds of changes to items.

### Global Switches Influencing Codegen
There are many compiler flags that change the way the generated code looks like, e.g. optimization and debuginfo levels. A simple strategy to deal with this would be to store the set of compiler flags used for building the cache and clearing the cache completely if another set of flags is used. Another option is to keep multiple caches, each for a different set of compiler flags (e.g. keeping both on disk, a 'debug build cache' and a 'release build cache').

### Source Location Encoding
The current `codemap` implementation in `rustc` stores everything in one global 'address space'. This is unsuited for incremental compilation since old versions of source files need to be removed from the codemap and new source files need to be added. This leads to a similar problem as the sequentially assigned AST node IDs.

It would be better to store span information in byte-offsets relative to a given source file. Then modifying the codemap is possible without invalidating existing source locations stored somewhere in the cache.

### Physical Cache Structure
The cache could be kept somewhere within the output directory. Object code could be maintained in an `ar` archive so the linker can directly access it. Something similar might be possible for LLVM bitcode.

### Automatic inter-function optimizations
It should not be too hard to let the compiler keep track of which parts of the program change infrequently and then let it speculatively build object files with more than one function in them. For these aggregate object files inter-function LLVM optimizations could then be enabled, yielding faster object code at little additional cost. Other strategies for controlling cache granularity can be implemented in a similar fashion.

### Parallelization
If some care is taken in implementing the above concepts it should be rather easy to do translation and codegen in parallel for all items, since by design we already have (or can deterministically compute) all the information we need.

# Drawbacks

Implementing this will need a lot of work on the compiler architecture. But that's something that will be needed sooner or later anyway.

# Alternatives

I'd definitely like to hear about them.

# Unresolved questions

## Dependency Graph Construction before Type Inference
I'm not sure if it would be possible to construct valid dependency graphs *before* type-inference or if that would miss some dependency edges. Or more generally, how much per-item work can be pushed until after caching strikes.

## RLIB/DYLIB metadata
I have not investigated how library metadata will be affected by this. I guess it must be made 'linkable' in some way or other.

## Debuginfo Redundancies
There might be a lot of debuginfo redundancies in cache entries because type debuginfo will be duplicated for each function that transitively refers to the type. Might use up a lot of disk space and make things slower than they need to be...

A lot more of these questions will probably pop up during implementation.
