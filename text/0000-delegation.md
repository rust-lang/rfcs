- Feature Name: delegation
- Start Date: 2018-04-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Syntax sugar for efficient code reuse via the composition pattern. Wrapper functions are generated for a struct, delegating most or all of a trait’s `impl` block to a member field that already implements the trait.


# Motivation
[motivation]: #motivation

Let's consider some existing pieces of code:
```rust
// from rust/src/test/run-pass/dropck_legal_cycles.rs
impl<'a> Hash for H<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}
```
We can see a recurring pattern where the implementation of a method only consists in applying the same method to a subfield or more generally to an expression containing `self`. Those are examples of the well known [composition pattern][object_composition]. It has a lot of advantages, but unfortunately requires writing boilerplate code again and again. In a classical OOP language we could also have opted for inheritance for similar cases. Inheritance comes with its own bunch of problems and limitations but at least it allows a straightforward form of code reuse: any subclass implicitly imports the public methods of its superclass(es).

One of the issues frequently mentioned when newcomers are learning Rust, is "I can do this easily in OOP, but is it even possible in Rust? And how the heck do I do it?" The lack of documentation/guides on this is a known issue and is being worked on, but it's not *just* a documentation issue.

OOP is used to solve real problems and if we want people to choose Rust in any of the domains they proliferate in, we need good solutions for those problems. As @withoutboats said:

> One aspect of inheritance-based polymorphism systems is that it is very easy to re-use code that has already been written as you are extending a system. The specific mechanism of re-use is connected to the more detrimental aspects of inheritance - the way it can result in ill-considered coupling and overly gregarious sharing of details which should be abstracted away.
> 
> To avoid the pitfalls that many inheritance-based languages have fallen into, Rust has avoided that form of polymorphism entirely, preferring instead a combination of behaviorally constrained parametric polymorphism (traits and generics) and straightforward type composition.
> 
> Avoiding inheritance has resulted in two major costs for users of Rust:
> 
> - There are patterns enabled by inheritance which have no clean equivalent in Rust.
> - The forms of abstraction Rust suggests can result in more boilerplate and less convenient code re-use than inheritance.
> 
> We've focused a lot of attention on resolving the first problem, which is what has driven the efforts around specialization and the like. But there hasn't been nearly as much attention at resolving the second problem. This RFC is aimed squarely at that problem, so thanks @contactomorph for considering that question.

