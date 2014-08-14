- Start Date: 2014-08-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Revise language semantics for drop so that all branches move or drop
the same pieces of state ("value-paths").  Add lint(s) to inform the
programmer of situations when this new drop-semantics could cause
side-effects of RAII-style code (e.g. releasing locks, flushing
buffers) to occur sooner than expected.  Remove the dynamic tracking
of whether a value has been dropped or not; in particular, (1.) remove
implicit addition of a drop-flag by `Drop` impl, and (2.) remove
implicit zeroing of the memory that occurs when values are dropped.

# Motivation

Currently, implementing `Drop` on a struct (or enum) injects a hidden
bit, known as the "drop-flag", into the struct (and likewise, each of
the the enum variants).  The drop-flag, in tandem with Rust's implicit
zeroing of dropped values, tracks whether a value has already been
moved to another owner or been dropped.  (See "How dynamic drop
semantics works" for more details.)

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
(See "How static drop semantics works" for more details.)

There are two important things to note about a static drop semantics:

 1. It is *equal* in expressive power to the Rust language as we know
    it today.  This is because, if the user is actually relying on the
    drop-flag today in some variable or field declaration `x: T`, they
    can replace that declaration with `x: Option<T>` and thus recreate
    the effect of the drop-flag.  (Note that formal comparisons of
    expressiveness typically say nothing about *convenience*.)

 2. Static drop semantics could be *surprising* to Rust programmers
    who are used to dynamic drop semantics, in terms of it potentially
    invalidating certain kinds of
    resource-acquisition-is-initialization (RAII) patterns.
    In particular, an implicit early drop could lead to unexpected
    side-effects occurring earlier than expected.  Such side-effects
    include:

     * Memory being deallocated (probably harmless, but some may care)

     * Output buffers being flushed in an output port.

     * Mutex locks being released (more worrisome potential change
       in timing semantics).

The bulk of the Detailed Design is dedicated to mitigating that second
observation, in order to reduce the expected number of surprises for
Rust programmers.  The main idea for this mitigation is the addition
of one or more lints that report to the user when an side-effectful
early-drop will be implicitly injected into the code, and suggest to
them that they revise their code to remove the implicit injection
(e.g. by explicitly dropping the path in question, or by
re-establishing the drop obligation on the other control-flow paths,
or by rewriting the code to put in a manual drop-flag via
`Option<T>`).


# Detailed design

TODO

This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.

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

I am not certain of all the implementation details of changes to the
`trans` module in `rustc`.

# Appendix

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

## How static drop semantics works

(This section is presenting a detailed outline of how static drop
semantics, which is part of this RFC proposal, looks from the view
point of someone trying to understand a Rust program.)

No struct or enum has an implicit drop-flag.  When a local variable is
initialized, that establishes a set of "drop obligations": a set of
structural paths (e.g. a local `a`, or a path to a field `b.f.y`) that
need to be dropped (or moved away to a new owner).

The drop obligations for a local variable `x` of struct-type `T` are
computed from analyzing the structure of `T`.  If `T` itself
implements `Drop`, then `x` is a drop obligation.  If `T` does not
implement `Drop`, then the set of drop obligations is the union of the
drop obligations of the fields of `T`.

When a path is moved to a new location or consumed by a function call,
it is removed from the set of drop obligations.

For example:

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
    //                                  {pDD.x, pDD.y}
    let pDS : Pair<D,S> = ...;
    //                                  {pDD.x, pDD.y, pDS.x}
    let some_d : Option<D>;
    //                                  {pDD.x, pDD.y, pDS.x}
    if test() {
        //                                  {pDD.x, pDD.y, pDS.x}
        {
            let temp = xform(pDD.x);
            //                              {       pDD.y, pDS.x, temp}
            some_d = Some(temp);
            //                              {       pDD.y, pDS.x, temp, some_d}
        } // END OF SCOPE for `temp`
        //                                  {       pDD.y, pDS.x, some_d}
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
    }

    // MERGE POINT: set of drop obligations must
    // match on all incoming control-flow paths...
    //
    // ... which they do in this case.

    // (... some code that does not change drop obligations ...)

    //                                  {       pDD.y, pDS.x, some_d}
}

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

    // After the implict drops, the resulting
    // remaining drop obligations are the
    // following:

    //                                  {              pDS.x, some_d}
}
```

