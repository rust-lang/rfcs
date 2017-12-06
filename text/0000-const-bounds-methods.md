- Feature Name: const_bounds_methods
- Start Date: 2017-12-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allows for:

1. `const fn` in `trait`s.

2. over-constraining trait `fn`s as `const fn` in `impl`s.

3. syntactic sugar `const impl` for `impl`s where all `fn`s are const.

4. `const` bounds as in `T: const Trait` satisfied by `T`s with only
`const fn`s in their `impl Trait for T {..}`.

# Motivation
[motivation]: #motivation

This RFC adds a non-trivial amount of expressive power to the language.

Let us go throw the motivation for each bit in order.

## 1. `const fn` in traits

Currently, associated constants are the only part of the constant time
evaluation that is available for `trait`s.

But there are useful `const fn`s besides those which associated constants model
(those from `() -> T` - i.e: a value not depending on inputs) where the
`const fn`s depend on inputs, be they `const` or other.

It is also inconsistent not to have `const fn`s in `trait`s as `fn` and
`unsafe fn` are both allowed today.

## 2. over-constraining `trait` `fn`s as `const fn` in `impl`s

This allows the user to be more strict and less allowing than the `trait`
permits. The expressive power gained here is a) that the user may statically
check that the `fn` may not do certain things, b) that when all `fn`s are
marked as `const fn` in the `impl`, the user may use the `impl` as the target
of a `const` trait bound which is discussed below.

## 3. syntactic sugar `const impl`

Prefixing an `impl` with `const` as in `const impl` is sugar for prefixing all
`fn`s in the `impl`, be it a trait `impl` or an inherent `impl`. As this is
sugar, it adds no additional expressive power to the language, but makes the
use of 2. and existing `const fn` use in inherent `impl`s more ergonomic.

It also aids searchability by allowing the reader to know directly from the
header that a trait impl is usable as a const trait bound, as opposed to
checking every `fn` for a `const` modifier.

By doubling as sugar usable for inherent `impl`s, the introduced syntax carries
its own weight. It allows the user to separate `const fn`s and normal `fn`s in
the documentation of inherent `impl`s.

## 4. `const` trait bounds, `T: const Trait`

Such a bound `T: const Trait` denotes that `impl Trait for T` must be a
constant trait impl (with only `const fn`s) as suggested in 2-3. In a
`const fn foo<T: const Trait>(..)`, this fact may be used to call methods
from `Trait` in `foo`.

It may also be used for `const` and `static` bindings or as input for const 
generics inside a normal `fn foo<T: const Trait>(..)` declaration. And in such
a context, the user can be certain that no I/O may happen inside the called
`const fn`s. When the methods of `Trait` are called with input that is `const`,
the user may also be certain that the call is cheap at runtime.

The new form of bound also allows reuse of traits, an important step to avoid
a bifurcation of existing traits along the lines of `const fn` vs. `fn`, both
in the standard library and elsewhere. A canonical example that const trait
bounds would solve is not having both `Default` and `ConstDefault`. Doing that
is important because it considerably reduces the amount of duplication of
`impl`s for such traits.

A consequence of const trait bounds is at least that `F: const FnOnce` is now
possible, allowing the user to effectively expect a `const fn` closure.

# Vocabulary
[vocabulary]: #vocabulary

Let's introduce the new terms used in this RFC.

+ **const impl syntax**, refers to the specific syntax `const impl`.
+ **constant trait impl**, refers to `impl`s of `trait`s where all `fn`s are
marked as `const fn`.
+ **const trait bound**, refers to a trait bound of the form `T: const Trait`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We will now go through all the points discussed in the summary and explain
what they entail.

## 1. `const fn` in traits

Simply put, the following is now allowed:

```rust
trait Foo {
    const fn bar(n: usize) -> usize;
}
```

In other words, `trait`s may now require of their `impl`s that a certain `fn`
be a `const fn`. Naturally, `bar` will now be type-checked as a `const fn`.

This is of course not specific to this trait or one method. Any trait,
including those with type parameters can now have `const fn`s in them,
and these `const fn`s may have any number of type parameters, parameters,
and any return type.

## 2. over-constraining `trait` `fn`s as `const fn` in `impl`s

Consider the `Default` trait:

```rust
pub trait Default {
    fn default() -> Self;
}
```

And and some type - for instance:

```rust
struct Foo(usize);
```

As a rustacean, you may now write:

```rust
impl Default for Foo {
    const fn default() -> Self {
        Foo(0)
    }
}
```

