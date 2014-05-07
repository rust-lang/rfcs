- Start Date: 2014-06-16
- RFC PR #:
- Rust Issue #:

# Summary

Add syntax sugar for type parameters and bounds as a prefix to a generic item
definition proper: `type <T: Rand> fn random() -> T { ... }`

# Motivation

The motivation is pretty much code like the following, taking an (arguably
dated and biased) example from `librustc/middle/typeck/infer/lattice.rs`,
reindented to match [the style guide](
https://github.com/mozilla/rust/wiki/Note-style-guide#function-declarations):

```rust
pub fn lattice_vars<L:LatticeDir + Combine,
                    T:Clone + InferStr + LatticeValue,
                    V:Clone + Eq + ToStr + Vid + UnifyVid<Bounds<T>>>(
                    this: &L,
                    a_vid: V,
                    b_vid: V,
                    lattice_dir_op: LatticeDirOp<T>)
                    -> cres<LatticeVarResult<V,T>> {
    [...]
}

pub fn lattice_var_and_t<L:LatticeDir + Combine,
                         T:Clone + InferStr + LatticeValue,
                         V:Clone + Eq + ToStr + Vid + UnifyVid<Bounds<T>>>(
                         this: &L,
                         a_id: V,
                         b: &T,
                         lattice_dir_op: LatticeDirOp<T>)
                         -> cres<T> {
    [...]
}
```

I propose syntax sugar to let us write that as

```rust
type <L:LatticeDir + Combine,
      T:Clone + InferStr + LatticeValue,
      V:Clone + Eq + ToStr + Vid + UnifyVid<Bounds<T>>>
pub fn lattice_vars(this: &L,
                    a_vid: V,
                    b_vid: V,
                    lattice_dir_op: LatticeDirOp<T>)
                    -> cres<LatticeVarResult<V,T>> {
    [...]
}

type <L:LatticeDir + Combine,
      T:Clone + InferStr + LatticeValue,
      V:Clone + Eq + ToStr + Vid + UnifyVid<Bounds<T>>>
pub fn lattice_var_and_t(this: &L,
                         a_id: V,
                         b: &T,
                         lattice_dir_op: LatticeDirOp<T>)
                         -> cres<T> {
    [...]
}
```

and alternatively

```rust
type <L:LatticeDir + Combine,
      T:Clone + InferStr + LatticeValue,
      V:Clone + Eq + ToStr + Vid + UnifyVid<Bounds<T>>> {
    pub fn lattice_vars(this: &L,
                        a_vid: V,
                        b_vid: V,
                        lattice_dir_op: LatticeDirOp<T>)
                        -> cres<LatticeVarResult<V,T>> {
        [...]
    }
    pub fn lattice_var_and_t(this: &L,
                             a_id: V,
                             b: &T,
                             lattice_dir_op: LatticeDirOp<T>)
                             -> cres<T> {
        [...]
    }
}
```

The simple form of a function definition is retained in the face of complex
type shenanigans. The function name isn't unduly separated from the value
parameter list. At a glance, it's obvious where the type parameter list ends
and where the value parameter list begins and ends and where the body begins,
even with long lists of either kind.

Type parameters and bounds common to multiple items only have to be written
once, making structural similarity between items clearly visible.

Having type parameter lists with bounds show up between a function name and the
value parameter list isn't common in mainstream languages, Rust is the odd one
out in that many type bounds blow up a function definition this much. Shallow
syntactic strawman translations:

```c++
// C++ (no bounds, doesn't really count, but it's the syntactic inspiration, so...)
template <typename L, typename T, typename V>
cres<LatticeVarResult<V,T>>
lattice_vars(L& self, V a_id, V b_id, LatticeDirOp<T> lattice_dir_op) {
    [...]
}
```

```java
// Java
class Foo {
  static <L extends LatticeDir & Combine,
          T extends Clone & InferStr & LatticeValue,
          V extends Clone & Eq & ToStr & Vid & UnifyVid<Bounds<T>>>
  CRes<LatticeVarResult<V,T>>
  latticeVars(L self, V aId, V bId, LatticeDirOp<T> latticeDirOp) {
    [...]
  }
}
```

```c#
// C#
class Foo {
  static
  CRes<LatticeVarResult<V,T>>
  LatticeVars<L, T, V>(L self, V aId, V bId, LatticeDirOp<T> latticeDirOp)
    where L : LatticeDir where L : Combine
    where T : Clone where T : InferStr where T : LatticeValue
    where V : Clone where V : Eq where V : ToStr where V : Vid where V : UnifyVid<Bounds<T>>
  {
    [...]
  }
}
```

```haskell
-- Haskell
latticeVars :: LatticeDir L, Combine L,
               Clone T, InferStr T, LatticeValue T,
               Clone V, Eq V, ToStr V, Vid V, UnifyVid (Bounds T) V
               => L -> V -> V -> LatticeDirOp T -> CRes (LatticeVarResult V T)
latticeVars this aId bId latticeDirOp = [...]
```

I don't know any OCaml but I'm confident that it fares better here too. I'm not
sure how D works for function templates, but they also have a mode that sorta
looks like `template TemplateName(T, U, V, etc) { some decls that use T, U, V
etc go here }`.

# Drawbacks

  * Two ways to do the same thing (or, if the other way goes away, the simple
    case of `fn f<OnlyOneTWithNoBounds>()` becomes more verbose: `type <T> fn
    f()`).

  * "Declaration mirrors use" doesn't really apply anymore. We already put
    up with `::<`, and other languages get away without declarations mirroring
    use, but it would be sad anyway.

# Detailed design

Items adorned with prefix-style type parameter lists desugar into regular
generic items with that same type parameter list. Blocks with multiple items
after a single prefix-style type parameter list desugar into a series of
regular generic items, not inside of any block or mod-like grouping, with that
same type parameter list, with the exception that type parameters not used by
all items get omitted in the type parameter lists for the items where they are
not used.

In the single-item case, attributes still come first. In the block case,
attributes on the `type<>` syntax element or inner attributes directly inside
the block are not allowed (items must be attributed individually).

The block isn't a real block, it doesn't have non-item statements or
expressions inside it even if it is itself inside a function body, it is just a
list of items delimited by `{`, `}`. All items in the block must refer to at
least one type parameter in the prefix-style type parameter list but must not
themselves have a type parameter list, all type parameters in the prefix-style
type parameter list must be used by at least one item.

This applies for all items that can have type parameter lists (but see below).
By example:

`type <T: Send> struct SendableThing(T);` => `struct SendableThing<T: Send>(T);`

`type <T: Show> type ShowVec = Vec<T>;` => `type ShowVec<T: Show> = Vec<Show>;`

`type <T: SomeTrait> impl SomeOtherTrait for MyType<T> { ... }`
=> `impl<T: SomeTrait> SomeOtherTrait for MyType<T> { ... }`

# Alternatives

  * Do nothing. Worked so far, probably won't be a problem for most people.
    Pretty safe choice.

  * Try to do vaguely this via a macro. That seems like a bad plan because it
    would probably have to reproduce most of the supported items' declaration
    syntax to properly insert the type parameters and wouldn't work so well in
    the case that not all items use all the type parameters specified.

  * Compromise on a C#-like syntax where the type variables are introduced
    first and bounds are listed later. It's uglier for functions because the
    return type already comes roughly in that syntactic position and it
    interferes with line-wrapping/indenting again, but even then we can
    probably come up with something more compact than all these `where`s. It's
    more verbose than this proposal and doesn't include the case of multiple
    generic items with just one type parameter list, but on the upside it's an
    excuse to claim the `where` keyword. It would avoid both drawbacks above.

  * Change syntax for generic functions to `fn<T: Rand> random() -> T`. This
    sucks because it breaks greppability of `fn functionnamehere` but on the
    other hand it doesn't impact other generic items.

# Unresolved questions

  * The concrete keyword was a random choice out of the list that sort of fit.
    Could use a new keyword, could use `for` or `let` instead since that reads
    nicely ("for a T that is Sendable/let T be Sendable, then `spawn_with` is a
    function that...") or even some `let <T: Send> in [item goes here]`
    construct, could use no keyword like Java sort of does. No strong
    preference.

  * I haven't really thought about lifetime parameters, I hope this all works
    out if you pretend I said "type or lifetime" every time I said "type".  But
    maybe lifetime parameters should be allowed to be introduced individually
    even within the block-style syntax since I suspect they are less likely to
    be common between multiple items.

  * If the proposed sugar actually turns out to be popular, we could remove
    current generics syntax so that there's only one way to do it.  This sounds
    like a bad idea for simple cases. (`struct Foo<T>`, `type Foo<T> = ...;`)
