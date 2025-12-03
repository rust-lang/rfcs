- Feature Name: `mut_non_mut`
- Start Date: 2024-12-13
- RFC PR: [rust-lang/rfcs#3742](https://github.com/rust-lang/rfcs/pull/3742)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

the `mut` keyword has two meanings currently:
 - When applied to a reference, it means the reference is not aliased
 - When applied to a local binding, it means that the binding is allowed be re-assigned or borrowed using `&mut`

The first of these is fundamental to Rust's safety guarantees and enables some optimizations.  The second,
however, is more of a lint: the error message may help catch mistakes, but ignoring it doesn't violate
type safety or subvert any of Rust's global correctness guarantees.

This RFC proposes that the re-assignment or creation of a `&mut` to a local variable not
marked with `mut` be changed from a hard error to a deny-by-default lint diagnostic (called
`mut_non_mut` for the sake of discussion).

This will allow users, if they wish, to `allow(mut_non_mut)` or `warn(mut_non_mut)`.

# Motivation
[motivation]: #motivation

This idea has been discussed previously:

 - [2014 Blog post](https://smallcultfollowing.com/babysteps/blog/2014/05/13/focusing-on-ownership/)
 - [2021 IRLO thread](https://internals.rust-lang.org/t/lack-of-mut-in-bindings-as-a-deny-by-default-lint/15818)
 - [Zulip thread](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Idea.3A.20Downgrade.20.22immutable.20variable.E2.80.9C.20errors.20to.20lints)

One benefit of this change is reduced friction: adding and removing of the need for `mut` on a given local
can happen frequently when refactoring, similar to how variables and functions may change from used to unused
and back.  In this context, a fatal error for `mut_non_mut` can be a flow interruption.  While adding `mut`
in all the right places is something you may want to do _eventually_, just as you may want to
delete unused variables/functions, it's not something that needs to prevent you from running the program.

Another benefit is towards language consistency/explainability: having `mut_non_mut` be a configurable lint
makes it reasonable that it will sometimes have false positives/negatives.  On the other hand, making it a
hard error adds its rules to the language sematics (and worse: conflates them with borrow checking):
 - Does `mut` mean mutable?  Surely not, since there exists interior mutability.
 - But `mut` doesn't just mean "exclusive," either: local variables are _already_ exclusive
   by nature, so wouldn't need `mut` under that definition.
 - If you can't get a `&mut` to a local declared without `mut`, how does `drop` do it?
 - There is also the "move loophole," where moving out of a binding lets you mutate it:
   ```rs
   let not_mutable = vec![1, 2, 3];
   let mut not_mutable = not_mutable;
   not_mutable.push(4);
   ```
 - But that loophole _doesn't_ work for `move` closures, which ostensibly do the same thing:
   ```rs
   let not_mutable = vec![1, 2, 3];
   let mut f = move || { 
      not_mutable.push(4);
   };
   f();
   ```

The IRLO and Zulip threads include more examples of inconsistency, and also some examples of people
being confused by the inconsistency or mistakenly believing that the `mut_non_mut` check
is related to Rust's static safety guarantees.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

> Explain the proposal as if it was already included in the language and you were teaching
> it to another Rust programmer.

In addition to using `&mut` to declare an exclusive reference, the `mut` keyword may be used to indicate
that a local variable or pattern binding is mutated later on.  The `unused_mut` and
`mut_non_mut` lints will notify you when a `mut` binding is never mutated or when a non-`mut` binding is
mutated, respectively.  Ensuring correct usage of `mut` annotations on all local variables will help
future readers of your code to better understand which values are and are not expected to change.

The below will warn that you added a `mut` annotation to a local that is not mutated:
```rs
fn get_num() -> i32 {
    let mut x = 12; // WARNING! Variable does not need to be mutable
    x
}
```

The below will warn that you omitted the `mut` annotation from a local that was re-assigned:
```rs
fn get_num() -> i32 {
    let x = 12;
    x = 13; // WARNING! Assigned twice to immutable variable `x`
    x
}
```

Creating a `&mut` reference is also considered a mutation, since a mutation could occur through the reference:
```rs
fn get_num() -> i32 {
    let x = 12;
    let px = &mut x; // WARNING! Borrowing `x` as mutable when it is not declared as mutable
    *px = 13;
    x
}
```

`mut` annotations may be applied to other forms of bindings as well, and the same rules apply:
```rs
fn do_something(mut arg: Option<i32>){ // Function parameter with `mut`
    let (mut a, b) = (12, 13);         // Irrefutable pattern with `mut`
    if let Some(mut val) = arg {}      // Refutable pattern with `mut`
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The borrow checker has to deal with the `mut` vs `&mut` distinction, and therefore
already has special handling for locals.  Making immutable re-assignment a lint just
means converting local-specific code paths in borrowck to trigger lints instead of fatal
diagnostics.  Because this is a relaxation of existing language semantics,
the change is fully backwards-compatible, and making the lint deny-by-default means even code which _fails_
to compile will continue to do so after the change. 

One interesting consequence of this change is that it allows mutation in places where there
previously was no syntax to allow it, for example in `ref` bindings:

```rs
let ref x = 12;
x = x; // Hard error currently, would become lint.

let ref mut x = 12;
x = x; // Also a hard error currently, would also become lint.
```

Note: [Upcoming changes to match ergonomics](https://github.com/rust-lang/rust/issues/123076) also enable this.

# Drawbacks
[drawbacks]: #drawbacks

Most languages with the concept of immutability treat mutation of immutable locals as a hard error, so the
current behavior is consistent with that.  There are reasons to treat Rust as a special case, though:
 - Some of the other languages with immutability treat immutable locals as compile-time evaluable,
   but rust handles that with a distinct concept (`const`)
 - Rust is "immutable by default," and intentionally makes mutable locals the "noisier" option.  This makes
   the hard error more ergonomically costly in Rust than it would be in other languages, since it comes
   up more frequently in practice

Converting the hard error to a lint, even a deny-by-default one, gives people the option to turn
the lint off.  While Rust doesn't have a culture of blanket disabling lints, people coming from
other languages with default-mutability may find it a tempting option.  On the other hand, a
compilation-blocking error hard-coded into rustc is about the most forceful way possible to
request that developers use a particular coding style, and it's not clear whether that's
warranted in this case.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Alternatives have been proposed to independently address the friction and language inconsistency:

## Addressing Friction

One alternative is for people inconvenienced by `mut_non_mut` is to err on
the side of using `mut` everywhere, since `unused_mut` is a lint already and not a hard
error.  There are problems with this strategy, though:
 - There are a _lot_ of places where you'd have to add `mut`, including function
   arguments and patterns, ~~and there are even places where syntax doesn't _exist_ to declare
   bindings mutable~~. (Note: this is no longer true with
   [Upcoming changes to match ergonomics](https://github.com/rust-lang/rust/issues/123076))
 - If we assume the goal is to eventually remove
   the unnecessary `mut`s, then that's a lot more `mut`s to be removed than the compiler
   would have requested you to add if you had left them all off.  You would 
   then have to put them all _back_ the next time you intend to refactor.
 - If you disable `unused_mut` entirely and never remove unused `mut`s, then you're in a significantly _worse_
   place than if you had been allowed to leave them off: the code now has a bunch of extra noise
   in it in the form of possibly-used `mut`s.

There also exists `cargo fix --broken-code` for automatically adding and removing `mut` annotations.
This can be triggered to run automatically on save/build, but:
 - it doesn't seem like `cargo fix` was meant to be used in this way.  Some of its transformations
   are not reversible, and the tool warns when you try to use it on uncommitted code.  It could
   be updated to support this use case, though.
 - If inadvertent mutation actually _would_ have caught a mistake, `cargo fix` will potentially
   hide that mistake.
 - `cargo fix` performs more than just `mut` additions/removals, meaning other important warnings
   may be suppressed as well

## Addressing language inconsistency

We could split the `mut` keyword into separate keywords which independently
describe mutability and ownership, with the latter being something like `&uniq`
or `&only`.  Such a change would likely be too dramatically breaking to be worth
doing, however.

# Prior art
[prior-art]: #prior-art

Reassignment or mutation of immutable bindings is considered
an error by just about every programming language with the
concept of immutability.  There are still some interesting
cases, though.

## C and C++

Technically speaking, a conforming C compiler is allowed to ignore or emit warnings
for reassignment-of-const, although MSVC, Clang, and GCC all treat it as an error.
C has no "mutable borrow" operation, but the equivalent to mutably borrowing an
immutable local compiles with warnings under the above three compilers:

```c
int const a = 12;
int *pa = &a; // WARNING! initialization discards 'const' qualifier from pointer target type
```

C++ is more strict, and rejects the above example with an error.  In either language,
it's UB to attempt to modify the value of `a`.

C and C++ compilers can use `const` annotations on local variables
for optimization purposes, but stricter reference semantics allow Rust
to perform the same optimizations regardless of how locals are annotated.

## Zig

Zig has recently pushed in the _opposite_ direction,
and now refuses to compile a program when variables are declared mutable but are never mutated:

```zig
var foo = 12; // ERROR! local variable is never mutated
_ = foo;
```

The change was somewhat controversial, and many discussions about it can be found online.  Points raised in
those discussions are very similar to the points raised here.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - How many lints should there be?  Reassignment and mutable borrowing
   seem distinct enough to warrant separate lints.
 - What name should the lint(s) have?
    - `mut_non_mut`
       - used in this document
       - short for "mutate non-mutable"
       - a bit cryptic
    - `missing_mut`
       - proposed in the IRLO thread
       - suggests that `mut` can be added to fix, which isn't always true
    - `unmarked_variable_mutation`
       - Avoids mentioning the `mut` keyword directly to avoid confusion

# Future possibilities
[future-possibilities]: #future-possibilities

There exist other candidates for conversion from hard-errors to lints, but they usually fall into one of
two categories: either they don't cause enough friction to be worth changing, or successfully
compiling the "error" case would require extra code generation.