Note that this `impl` has constrained `default` more than required by the
`Default` trait. Why this is useful will be made clear in the [motivation]
and in the [guide-level-explanation] of `const` trait bounds.

Naturally, `default` for `Foo` will now be type-checked as a `const fn`.

## 3. syntactic sugar `const impl`

In any inherent `impl` (not an impl of a trait), or an `impl` of a trait,
you may now write:

```rust
const impl MyType {
    fn foo() -> usize;

    fn bar(x: usize, y: usize) -> usize;

    // ..
}

const impl MyTrait for MyType {
    fn baz() -> usize;

    fn quux(x: usize, y: usize) -> usize;

    // ..
}
```

and have the compiler desugar this for you into:

```rust
impl MyType {
    const fn foo() -> usize;

    const fn bar(x: usize, y: usize) -> usize;

    // ..
}

impl MyTrait for MyType {
    const fn baz() -> usize;

    const fn quux(x: usize, y: usize) -> usize;

    // ..
}
```

For the latter case of `const impl MyTrait for MyType`, this always means that
`MyType` may be used to substitute for `T` in a bound like `T: const MyTrait`.

The compiler will of course now check that the `fn`s are const, and refuse to
compile your program if you lied to the compiler.

When it comes to migrating existing code to this new model, it is recommended
that you simply start by adding `const` right before your `impl` and see if it
still compiles and then continue this process until all `impl`s that can be
`const impl` are. For those that can't, you can still add `const fn` to some
`fn`s in the `impl`. The standard library will certainely follow this process
in trying to make the standard library as (re)useable as possible.

## 4. `const` trait bounds, `T: const Trait`

Speaking of const trait bounds, what are they? They are simply a bound-modifier
on a bound `T: Trait` denoted as `T: const Trait` with some changed semantics.

What are the semantics? That any type you substitute for `T` must in addition
to impl the trait in question, also do so without any normal `fn`s. Any `fn`s
occuring in the `impl` must be marked as `const fn`. These `impl`s are exactly
those `impl`s that are currently, or would type check as `const impl`.

A `const Trait` bound gives you the power to use all `fn`s in the trait in a
`const` context such as in const generics, `const` bindings and in `const fn`s
in general.

Currently, this RFC also proposes that you be allowed to write `impl const Trait`
and `impl const TraitA + const TraitB`, both for static existential and universal
quantification (return and argument positiion). However, the RFC does not, in
its current form, mandate the addition of syntax like `Box<const Trait>`.

If you try to use a type `MyType` that does not fulfill the `const`ness
requirement of `T: const MyTrait`, then the compiler will greet you with
an error message and refuse to compile your program.

Let us now see some, albeit somewhat contrived, examples of `const Trait` bounds.

### Static-dispatch existential quantification

```rust
fn foo() -> impl const Default + const From<()> {
    struct X;
    const impl From<()> for X { fn from(_: ()) -> Self { X } }
    const impl Default for X  { fn default() -> Self { X } }
    X
}
```

or with alternative and optional syntax:

```rust
fn foo() -> impl (const Default) + (const From<()>) {
    struct X;
    const impl From<()> for X { fn from(_: ()) -> Self { X } }
    const impl Default for X  { fn default() -> Self { X } }
    X
}
```

### Static-dispatch universal quantification

```rust
const fn foo(universal: impl const Into<usize>) -> usize {
    universal.into()
}
```

### In a free `fn`

```rust
fn foo<T: const Default + const Add>(x: T) -> T {
    T::default() + x
}
```

### In bounds on type variables of an `impl` and `trait`

```rust
trait Foo<X: const From<Self>>: Sized {
    const fn bar(x: X) -> Self {
        x.into()
    }
}

impl<F: const FnOnce(Self) -> Self> Twice<F> {
    const fn twice(self, fun: F) -> Self {
        fun(fun(self))
    }
}
```

We could enumerate a lot more examples - but instead, what you should really
understand is that anywhere you may write `T: SomeTrait`, you may also write:
`T: const SomeTrait`.

# How do we teach this?

It should be noted that the concept of a const trait bound is an advanced one.
As such, it will and should not be one of the early topics that an aspiring
rustacean will study. These topics should be taught in conjunction with
and gradually after teaching about free `const fn`s and their inherent siblings.
However, it should be noted that a user that only knows of `fn` and has never
heard of `const fn` may still happily and obliviously use a `const impl` or
overconstrained `const fn`s in impls just as they can with free `const fn`s
today.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC entails:

1. `const fn` in `trait`s.

2. over-constraining trait `fn`s as `const fn` in `impl`s.

3. syntactic sugar `const impl` for `impl`s where all `fn`s are const.

