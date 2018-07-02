- Feature Name: extern-existential-type
- Start Date: 2018-6-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

A version of https://github.com/rust-lang/rfcs/pull/2071's `existential type` where the definition can live in a different crate than the declaration, rather than the same module.
This is a crucial tool untangling for untangling dependencies within `std` and other libraries at the root of the ecosystem concerning global resources.

# Motivation
[motivation]: #motivation

We have a number of situations where one crate defines an interface, and a different crate implements that interface with a canonical singleton:

 - [`core::alloc::GlobalAlloc`](https://doc.rust-lang.org/nightly/core/alloc/trait.GlobalAlloc.html), chosen with [`#[global_allocator]`](https://doc.rust-lang.org/1.23.0/unstable-book/language-features/global-allocator.html)
 - `panic_fmt` chosen with [`#[panic_implementation]`](https://github.com/rust-lang/rfcs/blob/master/text/2070-panic-implementation.md)
 - The OOM hook, modified with [`std::alloc::{set,take}_alloc_error_hook`](https://doc.rust-lang.org/nightly/std/alloc/fn.set_alloc_error_hook.html)
 - `std::hash:RandomState`, if https://github.com/rust-lang/rust/pull/51846 is merged, the `hashmap_random_keys` lang item.
 - [`log::Log`](https://docs.rs/log/0.4.3/log/trait.Log.html) set with https://docs.rs/log/0.4.3/log/fn.set_logger.html

Each of these is an instance of the same general pattern.
But the solutions are all ad-hoc and distinct, burdening the user of Rust and rustc with extra work remembering/implementing, and preventing more rapid prototyping.

They also incur a run-time cost due to dynamism and indirection, which can lead to initialization bugs or bloat in space-constrained environments.
In the annotation case, there's essentially an extra `extern { fn special_name(..); }` whose definition the annotation generates.
This isn't easily inlined outside of LTO, and even then would prohibit rustc's own optimizations going into affect.
The `set`-method based ones involve mutating a `static mut` or equivalent with a function or trait object, and thus can basically never be inlined away.
So there's the overhead of the initialization, and then one or two memory dereferences to get the implementation function's actual address.
The potential bugs are due to not `set`ing before the resource is needed, a manual task because there's static way to prevent accessing the resource while it isn't set.

The `extern existential type` feature just covers the deferred definition of a type, and not the singleton itself, but that is actually enough. For example, with global allocation:

```rust
// In `alloc`

pub extern existential type Heap: Copy + Alloc + Default + Send + Sync;

struct Box<T, A: Alloc = Heap>;

impl Box<T, A: Alloc> {
    fn new_in(a: A) { .. }
}

impl Box<T, A: Alloc + Default = Heap> {
    fn new() { Self::new_in(Default::default())    }
}
```

```rust
// In `jemalloc`

#[deriving(Default, Copy)]
struct JemallocHeap;

impl Alloc for JemallocHeap {
    fn alloc(stuff: Stuff) -> Thing {
        ...
    }
}

extern existential type alloc::Heap = JemallocHeap;
```

```rust
// In a crate making an rust-implemented local allocator global.

struct MyConcurrentLocalAlloc(..);

impl Alloc for MyConcurrentLocalAlloc;

static GLOBALIZED_LOCAL_ALLOC = MyConcurrentLocalAlloc(..):

#[deriving(Default, Copy)]
struct MyConcurrentLocalAllocHeap;

impl Alloc for MyConcurrentLocalAllocHeap {
    fn alloc(stuff: Stuff) -> Thing {
        GLOBALIZED_LOCAL_ALLOC.alloc(stuff)
    }
}

extern existential type alloc::Heap = JemallocHeap;
```

By defining traits for each bit of deferred functionality (`Alloc`, `Log`), we can likewise cover each of the other use-cases.
This frees the compiler and programmer to forget about the specific instances and just learn the general pattern.
This is especially important for `log`, which isn't a sysroot crate and thus isn't known to the compiler at all at the moment.
It would be very hard to justify special casing `log` in rustc with e.g. another attribute as the problem is solved today, when it needs none at the moment.
As for the cost concerns with the existing techniques, no code is generated until the `extern existential type` is created, similar to with generic types, so there is no run-time cost whatsoever.

Many of the mechanisms listed in this RFC above are on the verge of stabilization.
This RFC doesn't want to appear to by tying things up forever, so the design strives to be simple while still being general enough.
This ought to also be forwards compatible with the more comprehensive solutions as described in the [alternatives](#alternatives) section.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

It's best to understand this feature in terms of regular `existential type`.
Type checking when *using* `pub extern existential type` works exactly the same way:
The type is opaque except for its bounds, and no traits can be implemented for it.
```rust
pub extern existential type Foo: Baz;
existential type Bar: Baz;
// both are used the same way in other modules
```
C and C++ programmers too will be familiar with the remote definition aspect of this from those language's "forward declarations" and their use in header files.

On the definition side, since it is explicitly defined (somewhere), there are no inference constraints on items in the same module as the declaration or definition.

One more interesting difference is the scope of where the type is transparent vs opaque: i.e. where can we see the type's definition, or only it's bounds.
Just as in C where one gets:
```rust
struct Foo;

// I know nothing about Foo

struct Foo { int a; };

// Ah now I do
```
when the `extern existential type` is in scope, the `existential existential type` becomes transparent and behaves as if the declaration and definition were put together into a normal type alias.
The definer can decide how one downstream gets to take advantage of it by making the definition public or not.
```rust
pub extern existential type alloc::Foo = Bar; // the big reveal
extern existential type alloc::Foo = Bar; // the tiny reveal
```
private allows the item to be used (as some definition is needed), but while no one downstream knows its true definition. like regular `existential type`.
Public allows downstream to choose between staying agnostic for increased flexibility, or peaking the hind the veil for extra functionality.
(e.g. maybe it wants to require the global allocator by jemalloc to use some special jemalloc-specific debug output.)
There are no restrictions on the type of publicity on the definitions compared to other items.

Only one crate in the build plan can define the `pub extern existential type`.
Unlike the trait system, there are no orphan restrictions that ensure crates can always be composed:
any crate is free to define the `pub extern existential type`, as long is it isn't used with another that also does, in which case the violation will only be caught when building a crate that depends on both (or if one of the crates depends on the other).
This is not very nice, but exactly like "lang items" and the annotations that exist for this purpose today,
so it is nothing worse than what's current about to be stabilized.
There is no natural orphan rules for this feature (or alternatively, regular `existential type` can be seen as this with the orphan rule that it must be defined in the same module), so this is expected.
See the first alternative for how we can use Cargo to ameliorate this.

As mentioned in the introduction, code gen can be reasoned about by comparing with generic and inlining).
We cannot generate for code for generic items until they are instantiated.
Likewise, imagine that everything that uses an `pub extern existential type` gets an extra parameter,
and then when the `impl pub extern existential type` is defined, we go back and eliminate that parameter by substituting the actual definition.
Only then can we generate code.
This is why from the root crate of compilation (the binary, static library, or other such "final" component), the dependency closure must contain an `extern existential type` for every `pub extern existential type` that's actually used.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`pub extern existential type` can also be formally defined in reference to `existential type`.
As explained in the guide-level explanation,
```rust
(pub <scope>)? extern existential type <name>: <bounds>;
```
creates an existential type alias that behaves just like a `use`d `existential type <name>: <bounds>` defined in another modules so it's opaque, while
```rust
(pub <scope>)? extern existential type <path> = <type>;
```
reveals the definition of the existential type alias at `path` as if it was a regular type alias.

There is a post-hoc coherence rule that every used `pub extern existential type` contains exactly one `impl exisitential type` definition within the entire build-plan of crates.
"used" here can be roughly defined by tracing all identifiers through their bindings, but should make intuitive sense.

There is nothing preventing private `extern existential type`, or a `impl extern type` in the same module as its `extern existential type`.
Both these situations make the feature useless and could be linted, but are well defined from the rules above.

# Drawbacks
[drawbacks]: #drawbacks

Niko Matsakis has expressed concerns about this being abused because singletons are bad.
Singletons are indeed bad, but the connection between existential types and singletons is not obvious at first sight (imagine if we had deferred definition mechanism with `static`s directly), which hopefully will make this be sufficiently difficult to abuse.
Even if we deem this very toxic, better all the use cases I listed above be white-listed and use same mechanism used for consistency (and one that is cost-free at run time), than use a bunch of separate solutions.
Also, by forcing the use of a trait in the bounds of the `extern existential type`, we hopefully nudge the user in the direction of providing a non-singleton-based way of accomplishing the same task (e.g. local allocators in addition to the global allocator).

Stabilization of many annotations and APIs called out in the [motivation](#motivation) section is imminent, and yes this would delay that a bit if we opted to do this and then rewrite those APIs to use it.

As per the [prior art](#prior-art) section, something like Haskell's backpack is wholly superior.
But as stabilization of the status quo is imminent, I wanted to pick something easier to implement and closer to existing rust features mentally/pedagogically.

# Rationale and alternatives
[alternatives]: #alternatives

- We can additionally mandate that `Cargo.toml` include all `extern existential type` declarations and definitions, and Cargo reject any build plan where they don't match 1-1.
  This ameliorates the crate composition issue in practice for the vast majority of users using Cargo (even just `Cargo.toml`s).

- Of course, we can always do nothing and just keep the grab bag of ad-hoc solutions we have today, and leave log with just a imperative dynamic solution.

- We could continue special-casing and white-listing in the compiler the use-cases I give in the motivation, but at least use the same sort of annotation for all of them for consistency.
  But that still requires leaving out `log`, or special casing it for the first time.
  As bad as I agree singletons are, I imagine a few yet-unforseen use-cases for this (e.g. for peripherals for bare-metal programming, which are morally singletons) arising.
  So that leaves other special-cases we would need to add in the future.

- I mention just doing this would delay stabilization.
  But, we could also retrofit those annotations as desugaring into this feature so as not to delay it.
  This keeps around the crust in the compiler forever, but at least we can deprecate the annotations in the next edition.
  I don't think it's worth it to bend over backwards for something that is still unstable, and consider it unwise to so "whiplash" the ecosystem telling them to use one stable thing and then immediate another, but for those that really want to stabilize stuff, this is an option.

- In many cases, the `extern existential type` would just be a ZST proxy used in a default argument.
  If we could add default arguments to existing type parameters, then the original items wouldn't need an abstract stand-in.
  @eddyb and others have thought very hard about this approach for many years, and it doesn't seem possible, however.

See the [prior art](#prior-art) section below for context on the last two.

- We couldn't do exactly ML's functors for this problem, because people both want to import `std` without passing in a global allocator, yet also be able to use `std` with different global allocators.

- I opted out from proposing exactly Haskell' backpack because of the perceived time pressure mentioned above, but it's straightforward to imagine `extern existential type` being convenient sugar for some more general parameterized module system, similar to `impl Trait` and regular `existential type`.

# Prior art
[prior-art]: #prior-art

The basic idea come from the "functors" of the ML family of languages, where a module is given explicit parameters, like
```rust
mod foo<...> { ... }
```
in Rust syntax, and then those modules can be applied like functions.
```rust
mod bar = foo<...>;
```

More appropriate is Haskell's new [backpack](https://plv.mpi-sws.org/backpack/) modules system, where the parameterization is not explicit in the code (`use`d modules may be resolved or just module signatures, in which case they act as parameters), and Cabal (the Cargo equivalent), auto-applies everything.
This would work for Rust, and in fact is wholly better:

 - It is more expressive because modules can be applied multiple times like ML and unlike this.

 - There is still no syntactic overhead of manual applications at use sites, like this and unlike ML.

 - Cabal, with it's knowledge of who needs what, can still complain early if something would be defined twice / two different instantiations do not unify as a downstream crate needs, like the first alternative.

[That latter issue problem is not possible here under the single-instantiation rule.]

# Unresolved questions
[unresolved]: #unresolved-questions

- The exact syntax. "existential" is a temporary stand-in from https://github.com/rust-lang/rfcs/pull/2071, which I just use here for consistency. I personally prefer "abstract" FWIW.

- Should Cargo have some knowledge of `extern abstract type` declarations and definitions from the get-go so it can catch invalid build plans early?
