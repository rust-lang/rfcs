- Feature Name: capture-disjoint-fields
- Start Date: 2017-11-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes that closure capturing should be minimal rather than maximal.
Conceptually, existing rules regarding borrowing and moving disjoint fields
should be applied to capturing. If implemented, the following code examples
would become valid:

```rust
let _a = &mut foo.a;
|| &mut foo.b; // Error! cannot borrow `foo`
```

```rust
let _a = &mut foo.a;
move || foo.b; // Error! cannot move `foo`
```

Note that some discussion of this has already taken place:
- rust-lang/rust#19004
- [Rust internals forum](https://internals.rust-lang.org/t/borrow-the-full-stable-name-in-closures-for-ergonomics/5387)

# Motivation
[motivation]: #motivation

In the rust language today, any variables named within a closure will be fully
captured. This was simple to implement but is inconstant with the rest of the
language because rust normally allows simultaneous borrowing of disjoint fields.
Remembering this exception adds to the mental burden of the programmer and makes
the rules of borrowing and ownership harder to learn.

The following is allowed; why should closures be treated differently?

```rust
let _a = &mut foo.a;
loop { &mut foo.b; } // ok!
```

This is a particularly annoying problem because closures often need to borrow
data from `self`:

```rust
pub fn update(&mut self) {
    // cannot borrow `self` as immutable because `self.list` is also borrowed as mutable
    self.list.retain(|i| self.filter.allowed(i));
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust understands structs sufficiently to know that it's possible
to borrow disjoint fields of a struct simultaneously. Structs can also be
destructed and moved piece-by-piece. This functionality should be available
anywhere, including from within closures:

```rust
struct OneOf {
    text: String,
    of: Vec<String>,
}

impl OneOf {
    pub fn matches(self) -> bool {
        // Ok! destructure self
        self.of.into_iter().any(|s| s == self.text)
    }

    pub fn filter(&mut self) {
        // Ok! mutate and inspect self
        self.of.retain(|s| s != &self.text)
    }
}
```

Rust will prevent dangerous double usage:

```rust
struct FirstDuplicated(Vec<String>)

impl FirstDuplicated {
    pub fn first_count(self) -> usize {
        // Error! can't destructure and mutate same data
        self.0.into_iter()
            .filter(|s| &s == &self.0[0])
            .count()
    }

    pub fn remove_first(&mut self) {
        // Error! can't mutate and inspect same data
        self.0.retain(|s| s != &self.0[0])
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC does not propose any changes to borrowck. Instead, the MIR generation
for closures should be altered to produce the minimal capture. Additionally, a
hidden `repr` for closures might be added, which could reduce closure size
through awareness of the new capture rules *(see unresolved)*.

A capture is a list of expressions, which we will call "capture items". These
expressions are parts of the closure body that will be pre-evaluated when the
closure is created.

If the closure is `move`:

- For each binding used in the closure, add a capture item which moves the
value of that binding.

Otherwise:


- For each binding used in the closure, add a capture item which borrows the
value of that binding. The mutability of this borrow must be inferred from the
closure body.

Next, the capture list is iterativly narrowed. When an item is narrowed, it is
removed from this list, but at least one new item is generated. Remember that
each capture item is some pre-evaluatable part of the closure body. Narrowing
stops when all items can not be narrowed further.

Any field of a captured struct is considered "used" if the body of the closure
requires its value.

Any item that evaluates to borrowed struct can be narrowed so long as narrowing
does not produce any expression that additionally calls a function or
overloaded deref. When a borrowed struct is narrowed, new items are generated
which similarly borrow each used field.

Any item that evaluates to an owned struct can be narrowed so long as the
struct does not implement `Drop`. Additionally, we might forbid narrowing if
the struct contains unused fields that implement `Drop`. This will prevent the
drop order of those fields from changing, but feels strange and non-orthogonal
*(see unresolved)*. Encountering this case at all could trigger a warning, so
that this extra rule could exist but be removed over an epoc *(see
unresolved)*. When an owned struct is narrowed, new items are generated which
move each used field.

It has also been proposed that any item `x`, which evaluates to `Box` can be
narrowed to `Deref::deref(x)`.  *(see unresolved)*. Potentially this could be
generalized over some concept of a "pure" deref, as marked by some new
annotation or trait *(see unresolved)*.



There exists an nonoptimal desugar/workaround that covers all cases via the
following expansion:

```
capture := [|'&'|'&mut '] ident ['.' moveable_ident]*

'|' args '|' [$e($c:capture):expression]* =>
'{'
    ['let' $name:ident '=' $c ';']*
    '|' args '|' [$e($name)]*
'}'
```

Applied to the first two examples:

```rust
let _a = &mut foo.a;
let b = &mut foo.b;
|| b;
```

```rust
let _a = &mut foo.a;
let b = foo.b;
move || b;
```

This proves that the RFC can be safely implemented without violating any
existing assumptions. Also, because the compiler would become strictly more
lenient, it is nonbreaking.

This RFC should *not* be implemented as such a desugar. Rather, the two
following changes might be made:

- Borrowck rules are altered so that capture-by-reference allows simultaneous
  borrowing of disjoint fields. Field references are either individually
  captured or all captured by a single nonexclusive pointer to the whole struct.
- Codegen and borrowck are altered so that move closures destructure and capture
  only used fields when possible. This does require some minimal knowledge of
  destructuring rules (types that implement `Drop` must be fully moved).

The compiler should resolve captures recursively, always producing the minimal
capture even when encountering complex cases such as a `Drop` type inside a
destructurable type.

## Examples of an ideal implementation

Below are examples of how the compiler might idealy handle various captures:

```rust
|| &mut foo.a;
```

- Borrowck passes because `foo` is not borrowed elsewhere.
- The closure captures a pointer to `foo.a`.

```rust
let _a = &mut foo.a;
|| &mut foo.b;
```

- Borrowck passes because `foo.a` and `foo.b` are disjoint.
- The closure captures a pointer to `foo.b`.

```rust
let _a = &mut foo.a;
|| (&mut foo.b, &mut foo.c);
```

- Borrowck passes because `foo.a`, `foo.b`, and `foo.c` are disjoint.
- The closure captures a pointer to `foo`.

```rust
move || foo.a;
```

- Borrowck passes because `foo` is not borrowed elsewhere.
- The closure moves and captures `foo.a` but not `foo.b` because `foo` can be
  destructured.

```rust
let _a = &mut foo.a;
move || foo.b;
```

- Borrowck passes because `foo.a` and `foo.b` are disjoint.
- The closure moves and captures `foo.b` because `foo` can be destructured.

```rust
move || drop_foo.a;
```

- Borrowck passes because no part of `drop_foo` is borrowed elsewhere.
- The closure moves and captures all of `drop_foo` because `drop_foo` implements
  `Drop`.

```rust
let _hello = &foo.hello;
move || foo.drop_world.a;
```

- Borrowck passes because `foo.hello` and `foo.drop_world` are disjoint and no
  part of `drop_world` is borrowed elsewhere.
- The closure moves and captures all of `foo.drop_world` but not `foo.hello`
  because `drop_world` implements `Drop` but `foo` does not.

## Example errors

```rust
let _foo_again = &mut foo;
|| &mut foo.a;
```

- Borrowck fails because `_foo_again` and `foo.a` intersect.

```rust
let _a = foo.a;
|| foo.a;
```

- Borrowck fails because `foo.a` has already been moved.

```rust
let _a = drop_foo.a;
move || drop_foo.b;
```

- Borrowck fails because `drop_foo` can not be destructured.

# Drawbacks
[drawbacks]: #drawbacks

This RFC does ruin the intuition that *all* variables named within a closure are
captured. I argue that that intuition is not common or necessary enough to
justify the current approach.

# Rationale and alternatives
[alternatives]: #alternatives

This proposal is purely ergonomic since there is a complete and common
workaround. The existing rules could remain in place and rust users could
continue to pre-borrow/move fields. However, this workaround results in
significant boilerplate when borrowing many but not all of the fields in a
struct. It also produces a larger closure than necessary which could be the
difference between inlining and heap allocation.

# Unresolved questions
[unresolved]: #unresolved-questions

- Depending on implementation, captured pointers may no longer be exclusive,
  careful with LLVM hints?
- How can safe MIR be generated when capturing non-exclusive pointers
  (dereferenced disjointly)?
  - Use refinement typing?
- Do non lexical lifetimes have any bearing on this particular inconvenience?
- Are detailed error messages required for complex cases (e.g.
  `foo.drop_world.a` being captured while `foo.drop_world.b` is borrowed)?