4. `const` bounds as in `T: const Trait` satisfied by `T`s with only
`const fn`s in their `impl Trait for T {..}`.

Let us unpack each of this one by one and go through their mechanics and
cooperation.

## 1. `const fn`s in `trait`s

The syntactic rules of `trait` (now) allow `fn`s to be optionally prefixed
with `const`.

Semantically, a `const fn` inside a `trait MyTrait` requires that any `impl`
for some type of the trait include that `fn`, and that it must be `const fn`.
Such trait-mandated `const fn`s are obviously type-checked as `const fn` also.
Unlike free `const fn`s, those in traits may refer to `self`, `Self`,
associated types, and constants, syntactically. Type checking (now) takes this
into account.

A simple example of `const fn`s in a trait is:

```rust
pub trait Foo {
    const fn bar(x: usize, y: usize) -> Self;

    const fn baz(self) -> usize;
}
```

## 2. over-constraining trait `fn`s as `const fn` in `impl`s

This entails that any trait `impl` may constrain an `fn` in the trait as
`const fn`. This means that the `impl` is voluntarily opting in to a
being more restrictions on the `fn`s than the trait required. Other than
knowing that certain things which `const fn`s forbid are now not used for
that `fn`, there are other benefits to opting in. The `impl` may opt-in
for as many trait `fn`s as it likes - zero, one, .. or even all.
Those over-constrained `fn`s are now type-checked as `const fn`s.

## 3. syntactic sugar `const impl`

Rust (now) allows the user to prefix `impl` with `const` as in this example:

```rust
const impl Foo {
    fn bar() -> usize;

    fn baz(x: usize, y: usize) -> usize;

    // ..
}

const impl Wibble for Wobble {
    fn wubble() -> usize;

    fn quux(x: usize, y: usize) -> usize;

    // ..
}
```

The compiler will desugar the above `impl`s to:

```rust
impl Foo {
    const fn bar() -> usize;

    const fn baz(x: usize, y: usize) -> usize;

    // ..
}

impl Wibble for Wobble {
    const fn wubble() -> usize;

    const fn quux(x: usize, y: usize) -> usize;

    // ..
}
```

## 4. `const` trait bounds, `T: const Trait`

Introduced syntax: in addition to `$ident: $ident` allowed in `where` clauses
and where type variables are introduced as in for example `impl< $here >` you
may (now) write `$ident: const $ident`. This is called a const trait bound.
An example of such a bound is `F: const FnOnce(X) -> Y` as well as
`D: const Default`.

Semantically, having such a bound means that when a type `MyType` replaces a
type variable with that bound, it may only do so iff there exists an `impl` of
the trait for the type that is also a constant `impl`.

What is a constant impl? For an `impl` to be considered constant, the only `fn`s
it may have are `const fn`s. This coincides with the `const impl` syntax,
in other words: `const impl` syntax introduces a constant impl. The user may
however manually prefix `const` before every method and have a valid constant
impl still.

Since a `T: const Trait` bound entails that any `<T as Trait>::method` be a
`const fn`. This further entails that `method` may be used within a `const fn`,
and other `const` contexts such as const-generics, defining the value of
an associated consttant, etc.

### Type checking

We give a high level description of an algorithm for type checking this idea.

During registration and collection of `impl`s, a check is done whether all `fn`s 
are prefixed with `const`. This is a purely syntactic check. A flag `is_const`
is then stored on/associated-with the `impl`. Checking that bodies of the
methods actually follow the rules of `const fn` can now be done separately.

During type checking of a `const fn`, iff a bound `T: const Trait` exists,
then the type checking allows the use of `T::trait_method()` inside. This also
applies to bounds on `impl` which allows associated constants to use such
functions in their definition site. For normal `fn`s with a const trait bound,
any `const` bindings inside the `fn` are now allowed to use `T::trait_method()`.

During unification/substitution of type variables for specific/concrete types,
a lookup is first done to see if the impl exists. So far so normal. If a const
trait bound exists, then the `is_const` (which is by default false) flag is
checked for `true`. If it is `true`, then the type is substituted. Otherwise,
a typeck error is raised.

## 4.1 Static-dispatch existential types (`impl Trait`)

Introduced syntax: In addition to normal (static-dispatch) existential type
syntax `-> impl Trait` or `-> impl TraitA + TraitB + ..` you may (now) also
write `-> impl const Trait` or `-> impl const TraitA + traitB + const TraitC`.
To aid reading, grouping with parenthesis is possible as:
`-> impl (const Trait)` or `-> impl (const TraitA) + traitB + (const TraitC)`.

