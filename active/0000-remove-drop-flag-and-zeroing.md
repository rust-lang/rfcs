- Start Date: 2014-08-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Three step plan:

 1. Revise language semantics for drop so that all branches move or drop
    the same pieces of state ("drop obligations").

 2. Add lint(s) to inform the programmer of situations when this new
    drop-semantics could cause side-effects of RAII-style code
    (e.g. releasing locks, flushing buffers) to occur sooner than
    expected.

 3. Remove the dynamic tracking of whether a value has been dropped or
    not; in particular, (a) remove implicit addition of a drop-flag by
    `Drop` impl, and (b) remove implicit zeroing of the memory that
    occurs when values are dropped.

# Motivation

Currently, implementing `Drop` on a struct (or enum) injects a hidden
bit, known as the "drop-flag", into the struct (and likewise, each of
the the enum variants).  The drop-flag, in tandem with Rust's implicit
zeroing of dropped values, tracks whether a value has already been
moved to another owner or been dropped.  (See the "How dynamic drop
semantics works" appendix for more details if you are unfamiliar
with this part of Rust's current implementation.)

Here are some problems with this:

 * Most important: implicit memory zeroing is a hidden cost that all
   Rust programs are paying.  With the removal of the drop flag, we
   can remove implicit memory zeroing (or at least revisit its utility
   -- there may be other motivations for implicit memory zeroing,
   e.g. to try to keep secret data from being exposed to unsafe code).

 * Hidden bits are bad, part I: Users coming from a C/C++ background
   expect `struct Foo { x: u32, y: u32 }` to occupy 8 bytes, but if
   `Foo` implements `Drop`, the hidden drop flag will cause it to
   double in size (16 bytes).

 * Hidden bits are bad, part II: Some users have expressed an
   expectation that the drop-flag only be present for individual local
   variables, but that is not what Rust does currently: when `Foo`
   implements `Drop`, each instance of `Foo` carries a drop-flag, even
   in contexts like a `Vec<Foo>` or `[Foo, ..100]` where a program
   cannot actually move individual values out of the collection.
   Thus, the amount of extra memory being used by drop-flags is not
   bounded by program stack growth; the memory wastage is strewn
   throughout the heap.

So, those are the main motivations for removing the drop flag.

But, how do we actually remove the drop flag? The answer: By replacing
the dynamic drop semantics (that implicitly checks the flag to
determine if a value has already been dropped) with a static drop
semantics (that performs drop of certain values more eagerly,
i.e. before the end of their owner's lexical scope).

A static drop semantics essentially works by inserting implicit calls
to `mem::drop` at certain points in the control-flow to ensure that
the set of values to drop is statically known at compile-time.
(See "How static drop semantics works" in the detailed design for more
discussion of how this is done.)

There are two important things to note about a static drop semantics:

 1. It should be *equal* in expressive power to the Rust language as we know
    it today.  This is because, if the user is actually relying on the
    drop-flag today in some variable or field declaration `x: T`, they
    can replace that declaration with `x: Option<T>` and thus recreate
    the effect of the drop-flag.  (Note that formal comparisons of
    expressiveness typically say nothing about *convenience*.)

 2. Static drop semantics could be *surprising* to Rust programmers
    who are used to dynamic drop semantics.  In particular, an implicit early
    drop could lead to unexpected side-effects occurring earlier than
    expected.

The bulk of the Detailed Design is dedicated to mitigating that second
observation, in order to reduce the number of surprises for
Rust programmers.  The main idea for this mitigation is the addition
of one or more lints that report to the user when an side-effectful
early-drop will be implicitly injected into the code, and suggest to
them that they revise their code to remove the implicit `drop` injection
(e.g. by explicitly dropping the path in question, or by
re-establishing the drop obligation on the other control-flow paths,
or by rewriting the code to put in a manual drop-flag via
`Option<T>`).


# Detailed design

The change suggested by this RFC has three parts:

1. Change from a dynamic drop semantics to a static drop semantics,

2. Provide one or more lints to inform the programmer of potential
   surprises that may arise from earlier drops that are caused by the
   static drop semantics, and

3. Remove the implicitly added drop-flag, and the implicit zeroing of
   the memory for dropped values.


Each of the three parts is given its own section below.

## How static drop semantics works

This section presents a detailed outline of how static drop semantics
looks from the view point of someone trying to understand a Rust
program.

### Drop obligations

No struct or enum has an implicit drop-flag.  When a local variable is
initialized, that establishes a set of "drop obligations": a set of
structural paths (e.g. a local `a`, or a path to a field `b.f.y`) that
need to be dropped (or moved away to a new owner).

The drop obligations for a local variable `x` of struct-type `T` are
computed from analyzing the structure of `T`.  If `T` itself
implements `Drop`, then `x` is a drop obligation.  If `T` does not
implement `Drop`, then the set of drop obligations is the union of the
drop obligations of the fields of `T`.

When a path is moved to a new location, or consumed by a function call,
or when control flow reaches the end of its owner's lexical scope,
the path is removed from the set of drop obligations.

### Example of code with unchanged behavior under static drop semantics

Consider the following example, where `D` represents some struct that
introduces a drop-obligation, while `S` represents some struct that
does not.

```rust

struct Pair<X,Y>{ x:X, y:Y }

struct D; struct S;

impl Drop for D { ... }

fn test() -> bool { ... }

fn xform(d:D) -> D { ... }

fn f1() {
    // At the outset, the set of drop obligations is
    // just the set of moved input parameters (empty
    // in this case).

    //                                      DROP OBLIGATIONS
    //                                  ------------------------
    //                                  {  }
    let pDD : Pair<D,D> = ...;
    //                                  { pDD.x, pDD.y }
    let pDS : Pair<D,S> = ...;
    //                                  { pDD.x, pDD.y, pDS.x }
    let some_d : Option<D>;
    //                                  { pDD.x, pDD.y, pDS.x }
    if test() {
        //                                 { pDD.x, pDD.y, pDS.x }
        {
            let temp = xform(pDD.x);
            //                             {        pDD.y, pDS.x, temp }
            some_d = Some(temp);
            //                             {        pDD.y, pDS.x, temp, some_d }
        } // END OF SCOPE for `temp`
        //                                 {        pDD.y, pDS.x, some_d }
    } else {
        {
            //                             { pDD.x, pDD.y, pDS.x }
            let z = D;
            //                             { pDD.x, pDD.y, pDS.x, z }

            // This drops `pDD.y` before
            // moving `pDD.x` there.
            pDD.y = pDD.x;

            //                             {        pDD.y, pDS.x, z }
            some_d = None;
            //                             {        pDD.y, pDS.x, z, some_d }
        } // END OF SCOPE for `z`
        //                                 {        pDD.y, pDS.x, some_d }
    }

    // MERGE POINT: set of drop obligations must
    // match on all incoming control-flow paths...
    //
    // ... which they do in this case.

    //                                  {       pDD.y, pDS.x, some_d }

    // (... some code that does not change drop obligations ...)

    //                                  {       pDD.y, pDS.x, some_d }
}
```

Some notes about the example above:

It may seem silly that the line `some_d = None;` introduces a
drop-obligation for `some_d`, since `None` itself contains nothing to
drop.  The analysis infers whether such an assignment introduces a
drop-obligation based on the type of `some_d` (`Option<D>`, which
represents a drop-obligation, or at least a potential one).  Anyway,
the point is that having this assignment introduce a drop-obligation
there makes things happier at the merge point that follows it in the
control flow.

### Example of code with changed behavior under static drop semantics

```rust
// `f2` is similar to `f1`, except that it will have differing set
// of drop obligations at the merge point, necessitating a hidden
// drop call.
fn f2() {
    // At the outset, the set of drop obligations is
    // just the set of moved input parameters (empty
    // in this case).

    //                                      DROP OBLIGATIONS
    //                                  ------------------------
    //                                  {  }
    let pDD : Pair<D,D> = ...;
    //                                  {pDD.x, pDD.y}
    let pDS : Pair<D,S> = ...;
    //                                  {pDD.x, pDD.y, pDS.x}
    let some_d : Option<D>;
    //                                  {pDD.x, pDD.y, pDS.x}
    if test() {
        //                                  {pDD.x, pDD.y, pDS.x}
        {
            let temp = xform(pDD.y);
            //                              {pDD.x,        pDS.x, temp}
            some_d = Some(temp);
            //                              {pDD.x,        pDS.x, temp, some_d}
        } // END OF SCOPE for `temp`
        //                                  {pDD.x,        pDS.x, some_d}

        // MERGE POINT PREDECESSOR 1

        // implicit drops injected: drop(pDD.y)
    } else {
        {
            //                              {pDD.x, pDD.y, pDS.x}
            let z = D;
            //                              {pDD.x, pDD.y, pDS.x, z}

            // This drops `pDD.y` before
            // moving `pDD.x` there.
            pDD.y = pDD.x;

            //                              {       pDD.y, pDS.x, z}
            some_d = None;
            //                              {       pDD.y, pDS.x, z, some_d}
        } // END OF SCOPE for `z`
        //                                  {       pDD.y, pDS.x, some_d}

        // MERGE POINT PREDECESSOR 2

        // implicit drops injected: drop(pDD.y)
    }

    // MERGE POINT: set of drop obligations must
    // match on all incoming control-flow paths.
    //
    // For the original user code, they did not
    // in this case.
    //
    // Therefore, implicit drops are injected up
    // above, to ensure that the set of drop
    // obligations match.

    // After the implicit drops, the resulting
    // remaining drop obligations are the
    // following:

    //                                  {              pDS.x, some_d}

    // (... some code that does not change drop obligations ...)

    //                                  {              pDS.x, some_d}
}
```

### Control-flow sensitivity

Note that the static drop semantics is based on a control-flow
analysis, *not* just the lexical nesting structure of the code.

In particular: If control flow splits at a point like an if-expression,
but the two arms never meet

### match expressions and enum variants that move

The examples above used just structs and `if` expressions, but there
is an additional twist introduced by `enum` types.  The examples above
showed that a struct type can be partially moved: one of its fields
can be moved away while the other is still present, and this can be
faithfully represented in the set of drop obligations.
But consider an `enum` and `match`:

```rust
pub enum Pairy<X> { Two(X,X), One(X,X) }
pub fn foo<A>(c: || -> Pairy<A>,
              dA: |A| -> i8,
              dR: |&A| -> i8) -> i8 {
    let s = c();
    let ret = match s {
        Two(ref r1, ref r2) => {
            dR(r1) + dR(r2)
        }
        One(a1, a2) => {
            dA(a1) + dA(a2)
        }
    };
    c();
    ret
}
```

In the above code, which is legal today in Rust, we have an arm for
`Two` that matches the input `s` by reference, while the arm for `One`
moves `s` into the match.  That is, if the `Two` arm matches, then `s`
is left in place to be dropped at the end of the execution of `foo()`,
while if the `One` arm matches, then the `s` is deconstructed and
moved in pieces into `a1` and `a2`, which are themselves then consumed
by the calls to `dA`.

While we *could* attempt to continue supporting this style of code
(see "variant-predicated drop-obligations" in the Alternatives
section), it seems simpler if we just disallow it.  This RFC
proposes the following rule: if any arm in a match consumes
the input via `move`, then *every* arm in the match must consume the
input *by the end of each arm's associated body*.

That last condition is crucial, because it enables patterns like
this to continue working:

```rust
    match s {
        One(a1, a2) => { // a moving match here
            dA(a1) + dA(a2)
        }
        Two(_, _) => { // a non-binding match here
            helper_function(s)
        }
    };

```

### Type parameters

With respect to static drop semantics, there is not much to say about
type parameters: unless they have the `Copy` bound, we must assume
that they implement `Drop`, and therefore introduce drop obligations
(the same as types that actually do implement `Drop`, as illustrated
above).

## Early drop lints

Some users may be surprised by the implicit drops that are injected
by static drop semantics, especially if the drop code being executed
has observable side-effects.

Such side-effects include:

  * Memory being deallocated earlier than expected (probably harmless,
    but some may care)

  * Output buffers being flushed in an output port earlier than
    expected.

  * Mutex locks being released earlier than expected (a worrisome
    change in timing semantics when writing concurrent algorithms).

In particular, the injection of the implicit drops could silently
invalidate certain kinds of "resource acquisition is initialization"
(RAII) patterns.

It is important to keep in mind that one can always recreate the
effect of the former drop flag by wrapping one's type `T` in an
`Option<T>`; therefore, the problem is *not* that such RAII patterns
cannot be expressed.  It is merely that a user may assume that a
variable binding induces RAII-style effect, and that assumption is then
invalidated due to a errant move on one control-flow branch.

Therefore, to defend against users being surprised by the early
implicit drops induced by static drop semantics, this RFC proposes
adding lints that tell the user about the points in the control-flow
where implicit drops are injected.  The specific proposal is to add two
lints, named `quiet_early_drop` and `unmarked_early_drop`, with the
default settings `#[allow(quiet_early_drop)]` and
`#[warn(unmarked_early_drop)]`.  (The two lints are similar in name
because they provide very similar functionality; the only difference
is how aggressively each reports injected drop invocations.)

### The `unmarked_early_drop` lint

Here is an example piece of code (very loosely adapted from the Rust
`sync` crate):

```rust
        let (guard, state) = self.lock(); // (`guard` is mutex `LockGuard`)
        ...
        if state.disconnected {
            ...
        } else {
            ...
            match f() {
                Variant1(payload) => g(payload, guard),
                Variant2          => {}
            }

            ... // (***)

            Ok(())
        }
```

In the `Variant1` arm above, `guard` is consumed by `g`.  Therefore,
when the bit of code labeled with a `(***)` represents a span that,
when reached via the `Variant2` branch of the match statement, has the
`guard` still held under dynamic drop semantics, but the `guard` is
*released* under static drop semantics.

The `unmarked_early_drop` lint is meant to catch cases such as this,
where the user has inadvertently written code where static drop
semantics injects an implicit call to a side-effectful `drop` method.

Assuming that the `LockGuard` has a `Drop` impl but does not implement
the `QuietEarlyDrop` trait (see below `quiet_early_drop` lint below),
then the #[warn(unmarked_early_drop)]` lint will report a warning for
the code above, telling the user that the `guard` is moved away on the
`Variant1` branch but not on the other branches.  In general the lint
cannot know what the actual intention of the user was.  Therefore, the
lint suggests that the user either (1.) add an explicit drop call, for
clarity, or (2.)  reinitialize `guard` on the `Variant1` arm, or (3.)
emulate a drop-flag by using `Option<LockGuard>` instead of
`LockGuard` as the type of `guard`.

### The `quiet_early_drop` lint and `QuietEarlyDrop` trait

To be effective, a lint must not issue a significant number of false
positives: i.e., we do not want to tell the user about every site in
their crate where a `String` was dropped on one branch but not
another.

More generally, it is likely that most sites where `Vec<u8>` is
dropped are not of interest either.  Instead, the user is likely to
want to focus on points in the control flow where *effectful* drops
are executed (such as flushing output buffers or releasing locks).

Meanwhile, some users may still care about every potential
side-effect, even those that their libraries have deemed "pure".  Some
users may just want, out of principle, to mark every early drop
explicitly, in the belief that such explicitness improves code
comprehension.

Therefore, rather than provide just a single lint for warning about
all occurrences of injected early drops, this proposal suggests a
simple two-tier structure: by default, `Drop` implementations are
assumed to have significant side-effects, and thus qualify for warning
via the aforementioned `unmarked_early_drop` trait.  However, when
defining a type, one can implement the `QuietEarlyDrop` trait, which
re-categorizes the type as having a `Drop` implementation that "pure"
(i.e. does not exhibit side-effects that the client of the crate is
likely to care about).

An easy example of such a type whose `drop` method is likely to be
considered pure is `Vec<u8>`, since the only side-effect of dropping a
`Vec<u8>` is deallocation of its backing buffer.  More generally,
`Vec<T>` should be `QuietEarlyDrop` for any `T` that is also
`QuietEarlyDrop`.

If a type implements `QuietEarlyDrop`, then early implicit drops of
that type will no longer be reported by `#[warn(unmarked_early_drop)]`
(instead, such a type becomes the responsibility of the
`#[allow(quiet_early_drop)]` lint).  Thus, the former lint will
hopefully provide well-focused warnings with a low false-positive
rate, while the latter, being set to `allow` by default, will
not generate much noise.

Meanwhile, to ensure that a particular fn item has no hidden early
drops, one can turn on `#[deny(quiet_early_drop)]` and
`#[deny(unmarked_early_drop)]`, and then all statically injected drops
are reported (and the code rejected if any are present), regardless of
whether the types involved implement `QuietEarlyDrop` or not.

### Scope end for owner can handle mismatched drop obligations

Consider again the long examples from the "How static drop semantics
works" section above.  Both examples took care to explicitly include a
bit at the end marked with the comment "(... some code that does not
change drop obligations ...)".  This represents some potentially
side-effectful code that comes between a merge-point and the end of
the procedure (or more generally, the end of the lexical scope for
some local variables that own paths in the set of drop obligations).

If you hit a merge-point with two sets of drop obligations like `{
pDD.x, pDD.y }` and `{ pDD.x, z }`, and there is no side-effectful
computation between that merge-point and the end of the scope for
`pDD` and `z`, then there is no problem with the mismatches between
the set of drop obligations, and neither lint should report anything.

### Type parameters, revisited

We noted in the "How static drop semantics works" section that
type parameters are not particularly special with respect t
static drop semantics.

However, with the lints there is potential for type parameters to be
treated specially.

Nonetheless, this RFC currently proposes that type parameters not be
treated specially by the lints: if you have mismatched drop
obligations, then that represents a hidden implicit drop that you may
not have known was there, and it behooves you to make an explicit call
to `drop`.

(See further discussion in the "Unresolved Questions.")

## Removing the drop-flag; removing memory zeroing

With the above two pieces in place, the remainder is trivial.  Namely:
once static drop semantics is actually implemented, then the compiler
can be revised to no longer inject a drop flag into structs and enums that
implement `Drop`, and likewise memory zeroing can be removed.

# Drawbacks

* The lint may be annoying to users whose programs are not affected by
  the early drops. (We mitigate this by providing ways for users to
  opt-out of the lint `#[allow(unmarked_early_drops)]`, both in a
  lexically scoped fashion, like other lints, and in a type-based
  fashion via a `QuietEarlyDrop` trait.)

* The early drops may surprise the users who are used to the dynamic
  drop semantics. (We mitigate this by providing warnings via the
  lint, a clear path for rewriting code in terms of `Option<T>` to
  emulate a drop-flag, and a way for users to enable a stricter lint:
  `#[warn(quiet_early_drops)]` that reports all early drops, including
  those hidden via `QuietEarlyDrop`.)

* There may be benefits to implicit memory-zeroing that are not
  accounted for in this RFC, in which case we may end up only removing
  the drop-flag but not the implicit memory-zeroing.  Still, even if
  the only benefit is removing the drop-flag, it may still be worth
  the pain of static drop semantics.

# Alternatives

## Do nothing

Keep dynamic drop semantics, the drop flag, and the implicit memory
zeroing, paying their hidden costs in time and space.

## Require explicit drops rather than injecting them

Rather than injecting implicit drops where necessary, we could just
reject programs that have control-flow merge points with an
inconsistent set of incoming drop-obligations.

This would be equivalent to doing `#[deny(unmarked_early_drops)]`.

Felix (the author) was originally planning to take this approach, but
it became clear after a little experimentation that the annoyance
implied here made this a non-starter.

## Do this, but add support for variant-predicated drop-obligations

In "match expressions and enum variants" above, this RFC proposed a
rule that if any arm in a match consumes the input via `move`, then
every arm in the match must consume the input (by the end of its
body).

There is an alternative, however.  We could enrich the structure of
drop-obligations to include paths that are predicated on enum
variants, like so: `{(s is Two => s#0), (s is Two => s#1)}`.  This
represents the idea that (1.) all control flows where `s` is the `One`
variant dropped `s` entirely but also, (2.) all control flows where
`s` is the `Two` variant still has the 0'th and 1'st tuple components
remaining as drop obligations.

I do not currently know how to efficiently implement such an enriched
drop-obligation representation.  It seems it would get nasty when
one considers that these predicates can accumulate.

Also, if we do figure out how to implement this, we could add this
later backward compatibly.  I do not want to attempt to implement it
in the short-term.

## Does the match-arm rule break expressiveness claim?

I made the claim in a number of places that a static drop semantics
should be *equal* in expressive power to the Rust language as we know
it today.

However, when I made that claim, I did not think carefully
about the impliciations of the simple match arm rule.
Being forced to move out of the original owner in every arm
might imply that you cannot perform a mechanical transformation
on the program to reencode the prior behavior.
(I am not sure, it is too late at night right now for me to
be sure one way or another about this.)

## Associate drop flags with stack-local variables alone

I mentioned in "Hidden bits are bad, part II" that some users have
said they thought that the drop flag was only part of a local
variable's state, not part of every occurrence of a struct/enum.

We could try to explore this option, but it seems potentially very
complicated to me. E.g. would each droppable structs embedded within a
type get its own drop flag when stack-allocated (thus making the
layout of a type potentially depend on where it is allocated; that, or
the drop flags would have to be stored out-of-band compared to the
location of the struct itself in memory).

Besides, we would still need to do something about droppable types
that are *not* stack-allocated, which implies that we would still need
to do some sort of static drop semantics for those values.  And if we
are going to do that anyway, we might as well apply it across the
board.

## Separate individual and grouped instances of a type

(This is a variant on "Associate drop flags with stack-local variables
alone" above, but with a clearer vision for how it would work.)

Instead of having a single `size_of` value for a given type, treat
each type as having two different sizes: `indiv_size_of::<T>` and
`group_size_of::<T>`.

An individual instance can be moved on its own.  It gets a drop-flag.

An instance that is part of a group cannot be moved on its own.  The
whole group gets a drop flag, but each instance within it does not.
(I am assuming the group is itself allocated as an individual
instance, though perhaps one could recursively structure a group made
up of groups; that is an unclear aspect of this thought-exercise.)

When looking at a slice `[T]`, the instances are part of the group,
and one uses `group_size_of::<T>` for offset calculations.

For enums, structs, and tuples, the fields are considered individuals.
(Though perhaps we could have "atomic" enums/structs/tuples where the
fields are considered part of the group as a whole and cannot be
independently moved to new owners on control flow branches.)

This is an interesting thought exercise, but a pretty serious
language/library change, and it is not clear whether it would be any
better than static drop semantics in terms of Rust's language
complexity budget.

# Unresolved questions

## Names (bikeshed welcome)

I know there must be better names for lints and the traits being added
here.  It took me a while to come up with `unmarked_early_drop` to
categorized the types that implement `Drop` but do not have a
`QuietEarlyDrop` impl, and `quiet_early_drop` to categorize
(obviously) the types that do implement both traits.

## Which library types should be `QuietEarlyDrop`.

Side-effectfulness is in the eye of the beholder.  In particular,
I wonder how to handle `rc`; should it be:

```rust
impl<T> QuietEarlyDrop for Rc<T>
```

or should it be like other container types, like so:
```rust
impl<T:QuietEarlyDrop> QuietEarlyDrop for Rc<T>
```

One school of thought says that when you use `Rc`, you have
effectively given up on RAII for that type, at least when RAII is
viewed as being tied to a particular lexical scope, and therefore
all instances of `Rc<T>` should be considered to have pure drop
behavior, regardless of their contents.

But another school of thought says that since the destruction timing
of `Rc<T>` is predictable (compared to `Gc<T>` which in principle is
not predictable), then it would make sense to continue using the same
bubble-up semantics that the other collection types use.


## Should type parameters be treated specially

It is possible that generic code in general does not
need to be written with the same sort of care about drop timing that
code specific to a particular effectful type needs to be.  (Or rather,
it is possible that someone writing generic code will either want to
opt into receiving warnings about merge-points with mismatched drop
obligations.)



## How should moving into wildcards be handled?

In an example like:

```rust
enum Pair<X,Y> { Two(X,Y), One(X), Zed }
let x : Pair<FD, FD> = ...; // FD is some eFfectful Drop thing.
match x {
    Two(payload, _) => {
        ... code working with, then dropping, payload ...
    }
    One(payload) => {
        ... code working with, then dropping, payload ...
    }
    Zed => {
    }
}
```

In the above example, when the first match arm matches, we move `x`
into it, binding its first tuple component as `payload`.  But how
should the second tuple component of `x` be handled?  We need to drop
it at some point, since we need for all of `x` to be dropped at
the point in the control-flow where all of the match arms meet.

The most obvious options are:

 1. In any given arm, implicitly drop state bound to `_` at the end of the
    arm's scope.

    This would be as if the programmer had actually written:

    ```rust
    Two(payload, _ignore_me) => {
        ... code working with, then dropping, payload ...
    }
    ```

 2. In any given arm, implicitly drop state bound to `_` at the
    beginning of the arm's scope.

    This would be as if the programmer had actually written:

    ```rust
    Two(payload, _ignore_me) => {
        ... code working with, then dropping, payload ...
    }
    ```

 3. Disallow moving into `_` patterns; force programmer to name them
    and deal with them.

    While this is clearly a clean conservative solution, it is also
    awkward when you consider attempting to simplify the code above
    like so (illegal under the suggested scheme):

```rust
enum Pair<X,Y> { Two(X,Y), One(X), Zed }
let x : Pair<FD, FD> = ...; // FD is some eFfectful Drop thing.
match x {
    Two(payload, _) |
    One(payload) => {
        ... code working with, then dropping, payload ...
    }
    Zed => {
    }
}
```


# Appendices

## Program illustrating space impact of hidden drop flag


```rust
#![feature(unsafe_destructor, macro_rules)]

use std::mem;

struct S0;
struct D0;
struct S3u8 { _x: [u8, ..3] }
struct D3u8 { _x: [u8, ..3] }
struct Si32 { _x: i32, }
struct Di32 { _x: i32, }
struct Si64 { _x: i64, }
struct Di64 { _x: i64, }

macro_rules! show_me {
    ($e:expr) => { println!("{:>#50s}: {}", stringify!($e), $e); }
}

macro_rules! empty_drops {
    ($($i:ident)*) => { $(impl Drop for $i { fn drop(&mut self) { } })* }
}

empty_drops!(D0 D3u8 Di32 Di64)

fn main() {
    show_me!(mem::size_of::<S0>());
    show_me!(mem::size_of::<D0>());
    show_me!(mem::size_of::<S3u8>());
    show_me!(mem::size_of::<D3u8>());
    show_me!(mem::size_of::<Si32>());
    show_me!(mem::size_of::<Di32>());
    show_me!(mem::size_of::<Si64>());
    show_me!(mem::size_of::<Di64>());
    show_me!(mem::size_of::<[S3u8, ..100]>());
    show_me!(mem::size_of::<[D3u8, ..100]>());
    show_me!(mem::size_of::<[Si32, ..100]>());
    show_me!(mem::size_of::<[Di32, ..100]>());
}
```

## How dynamic drop semantics works

(This section is just presenting background information on the
semantics of `drop` and the drop-flag as it works in Rust today; it
does not contain any discussion of the changes being proposed by this
RFC.)

A struct or enum implementing `Drop` will have its drop-flag
automatically set to a non-zero value when it is constructed.  When
attempting to drop the struct or enum (i.e. when control reaches the
end of the lexical scope of its owner), the injected glue code will
only execute its associated `fn drop` if its drop-flag is non-zero.

In addition, the compiler injects code to ensure that when a value is
moved to a new location in memory or dropped, then the original memory
is entirely zeroed.

A struct/enum definition implementing `Drop` can be tagged with the
attribute `#[unsafe_no_drop_flag]`.  When so tagged, the struct/enum
will not have a hidden drop flag embedded within it. In this case, the
injected glue code will execute the associated glue code
unconditionally, even though the struct/enum value may have been moved
to a new location in memory or dropped (in either case, the memory
representing the value will have been zeroed).

The above has a number of implications:

 * A program can manually cause the drop code associated with a value
   to be skipped by first zeroing out its memory.

 * A `Drop` implementation for a struct tagged with `unsafe_no_drop_flag`
   must assume that it will be called more than once.  (However, every
   call to `drop` after the first will be given zeroed memory.)

### Program illustrating semantic impact of hidden drop flag

```rust
#![feature(macro_rules)]

use std::fmt;
use std::mem;

#[deriving(Clone,Show)]
struct S {  name: &'static str }

#[deriving(Clone,Show)]
struct Df { name: &'static str }

#[deriving(Clone,Show)]
struct Pair<X,Y>{ x: X, y: Y }

static mut current_indent: uint = 0;

fn indent() -> String {
    String::from_char(unsafe { current_indent }, ' ')
}

impl Drop for Df {
    fn drop(&mut self) {
        println!("{}dropping Df {}", indent(), self.name)
    }
}

macro_rules! struct_Dn {
    ($Dn:ident) => {

        #[unsafe_no_drop_flag]
        #[deriving(Clone,Show)]
        struct $Dn { name: &'static str }

        impl Drop for $Dn {
            fn drop(&mut self) {
                if unsafe { (0,0) == mem::transmute::<_,(uint,uint)>(self.name) } {
                    println!("{}dropping already-zeroed {}",
                             indent(), stringify!($Dn));
                } else {
                    println!("{}dropping {} {}",
                             indent(), stringify!($Dn), self.name)
                }
            }
        }
    }
}

struct_Dn!(DnA)
struct_Dn!(DnB)
struct_Dn!(DnC)

fn take_and_pass<T:fmt::Show>(t: T) {
    println!("{}t-n-p took and will pass: {}", indent(), &t);
    unsafe { current_indent += 4; }
    take_and_drop(t);
    unsafe { current_indent -= 4; }
}

fn take_and_drop<T:fmt::Show>(t: T) {
    println!("{}t-n-d took and will drop: {}", indent(), &t);
}

fn xform(mut input: Df) -> Df {
    input.name = "transformed";
    input
}

fn foo(b: || -> bool) {
    let mut f1 = Df  { name: "f1" };
    let mut n2 = DnC { name: "n2" };
    let f3 = Df  { name: "f3" };
    let f4 = Df  { name: "f4" };
    let f5 = Df  { name: "f5" };
    let f6 = Df  { name: "f6" };
    let n7 = DnA { name: "n7" };
    let _fx = xform(f6);           // `f6` consumed by `xform`
    let _n9 = DnB { name: "n9" };
    let p = Pair { x: f4, y: f5 }; // `f4` and `f5` moved into `p`
    let _f10 = Df { name: "f10" };

    println!("foo scope start: {}", (&f3, &n7));
    unsafe { current_indent += 4; }
    if b() {
        take_and_pass(p.x); // `p.x` consumed by `take_and_pass`, which drops it
    }
    if b() {
        take_and_pass(n7); // `n7` consumed by `take_and_pass`, which drops it
    }
    
    // totally unsafe: manually zero the struct, including its drop flag.
    unsafe fn manually_zero<S>(s: &mut S) {
        let len = mem::size_of::<S>();
        let p : *mut u8 = mem::transmute(s);
        for i in range(0, len) {
            *p.offset(i as int) = 0;
        }
    }
    unsafe {
        manually_zero(&mut f1);
        manually_zero(&mut n2);
    }
    println!("foo scope end");
    unsafe { current_indent -= 4; }
    
    // here, we drop each local variable, in reverse order of declaration.
    // So we should see the following drop sequence:
    // drop(f10), printing "Df f10"
    // drop(p)
    //   ==> drop(p.y), printing "Df f5"
    //   ==> attempt to drop(and skip) already-dropped p.x, no-op
    // drop(_n9), printing "DnB n9"
    // drop(_fx), printing "Df transformed"
    // attempt to drop already-dropped n7, printing "already-zeroed DnA"
    // no drop of `f6` since it was consumed by `xform`
    // no drop of `f5` since it was moved into `p`
    // no drop of `f4` since it was moved into `p`
    // drop(f3), printing "f3"
    // attempt to drop manually-zeroed `n2`, printing "already-zeroed DnC"
    // attempt to drop manually-zeroed `f1`, no-op.
}

fn main() {
    foo(|| true);
}
```