Efficient code reuse is about making us more productive, not just in terms of typing less, which is nice, but being able to clearly express *intent* in our code, making it easier to read, understand, refactor and prototype quickly. It also enables DRY and is in-line with the [2017 Roadmap](https://github.com/rust-lang/rust-roadmap): 

> In short, productivity should be a core value of Rust. By the end of 2017, let's try to earn the slogan: 
> 
> - Rust: fast, reliable, productive—pick three.

Rust has no inheritance (yet) and as a result composition is an even more interesting pattern for factoring code than in other languages. In fact it is already used in many places. Some (approximate) figures:

Project           | Occurences of "delegating methods" |
------------------| ---------------------------------- |
rust-lang/rust    | 845                                |
rust-lang/cargo   | 38                                 |
servo/servo       | 314                                |

Functional programmers like @Centril love delegation and absolutely adore that they can do the following in Haskell with `{-# GeneralizedNewtypeDeriving #-}`:

```haskell
newtype NormT m (a :: *) = NormT { _runNormT :: WriterT Unique m a }
  deriving ( Eq, Ord, Show, Read, Generic, Typeable
           , Functor, Applicative, Monad, MonadFix, MonadIO, MonadZip
           , Alternative, MonadPlus, MonadTrans, MFunctor, MMonad
           , MonadError e, MonadState s, MonadReader r
           , MonadWriter Unique )
```

This is massive code reuse and not in any OOP language ^,-

By providing syntax sugar for the composition pattern, it becomes a privileged tool for code reuse while being as terse as the inheritance-based equivalent. It could also enable ergonomic implementation of custom widgets in a pure Rust GUI library.

Related discussions:
 
* [Initial pre-RFC][pre_rfc]
* [Reddit thread][comp_over_inh]
* [Initial RFC][initial_rfc]
* [Post-RFC design discussion][design_discussion]
* [Internals thread for this RFC][internals_thread]

[pre_rfc]: https://internals.rust-lang.org/t/syntactic-sugar-for-delegation-of-implementation/2633
[comp_over_inh]: https://www.reddit.com/r/rust/comments/372mqw/how_do_i_composition_over_inheritance/
[object_composition]: https://en.wikipedia.org/wiki/Composition_over_inheritance
[initial_rfc]:
https://github.com/rust-lang/rfcs/pull/1406
[design_discussion]:
https://internals.rust-lang.org/t/3-weeks-to-delegation-please-help/5742
[internals_thread]: https://internals.rust-lang.org/t/new-rfc-for-delegation-anyone-interested-in-contributing/6644

# Guide-level explanation
[guide]: #guide

In Rust, we prefer composition over inheritence for code reuse. For common cases, we make this convenient with delegation syntax sugar.

Whenever you have a struct `S` with a member field `f` of type `F` and `F` already implements a trait `TR`, you can delegate the implementation of `TR` for `S` to `f` using the keyword `delegate`:

```rust
impl TR for S {
    delegate * to self.f;
}
```

This is pure sugar, and does exactly the same thing as if you “manually delegated” all the `fn`s of `TR` like this:

```rust
impl TR for S {
    fn foo(&self) -> u32 {
        self.f.foo()
    }
    fn bar(&self, x: u32, y: u32, z: u32) -> u32 {
        self.f.bar(x, y, z)
    }
    // ...
}
```

To delegate most of a trait, rather than all of it, simply `delegate *` and then write the manual implementations for the items you don’t want to delegate.

```rust
impl TR for S {
    delegate * to self.f;

    fn foo(&self) -> u32 {
        42
    }
}
```

Aside from the implementation of `foo()`, this has exactly the same meaning as the first example.

If you only want to delegate specific items, rather than “all” or “most” items, then replace `*` with a comma-separated list of only the items you want to delegate. Since it’s possible for types and functions to have the same name, the items must be prefixed with `fn`.

```rust
impl TR for S {
    delegate fn foo, fn bar
        to self.f;
}
```

This also has the exact same meaning as the first example. While `to field_name;` is not required to be on a new line, this is the `rustfmt` default when delegating individual trait items, as it makes it easy to visually identify the field being delegated to.

In addition to "regular" structs with named fields, you can delegate to indexed fields on a tuple struct:

```rust
struct AddOnlyCounter(u32);

impl AddOnlyCounter {
    fn new() -> Self {
        Self { 0 }
    }
}

impl AddAssign for AddOnlyCounter {
    delegate * to self.0;
}
```

By using the _newtype pattern_, we _restrict_ the behaviour of the original type and only allow semantically valid methods/operations. If `AddOnlyCounter` is defined in another module, users can't break its guarantees, as the `u32` is private aka encapsulated.

```rust
let counter = AddOnlyCounter::new();

// this works:
counter += 1;

// these are compile-time errors:
counter = 5;
counter -= 1;
counter.0 -= 1;
```


# Reference-level explanation
[reference]: #reference

A delegation item can only appear inside a trait impl block. Delegation inside inherent impls is left as a future extension.

Delegation must be to a field on `Self`. Other kinds of implementer expressions are left as future extensions. This also means delegation can only be done on structs for now.

Only `fn`s can be delegated. Delegation of `type` and `const` items are left as future extensions.

For a method to be delegated, the receiver must be `self`, `&self` or `&mut self`. The receiver `Box<Self>` and [Custom Self Types](https://github.com/rust-lang/rfcs/pull/2362) are not supported for delegation at this time, nor are other parameters/return types containing `Self`.

A delegation item always consists of:

-   the keyword `delegate`
-   either a `*`, or a comma-separated list of items being delegated
-   the contextual keyword `to`
-   the delegation target `self.field_name`
-   a semicolon

An “item being delegated” is always two tokens. The first token must be `fn`. The second is any valid identifier for a trait item.

The semantics of a delegation item should be the same as if the programmer had written each delegated item implementation manually. For instance, if the trait `TR` has a default implementation for method `foo()`, and the type `F` does not provide its own implementation, then delegating `TR` to `F` means using `TR`’s implementation of `foo()`. If `F` does provide its own implementation, then delegating `TR` to `F` means using `F`’s implementation of `foo()`. The only additional power granted by this feature is that `delegate *` can automatically change what items get implemented if the underlying trait `TR` and type `F` get changed accordingly. There can be at most one `delegate *` per `impl` block.

To generate the wrapper function:

-   The function signature is copied from the function being delegated to.
-   The self parameter is mapped to the implementer expression `self.field_name`.
-   `.trait_method_name()` is appended to the implementer expression.
-   Subsequent parameters are passed through, e.g.
    ```rust
    fn check_name(&self, name: &str, ignore_capitals: bool, state: &mut State) -> bool {
        self.f.check_name({name}, {ignore_capitals}, {state})
    }    
    ```

It is a compile-time error to `delegate` a trait to a struct field that doesn't implement the trait.


# Possible Future Extensions
[future_extensions]: #future_extensions

There are a _lot_ of possibilities here. We probably don’t want to do most of these, as this is supposed to be a pure sugar feature targeting the most common cases where writing out impls is overly tedious, not every conceivable use case where “delegation” might apply. However, the authors believe it likely that a few of these extensions will happen and the proposed syntax is intended to make it as easy as possible to add any of these.

Attempting to delegate an item requiring a possible future extension results in a compile-time error, e.g. `Delegating ... is not supported at this time. For more information, see RFC #2393.`


## Associated Types and Constants
[other_trait_items]: #other_trait_items

We expect to support delegation of trait `type` and `const` items. They are not proposed at this time in interest of being conservative.

```rust
impl TR for S {
    delegate fn foo, fn bar, type Item, const MAX
        to self.f;
}
```

Expands to:

```rust
impl TR for S {
    type Item = <F as TR>::Item;
    const MAX = <F as TR>::MAX;

    fn foo(&self) -> u32 {
        self.f.foo()
    }
    fn bar(&self, x: u32, y: u32, z: u32) -> u32 {
        self.f.bar(x, y, z)
    }
}
```


## Custom `Self` Types and `Box<Self>`
[custom_self_types]: #custom_self_types

We expect to support delegation for any receiver type `T` where `T: Deref<Target = Self>`, as per the [Custom Self Types RFC](https://github.com/rust-lang/rfcs/pull/2362).

```rust
impl TR for S {
    fn foo(self: Box<Self>) -> u32 {
        // ...
    }
}
```

When an implementation has been completed, this RFC will be amended.


## Getter Methods
[getter_methods]: #getter_methods

The most commonly requested extension is delegating to getter methods instead of fields, which would also allow delegation for other types: enum, array, etc. 

```rust
impl Read for Wrapper { delegate * to self.get_read(); }
impl Write for Wrapper { delegate * to self.get_write(); }
```


## Inherent Impls
[inherent_impls]: #inherent_impls

While we imagine delegating traits will often be best practice, it's also valuable to do so with inherent `impl`s. This makes the language more consistent and preempts the questions: "Why can't I do this directly on my type? Why do I have to define a trait?"

```rust
struct AppendOnlyVec<T> (Vec<T>);

impl<T> AppendOnlyVec<T> {
    delegate fn push to self.0;
    // other meaningful methods
}
```

This is an example of a "restricted type" without using a trait. We can easily delegate the methods we want to the inner `Vec`, thereby restricting access to functionality we don't want to expose.


## Inherent Traits
[inherent_traits]: #inherent_traits

When a trait is implemented for a type, the user of the type must also `use` the trait in order to use its methods. We can avoid this by delegating to the trait impl.

```rust
impl S {
    delegate * to trait TraitOne;
    delegate fn just_one_method to trait TraitTwo;
}
```

Delegating individual methods may not be necessary, but is included for consistency. Delegating other trait items does not make sense and would result in a compile-time error; stabilization is blocked on the quality of this error message.

This extension requires `rustdoc` support to avoid the confusion of duplicated methods, perhaps a category “Inherent Trait Implementations.” Stabilization is also blocked on this.

There is a concern about _inherent traits_ causing duplicated symbols, which should be resolved during implementation.

While this is listed as a possible future extension as we do not want to block acceptance of this RFC on acceptance of inherent traits, there is a [current RFC](https://github.com/rust-lang/rfcs/pull/2375), [previous RFC](https://github.com/rust-lang/rfcs/pull/2309) and [prior](https://github.com/rust-lang/rfcs/issues/1880) [discussions](https://github.com/rust-lang/rfcs/issues/1971) indicating this is a strongly desired feature and should not be overlooked.


## Delegating a Trait to Multiple Fields
[multiple_fields]: #multiple_fields

If you want to delegate some methods to one field and some to another, simply write multiple `delegate` items:

```rust
impl TR for S {
    delegate * to self.field_one;
    delegate fn foo, const MAX, type Item
        to self.field_two;
}
```

This delegates `foo`, `MAX`, and `Item` to `field_two`. Everything else is delegated to `field_one`.


## Delegate Block
[delegate_block]: #delegate_block

In a delegation item, the `to` contextual keyword and field (delegation target) are replaced with a delegate block. The block maps Self parameters to implementer expressions and return values to output types.

e.g. Delegating to an inner type using getter methods instead of fields:

```rust
delegate fn foo, fn bar {
    |&self| self.get_inner(),
    |&mut self| self.get_inner_mut(),
    |self| self.into_inner(),
    |x: Rc<Self>| self.rc_into_inner_rc(),
} -> {
    |delegate| Self::from_inner(delegate),
    |x: Rc<Delegate>| Self::rc_from_inner_rc(x)
}
```

In addition to `delegate`, this extension requires `Delegate` to be a keyword in edition 2018 to avoid parser complexity, although it is not necessarily expression context, so we could potentially make it contextual. If this extension is not ruled out during the RFC process, `Delegate` should also be reserved in edition 2018. Alternatively, this could be written as `x: Rc<delegate>`, breaking tradition with capitalized `Self`.


A `delegate` block could potentially be used to implement some [other extensions](#other-extensions):
-   Delegating to static values or functions.
-   Delegating to arbitrary expressions.
-   Delegating a trait impl to an inherent impl.


### Delegating an enum

> [name=elahn] This is an exploration, starting with an assumption that a delegate block is required to use getter methods.

```rust
use std::io::{Read, Write};

struct Foo { ... } // : Read + Write
struct Bar { ... } // : Read + Write

enum Wrapper { Foo(Foo), Bar(Bar) }

impl Wrapper {
    fn get_read(&mut self) -> &mut Read {
        match *self {
            Wrapper::Foo(ref mut foo) => foo,
            Wrapper::Bar(ref mut bar) => bar,
        }
    }
    fn get_write(&mut self) -> &mut Write {
        match *self {
            Wrapper::Foo(ref mut foo) => foo,
            Wrapper::Bar(ref mut bar) => bar,
        }
    }
}
impl Read for Wrapper {
    delegate * {
        |&mut self| self.get_read(),
    }
}
impl Write for Wrapper {
    delegate * {
        |&mut self| self.get_write(),
    }
}
```

Expands to:

```rust
impl Read for Wrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.get_read().read(&mut buf)
    }
}
impl Write for Wrapper {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.get_write().write(&mut buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.get_write().flush()
    }
}
```

> [name=elahn] After writing this out with a delegate block, I'm not convinced it's a good idea to disallow delegating to a trait method without one, e.g 
> ```rust
> impl Read for Wrapper { delegate * to self.get_read(); }
> impl Write for Wrapper { delegate * to self.get_write(); }
> ```
> 
> It makes the syntax a lot noisier for traits where we delegate * or a single method, which are fairly common. AFAICT, the implementer experession is always `self.method_name()`, regardless of the type of Self, except for:
> -   Delegating to an inner type using getter methods instead of fields.
> -   Delegating to static values or functions.
> -   Delegating to arbitrary expressions.
>
> This makes me think deleagate blocks should be an advanced feature and we should only use them if we decide we want to enable those use cases.

If "delegating for an enum where every variant's data type implements the same trait" is the common case, we could create sugar for it. e.g.

```rust
impl Read for Wrapper { delegate * to enum &mut Read; }
impl Write for Wrapper { delegate * to enum &mut Write; }
```
Expands to:
```rust
impl Wrapper {
    fn get_read(&mut self) -> &mut Read {
        match *self {
            Wrapper::Foo(ref mut foo) => foo,
            Wrapper::Bar(ref mut bar) => bar,
        }
    }
    fn get_write(&mut self) -> &mut Write {
        match *self {
            Wrapper::Foo(ref mut foo) => foo,
            Wrapper::Bar(ref mut bar) => bar,
        }
    }
}
impl Read for Wrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.get_read().read(&mut buf)
    }
}
impl Write for Wrapper {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.get_write().write(&mut buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.get_write().flush()
    }
}

```
Alternatively, we could match in each function, skipping the indirection through a trait reference.
```rust
impl Read for Wrapper { delegate * to enum; }
impl Write for Wrapper { delegate * to enum; }
```
Expands to:
```rust
impl Read for Wrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match *self {
            Wrapper::Foo(ref mut foo) => foo.read(&mut buf),
            Wrapper::Bar(ref mut bar) => bar.read(&mut buf),
        }
    }
}
impl Write for Wrapper {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match *self {
            Wrapper::Foo(ref mut foo) => foo.write(&buf),
            Wrapper::Bar(ref mut bar) => bar.write(&buf),
        }
    }
    fn flush(&mut self) -> Result<()> {
        match *self {
            Wrapper::Foo(ref mut foo) => foo.flush(),
            Wrapper::Bar(ref mut bar) => bar.flush(),
        }
    }
}
```


## `unimplemented!()`
[unimplemented]: #unimplemented

This particular expression could be allowed as a special case as opposed to allowing arbitrary expressions.

```rust
impl TR for S {
    delegate const MAX, type Item
        to self.f;
    delegate _ to unimplemented!();

    fn foo(&self) -> u32 {
        42
    }
}
```

Unspecified `fn`s are stubbed out with `unimplemented!()` to allow rapid prototyping. This would give most of the benefits of `#[unfinished]` in [RFC #2205](https://github.com/rust-lang/rfcs/pull/2205) without introducing a new attribute.

```rust
    fn bar(&self, x: u32, y: u32, z: u32) -> u32 {
        unimplemented!()
    }
```


## Other Extensions
[other_extensions]: #other_extensions

-   Delegating to static values or free functions.
-   Delegating to arbitrary expressions.
-   Delegating a trait impl to an inherent impl.
-   Delegating a method `foo()` to a differently-named method `bar()` that happens to have the same signature.
-   Delegating “multiple Self arguments” for traits like PartialOrd, so that `delegate * to self.f;` would desugar to something like `self.f.partial_cmp(other.f)`
-   Delegating for an enum where every variant's data type implements the same trait.
-   Delegating trait fields, once that feature is implemented.
-   Delegating multiple traits in a single item, e.g.
    ```rust
    impl PartialEq + PartialOrd + Ord for PackageId {
        delegate * to self.f;
    }
    ```


# Drawbacks
[drawbacks]: #drawbacks

-   A new keyword `delegate` in edition 2018, in accordance with the lang team [keyword policy](https://paper.dropbox.com/doc/Keyword-policy-SmIMziXBzoQOEQmRgjJPm) that new features should be real keywords for maintenance reasons. Here is a quick review of the breakage risk:
    -   TL;DR: The risk is quite minimal and something we could probably live with.
    -   Usage as ident in libstd: No
    -   Usage as the name of a crate: No
    -   Usage as idents in crates ([sourcegraph](https://sourcegraph.com/search?q=repogroup:crates+case:yes++%5Cb%28%28let%7Cconst%7Ctype%7C%29%5Cs%2Bdelegate%5Cs%2B%3D%7C%28fn%7Cimpl%7Cmod%7Cstruct%7Cenum%7Cunion%7Ctrait%29%5Cs%2Bdelegate%29%5Cb+max:400)): 19+ uses
-   This is a new way of writing trait implementations and we already have two ways, including `#[derive(..)]`.
-   If too many of the future extensions are implemented, this could become an overly complex feature.
-   The `delegate *` syntax may be too implicit:
    -   When a function is delegated implicitly, it is harder for a reader to find the actual definition, especially if multiple struct members providing different traits are delegated to using the `*` syntax.
    -   The list of functions delegated-to by `*` depend on the type of the *trait* being delegated, not on the functions provided by the delegated-to object. This is potentially confusing.

> [color=#d41ced] TODO: Summarize this potential drawback and clarify how `delegate` is used in Objective C / Swift.
> 
> @zackw: Is the word “delegate” really appropriate in this context? I associate it with a complicated and confusing feature of C# that seems like it does a lot more than this (to be fair, I have never actually learned C#). This proposal is just syntactic sugar for wrapper functions.
>
> @Ixrec: To me, C# delegates are a really weird name for what I normally call “an event handler” or “the observer pattern” in other languages like Javascript and C++ where it’s not a core language feature. As far as I know, “delegate” is not used that way by any other language, so I’m not personally worried about confusion. I’m also not aware of any other good names for this feature, though that may just be because there hasn’t been much brainstorming for it.
>
> @steven099: It’s used extensively in Objective C / Swift. The thing is that the _delegate_ (n. /ˈdɛlɪɡət/) is the value/type to which you _delegate_ (v. /ˈdɛlɪˌɡeɪt/) functionality. C# has /ˈdɛlɪɡət/s, while I believe this proposal is about /ˈdɛlɪˌɡeɪt/ing.


# Rationale and alternatives
[alternatives]: #alternatives

The biggest non-syntax alternative is supporting delegation of all items: methods, associated types and `const`s. While this has been removed in the interest of being conservative, the authors would prefer to support all “trait items” because the whole point is to “make trivial wrapper impls trivial,” even if you’re implementing a trait like `Iterator` which has an associated type as well as several methods.

### Alternative syntax

```rust
impl TR for S {
    delegate to self.f for *;
}

impl TR for S {
    delegate to self.f for fn foo, fn bar, const MAX, type Item;
}
```

The transition to _delegate block_ syntax isn't quite as nice, but it still works:

```rust
delegate fn foo, fn bar {
    |&self| self.get_inner(),
    |&mut self| self.get_inner_mut(),
    |self| self.into_inner(),
    |x: Rc<Self>| self.rc_into_inner_rc(),
} -> {
    |delegate| Self::from_inner(delegate),
    |x: Rc<Delegate>| Self::rc_from_inner_rc(x)
}
```

The argument for this syntax is "when trait items are explicitly listed rather than globbed, the line quickly becomes long and difficult to read." However, this is solved by the `rustfmt` default of moving `to field_name;` to a new line when delegating individual trait items, e.g.

```rust
impl TR for S {
    delegate fn foo, fn bar, fn baz, const MAX, type Item, type Output
        to self.really_long_field_name;
}
```

### Omitting `self` in delegation items

```rust
impl TR for S {
    delegate * to f;
}

impl TR for S {
    delegate to f for *;
}
```

While this syntax is less verbose, it is less obvious the target is a struct field, especially for tuple structs.

`self.` is unnecessary and could lead newcomers to believe they can write arbitrary Rust code there. However, this risk also applies to allowing getter methods in the basic delegation syntax.


### Omitting the `impl` block

The `impl` block is a lot of extra noise when simply delegating all trait items to a struct field. There can be a lot of generic type parameters which are copy-pasted from the member field's trait impl.

```rust
delegate TR::* to S::f;
delegate TR::* to S::0;
delegate TR::{fn foo, fn bar}
    to S::f;
delegate TR::* to S::getter();
delegate fn push to AppendOnlyVec::0;
delegate S::* to trait TR;

// The common case is delegating entire trait(s):
delegate TR to S::f;
delegate TR1, TR2 to S::f;
```

This is shorter and easier to read than the equivalent:

```rust
impl TR for S { delegate * to self.f; }
impl TR for S { delegate * to self.0; }
impl TR for S {
    delegate fn foo, fn bar
        to self.f;
}
impl TR for S { delegate * to self.getter(); }
impl<T> AppendOnlyVec<T> { delegate fn push to self.0; }
impl S { delegate * to trait TR; }

// The common case is delegating entire trait(s):
impl TR for S { delegate * to self.f; }
impl TR1 + TR2 for S { delegate * to self.f; }
```

However, in the case of delegating most of a trait's methods, an `impl` block is still required and now the "overridden" method can be seperated from the delegate item, which could be confusing and easier to miss what is happening.

We could offer both syntaxes as a good 4 step ratchet:

1.  first try `#[derive(..)]`
2.  then go with `delegate TR::* to S::f;`
3.  then delegate inside an impl
4.  finally implement things manually.


### Assume items are `fn`

When listing items to delegate, the prefix `fn` is assumed, since it is most common. Items may be prefixed with `fn` if desired.

```rust
impl TR for S {
    delegate foo, bar, const MAX, type Item
        to self.f;
}
impl TR for S {
    delegate to self.f for foo, bar, const MAX, type Item;
}
delegate TR::{foo, bar, const MAX, type Item}
    to S::f;
delegate push to AppendOnlyVec::0;
```

This would make `fn`s ambiguous with traits if we decide to allow shorter forms for the common case of delegating entire trait(s):

```rust
delegate TR to S::f;
delegate TR1, TR2 to S::f;
```


### Different syntax for delegating most trait items

When refactoring from "delegate all" (`delegate * to f;`) to "delegate most" trait items, `*` is replaced with `_`.

```rust
impl TR for S {
    delegate _ to self.f;

    fn foo(&self) -> u32 {
        42
    }
}
```

This overcomes the drawback: the `delegate *` syntax may be too implicit.

It also overcomes the objection to omitting the `impl` block: the "overridden" method can be seperated from the delegate item, which could be confusing and easier to miss what is happening.

The increase in cognitive load is minimal, since `_` is widely used to mean "inferred by the compiler."


### Other syntax options the authors chose not to use in this RFC:

Many of these syntaxes were never “rejected” in the original RFC’s comment thread and are likely still on the table. This list merely describes the authors' rationale for preferring `delegate ... to field_name;` over all of these alternatives.

-   `impl TR for S use self.F { ... }` was criticized in the first RFC’s comment thread for looking too much like inheritance.
-   `impl TR for S { use self.F; ... }` was criticized in the first RFC’s comment thread for ruling out the possibility of `use` declarations inside impl blocks.
-   `impl TR for S => self.F;` and `impl TR for S => self.F { ... }` This is good for delegating an entire trait impl, but when used for partial delegation where the remaining implementations are inside the curly braces, this starts looking like inheritance again, appears to put implementation details in the signature where they don’t belong, and I believe would be relatively easy to overlook compared to most of the other syntaxes.
-   `fn method = self.field.method;` This syntax was suggested for delegating a single item. It’s not clear how to extend it to delegating multiple items, “most” items or all items in an impl.
-   Various attribute syntaxes like `#[delegate(foo=S)]`. Most of these made it hard to tell what was the item being delegated and what was the field being delegated to. This also seems like it would lead to “stringly typed” attribute syntax like `#[delegate(foo="self.f.foo()")]` if we tried to make it cover most of the possible future extensions. Also, attributes for an impl block would normally go outside the impl block, but since delegation is purely an implementation detail it again seems strange to put it outside the block. Finally, an attribute would be appropriate if this feature could be implemented as a proc macro someday, but delegation cannot because it requires “looking outside” the impl block to see all the items in the trait being implemented.


### What is the impact of not doing this?

Without a mechanism for efficient code reuse, Rust will continue to be criticised as "verbose" and "requiring a lot of boilerplate." Delegation isn't a perfect vaccine for that criticism, but goes a long way by making code reuse as easy in Rust as in common OOP and functional languages.


# Unresolved Questions
[unresolved_questions]: #unresolved_questions

We expect to resolve through the RFC process before this gets merged:

-   For the possible future extension _delegate block_, should we reserve the keyword `Delegate` in edition 2018?
-   Should `to` be a keyword in edition 2018?
-   Are there any possible extensions the proposed syntax is not forward compatible with?

We expect to resolve through the implementation of this feature before stabilization:

-   The final syntax for delegation items.
-   How does delegation interact with specialization? There will be a [default impl](https://github.com/rust-lang/rfcs/blob/master/text/1210-impl-specialization.md#default-impls) block in the future. Should we allow `delegate` to be used in a `default impl` block?
    - The authors of this RFC do not have a specific reason to disallow this. The question was raised during discussion and we don't know enough about specialization to answer it.

Out of scope for this RFC:

-   For readability, `rustfmt` could move delegation items to the top of an impl block. This is left to a future [style-fmt RFC](https://github.com/rust-lang-nursery/fmt-rfcs).
