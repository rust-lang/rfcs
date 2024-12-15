- Feature Name: `never_patterns`
- Start Date: 2024-10-27
- RFC PR: [rust-lang/rfcs#3719](https://github.com/rust-lang/rfcs/pull/3719)
- Rust Issue: [rust-lang/rust#118155](https://github.com/rust-lang/rust/issues/118155)

# Summary
[summary]: #summary

A `!` pattern indicates a type with no valid values. It is used to indicate an impossible case when matching an empty type in unsafe code.

```rust
enum Void {}
unsafe fn empty_tup<T>(tup: *const (T, Void)) -> ! {
    unsafe {
        match *tup { ! }
    }
}
```

Note: this RFC is purely about a new pattern syntax. It does not propose changes to exhaustiveness checking or to the operational semantics of uninhabited types.


# Motivation
[motivation]: #motivation

Rust's unsafe semantics are access-based: the validity of data only matters when reading from/writing to a place. When pattern-matching, patterns only access the data (discriminants/values) they need to choose the right arm.

Empty types are funny because the natural way of matching them is to write no arms at all:

```rust
fn empty_tup<T>(tup: (T, !)) -> ! {
    match tup {}
}
```

Here the absence of an arm plays the role of a "read discriminant" kind of operation. This is fine in this case, but around unsafe code that interacts with possibly-uninitialized data, accesses should be explicitly visible in the pattern.

Today, when matching empty types inside places that may contain uninitialized data, rust requires a dummy match arm:
```rust
enum Void {}
unsafe fn empty_tup<T>(tup: *const (T, Void)) -> ! {
    unsafe {
        match *tup {
            _ => unreachable_unchecked(),
            // or
            (_, x) => match x {}
        }
    }
}

union MyUnion<T: Copy> {
    uninit: (),
    value: (T, !),
}
impl<T: Copy> MyUnion<T> {
    unsafe fn assume_init(self) -> ! {
        match self {
            MyUnion { value: (_, x) } => x,
        }
    }
}
```

This RFC proposes a new `!` pattern that works as an explicit "access data" or "assert validity" operation for uninhabited types. The examples above become:

```rust
enum Void {}
unsafe fn empty_tup<T>(tup: *const (T, Void)) -> ! {
    unsafe {
        match *tup { ! }
    }
}

union MyUnion<T: Copy> {
    uninit: (),
    value: (T, !),
}
impl<T: Copy> MyUnion<T> {
    unsafe fn assume_init(self) -> ! {
        match self { MyUnion { value: (_, !) } }
    }
}
```

<!--

```rust
enum E { A(i32), B(i32, !) }
let result: *const E = some_function();
unsafe {
    let x = match *result {
        A(x) => x,
        // An arm is required (https://github.com/rust-lang/unsafe-code-guidelines/issues/443) but `!` asserts validity of the `!` data so the arm doesn't need a body.
        B(_, !),
    };
    // Alternatively:
    let (A(x) | B(_, !)) = *result;
}
```

```rust
enum E { A(i32), B(i32, !) }
let val: *mut E = Box::leak(Box::new(E::A(42)));
// It may be possible to partially initialize this to the `B` variant
// in the future.
unsafe { set_discriminant_to_b(val) };
unsafe {
    match *val {
        A(x) => { ... }
        // This branch would then be reached without UB.
        B(..) => println!("Reachable!"),
    }
}

// It may be possible to construct a `&!` pointing to
// uninitialized data (unsafe, but valid).
let never_ref: &! = ...;
let result: Result<u8, &!> = Err(never_ref);
match result {
    Ok(x) => { ... }
    // This branch would then be reached without UB.
    Err(_) => println!("Reachable!"),
}
```

```rust
enum Void {}
let result: &Result<T, Void> = ...;
let x = match result {
    Ok(x) => x,
    Err(!), // No need for an arm body
};
// Or even
let (Ok(x) | Err(!)) = result;
```

-->


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Patterns can be used on a partially uninitialized place; the basic rule is that a pattern only accesses data that is directly mentioned in the pattern: an `Ok(..)` pattern requires a discriminant read, a binding requires a full read, a wildcard `_` accesses nothing etc. 

For uninhabited types (types with no valid values), normal patterns cannot be used to indicate an access since there is no data to be accessed. For this purpose, you can use the special `!` pattern.

The `!` pattern is allowed on any uninhabited type (such as `enum Void {}` or `(Result<!, !>, u32)`) and does an access to the underlying data. Since there can be no such data (on pain of UB), the corresponding arm is known to be unreachable and does not need a body:
```rust
enum Void {}
unsafe fn empty_tup<T>(tup: *const (T, Void)) -> ! {
    unsafe {
        match *tup { ! }
    }
}
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We add `!` to the syntax of patterns. A `!` pattern is accepted for any type that is visibly uninhabited (i.e. uninhabited taking into account private fields and `#[non_exhaustive]` annotations) from the item containing the match expression.

A pattern that contains a `!` is called a _never pattern_. For or-patterns, all alternatives must be never patterns for the whole pattern to be too. For example `Ok(x) | Err(!)` is not a never pattern but `(_, !)` and `Ok((_, !)) | Err(!)` are.

A never pattern is accepted as an arm in a match expression, and that arm takes no body nor guards.

```rust
let x: *const Option<(u32, (u32, !))> = ...;
match *x {
    None => { ... }
    Some((_, !)),
}
```

A never pattern is also accepted in other places a pattern would be (`if let`, destructuring `let`, etc.), and makes code that corresponds to that branch unreachable.

```rust
enum Void {}
impl Void {
    fn ex_falso<T>(!: Self) -> T {}
}
```

In terms of both semantics and exhaustiveness checking, a `!` pattern behaves like a binding. E.g. a `Some(!),` arm (absent match ergonomics) is semantically equivalent to `Some(x) => match x {}`. Indeed, they both indicate a load at a place of an uninhabited type, followed by an unreachable body.

Never patterns interact with match ergonomics as expected: a `!` pattern is allowed on e.g. `&(T, !)`.

A never pattern may be linted as "unreachable" if there is no well-behaved execution that can reach it. For example, if it was guaranteed that reading the discriminant of `Result<T, !>` could never return `Err`, the `Err(!)` pattern would be linted as unreachable. Never patterns are not otherwise linted as unreachable even when removing them would be accepted.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is that this new pattern is unlike other patterns (e.g. it doesn't require an arm body) which adds some maintenance burden. It's otherwise a simple feature.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Status quo alternative: we could do nothing, and consider that writing `Err(_) => unreachable!()` or `Err(x) => match x {}` is good enough. The drawback is more noticeable for or-patterns, e.g. `let (Ok(x) | Err(!)) = ...;` would have to become a `let .. else` or a full `match`.

Alternatively, we could allow omitting all empty arms; this is what the unstable `exhaustive_patterns` feature does today. A match like
```rust
match ... {
    Ok(_) => ...,
}
```
would implicitly contain a `Err(!)` arm, i.e. would trigger UB if the discriminant is `Err`. This was deemed a footgun and was the original motivation for this feature.


# Prior art
[prior-art]: #prior-art

This proposal is entirely based on @nikomatsakis and @RalfJung's original idea, presented in Niko's [blog post](https://smallcultfollowing.com/babysteps/blog/2018/08/13/never-patterns-exhaustive-matching-and-uninhabited-types-oh-my/).

The only other language I (Nadrieril) am aware of that has the intersection of features that would make this necessary is Zig. They are in the process of clarifying the semantics of their empty types (https://github.com/ziglang/zig/issues/15909); they may or may not end up encountering the same explicitness problem as us.

Ocaml and Adga have respectively [refutation cases](https://ocaml.org/manual/5.2/gadts-tutorial.html#s:gadt-refutation-cases) and [absurd patterns](https://agda.readthedocs.io/en/latest/language/function-definitions.html#absurd-patterns), both of which give a way to say "this pattern is impossible and I shouldn't have to write an arm for it". Their flavor of impossibility is related to types however, not runtime validity.

Something that's suprisingly semantically (and syntactically!) close is Haskell's [bang patterns](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/strict.html#bang-patterns-informal). Haskell is a lazy language and a pattern only evaluates the underlying value if needed to decide which branch to take. A `!x` instead pattern forces the evaluation of the underlying value. Rust's patterns are a bit lazy too: they lazily assert validity of the data they access. In Haskell one must be careful to evaluate the right things in the right order else you may diverge; in unsafe Rust similar care is needed to avoid UB.

Never patterns could then be described as playing the role of both a bang pattern ("please evaluate this") and an absurd pattern ("this is an impossible case") at the same time.

From [the lazy patterns literature](https://dl.acm.org/doi/abs/10.1145/3408989) we can also take the distinction between a "redundant" pattern (can be removed without changing semantics), and an "inaccessible" pattern (will never execute but cannot be removed without changing semantics). E.g., in rust:
```rust
let ptr: *const ! = ...;
match *ptr {
    // This causes a read of the value, hence UB if reached. Removing it would remove UB. Therefore this is inaccessible and not redundant.
    _x => {}
    _ => {}
}
```
Never patterns are our way to notate inaccessible patterns.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Does this cause problems with macros-by-example?
- Should the MIR-lowering of a never pattern actually include a place read or is a branch to `Unreachable` good enough?
- Out of scope: opsem and exhaustiveness decisions.


# Future possibilities
[future-possibilities]: #future-possibilities

This is a pretty self-contained feature.

This RFC is part of the [Patterns of Empty Types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html) project goal. In parallel with this, I plan to propose that we allow omitting empty arms behind references (e.g. `let Ok(_) = expr;` should be allowed for `expr: Result<T, &!>`). That way we'd get: for safe places (places that can be accessed in safe code), you can omit an empty arm; for unsafe places, you must write an arm and never patterns make this convenient.

This feature is closely related to the [`never_type`](https://github.com/rust-lang/rust/issues/35121) initiative, which aims to provide `!` as the default empty type in the language. These initiatives are independent and complement each other.