As expected, `-> impl const Trait` entails that the returned type now must,
in addition to providing an `impl` of the `Trait` for the returned anonymous
type now also do so by having the impl be constant trait impl as done in
the following example:

```rust
fn foo() -> impl const Default + const From<()> {
    struct X;
    const impl From<()> for X { fn from(_: ()) -> Self { X } }
    const impl Default for X  { fn default() -> Self { X } }
    X
}
```

### Type checking

A caller of `foo()` may use the result where a universally quantified bound
`T: const Default + const From<()>` exists and use the methods of `Default`
and `From<()>` in a `const` context: `const fn`, associated consts,
const bindings, and array length size.

## 4.2 Static-dispatch universal quantification

Introduced syntax: In addition to normal anonymous (static-dispatch) universally
quantified type syntax `argument: impl Trait`, you may (now) also write:
`argument: -> impl const Trait` as in the following example:

```rust
const fn foo(universal: impl const Into<usize>) -> usize {
    universal.into()
}
```

### Type checking

In the above example, the behaviour is identical to:

```rust
const fn foo<T: const Into<usize>>(universal: T) -> usize {
    universal.into()
}
```

and treated as such for type-checking purposes.
The only difference is that the type `T` is anonymous and may now
not be reused other than by type alias.

## 4.3 `impl trait` type alias

Introduced syntax: As you may write `type Foo = impl Trait;` you may (now) also write: `type Foo = impl const Trait`.

# Drawbacks
[drawbacks]: #drawbacks

- This is quite a large addition to the language both semantically and
syntactically. Some users may wish for a smaller language with fewer features.

- The syntax `T: const Trait` may be confused with `const T: usize`.

- The usefulness of `const impl` can be called into question. However it
may pull its own weight especially during migration.

- This will lead to increased compile times, but due to semantic compression
(less code for more intent), compile times can also increase less than it would
with bifurcation of the trait system. Const trait bounds are cheap to check.

# Rationale and alternatives
[alternatives]: #alternatives

The impact of not using the ideas at the core of the RFC is loss of
expressive power.

With regards to const trait bounds, we trait system could simply be bifurcated.
Have one trait for the const version, and one for the normal. This does however
not scale. It is much better to have a modifier on bounds than many more traits.
The RFC argues therefore that this design is better compared to introducing
traits such as:

```rust
pub trait ConstDefault: Default { const DEFAULT: Self; }
```

If part 1. of this RFC is not merged, only `fn`s of the form `() -> RegularType`
may even be used, so full bifurcation would not even be possible then.

The natural companions to const trait bounds are constant impls, without them,
that part of the proposal wouldn't be nearly as expressive.

We can ask why const trait bounds impose an all-or-nothing proposition and why
the user is not just obliged to satisfy constness of those particular `fn`s that
the user of the bound uses. This is due to the fragility of such a system.
The const trait bound is morally right to at any time use more `fn`s from the
repertoire of `fn`s at their disposal. But if all `fn`s were not required to
be `const` and the call site provided a type which only satisfied constness for
one `fn`, then the type-checker will all of a sudden refuse to give its go-ahead
and as a result, the program refuses to compile and you have a backwards compatibility breakage.

We could of course solve this by requiring the bound to specify exactly what
`fn`s are required to be `const`. In practice, this would be tedious to write
since there may be more than one `fn`. It is therefore a recipe for bad
ergonomics and developer experience.

# Unresolved questions
[unresolved]: #unresolved-questions

- We must be reasonably confident of the soundness of this proposal.
Some edge cases may still be resolvable prior to stabilization.

- We should decide on the interaction of const trait bounds with trait objects.
Should the user be able to say for example `Box<const Foo>`? While very little
expressive power is gained through this (you get to restrict what `&(const Foo)` 
may do by only allowing calls to `const fn`s), `&'static (const Foo)` may be
more useful - and then, for the sake of not special casing, `Box<const Foo>`
should be allowed.

- Should we consider the syntax `const trait Foo { .. }`? The current thinking
is that it would not carry its weight. It is much more common to define `impl`s
than `trait`s!

- Does `T: const Trait` specialize `T: Trait`? We can always initially answer
no to this and then change during stabilization as power is strictly gained.

# Acknowledgements
[acknowledgements]: #acknowledgements

This RFC was significantly improved by exhaustive and deep discussions with
fellow rustaceans Ixrec, Alexander "durka" Burka, rkruppe, and Eduard-Mihai
"eddyb" Burtescu. I would like thank you for being excellent and considerate
people.