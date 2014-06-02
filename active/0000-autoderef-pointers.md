- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Non-raw pointers in Rust should be automatically dereferenced by the compiler,
making them behave more like C++ references.

# Motivation

In C and C++, an `int*` is conceptually an `Option<&int>` (in Rust) because the
pointer may be `NULL` so it can't be safely dereferenced. Even if a pointer is
not `NULL`, the memory behind it might have already been freed.

But these concerns do not exist in Rust. Non-raw pointers cannot be `NULL` and
are always valid so they can _always_ be safely dereferenced. This means that we
can do away with explicit pointer deref in most cases, increasing usability.

A simple example where implicit pointer deref would help is the following:

```rust
let x: &u8 = &10;
let y = x == '\n' as u8;
```

Currently, this code fails with `expected &u8 but found u8`. Since there is zero
danger in dereferencing `x` and the compiler knows that `x` is a `&u8`, this
should be done for the user instead of asking them to do this manually.

Critically, Rust _already_ implicitly dereferences pointers in some cases, but
this is done piecemeal. From the tutorial:

```rust
let point = &~Point { x: 10.0, y: 20.0 };
println!("{:f}", point.x);
```

We don't force the user to write `(*point).x` because doing so would be a big
usability hit.

**Outside of Rust, pointers that are auto-dereferenced are neither a novel idea
nor considered controversial. After being present in C++ for 30 years,
references are widely considered superior to pointers; [standard practice says
they should be preferred over pointers wherever possible][so-practice] as does
the [official isocpp.org FAQ][isocpp].**

The main reason why references are preferred to pointers in C++ is because of
the increased safety, but the syntactical benefits are a factor. The main point
is that effectively no one in the C++ community considers the "hidden"
dereference operations to be an issue.

Forcing the user to manually deref pointers introduces a user-level cost without
providing a benefit.

# Detailed design

In all cases where the compiler sees a non-raw pointer (`&T`, `Box<T>` or a
layered combination, however deep) _and needs a `T` for the code to compile_, it
implicitly converts `&T` to a `T` (where the implicit conversion actually
performs a dereference). This would generalize the custom handling of method
invocations (as stated above, `.` will auto-deref).

Note that the auto-deref _only_ happens if the code wouldn't compile with `&T`
but would with `T`; if there's a `foo` method implemented for both `&T` and `T`,
the one for `&T` would be called. In other words, the auto-deref acts like a
fallback.

More specifically, the following code:

```rust
fn foo(_: int) {}
fn main() {
    let x = &&&3;
    let y = x;
    foo(y);
}
```

would compile as if it were written as:

```rust
fn foo(_: int) {}
fn main() {
    let x = &&&3;
    let y = x;
    foo(***y);
}
```

Types implementing `Deref` are considered non-raw pointers for the purposes of
this change.

Raw pointers would be left untouched; they have to be dereferenced manually by
the user.

There would be no change to how variables or functions are declared.

The user would still be allowed to dereference pointers by hand. This would make
this change entirely backwards-compatible.

The syntactical overhead of Rust non-raw pointers when compared to C++
references would disappear and the resulting design is strictly better; non-raw
Rust pointers are always safe to deref, whereas C++ references are safe to deref
by language standard fiat (creating a `NULL` or dangling reference is undefined
behavior), which doesn't prevent bugs. This would provide yet another selling
point for converting C++ programmers to Rust.

Programmers coming from dynamic languages would have an easier time learning
Rust with a lower likelihood of falling into the trap of "just throw sigils at
it until it compiles."

Manual deref of raw pointers would serve as both a slight deterrent against
their usage and would visually call attention to the presence of a raw pointer.
In codebases where all non-raw pointers are auto-derefed, `*x` would immediately
signal a possible failure location.

# Drawbacks

Possible drawbacks:

- Difference between usage of non-raw pointers and raw pointers. This shouldn't
  be a big issue because raw pointer usage should be rare. As stated above, this
  can be considered a benefit.

# Alternatives

- Leave the current system in place.
    - Rust continues to auto-deref pointers in some places and not in others.
      This remains arbitrary.
    - The current system [remains confusing][confusing].
    - The current system leads to some justifiable user anger over "the compiler
      knows how to get a `T` from my `&T` and knows that doing so is safe, why
      do I have to it by hand?"
    - C++ programmers examining Rust continue to decry the "Rust giveth and Rust
      taketh away" result of switching from C++ references (more safety, but
      usability hit).
- Also introduce auto-ref alongside auto-deref. Code like the following would
  compile:

    ```rust
    let x = 10;
    let y : Option<&u8> = Some(x);
    ```
  The compiler would always prefer `T` if given `T`, but would deref and produce
  `&T` if needed to make the code compile. This would make most usage of `&`
  obsolete along with `*`.

# Unresolved questions

None currently.

[so-practice]: http://stackoverflow.com/a/7058373/1672783
[isocpp]: https://isocpp.org/wiki/faq/references#refs-vs-ptrs
[confusing]: http://www.reddit.com/r/rust/comments/272i7p/rfc_autodereferencing_nonraw_pointers/chws5x7
