- Feature Name: capture-disjoint-fields
- Start Date: 2017-11-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes that closure capturing should be minimal rather than maximal.
Specifically, existing rules regarding borrowing and moving disjoint fields
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

Note that there is open issue relating to this RFC
([#19004](https://github.com/rust-lang/rust/issues/19004)) where some discussion
has already taken place.

# Motivation
[motivation]: #motivation

In the rust language today, any variables named within a closure will be fully
captured. This was simple to implement but is inconstant with the rest of the
language because rust normally allows simultaneous borrowing of disjoint fields.
Remembering this exception adds to the mental burden of the programmer and
worsens rust's already terrible learning curve.

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

The borrow checker understands structs sufficiently to know that it's possible
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

There exists an nonoptimal but trivial desugar/workaround that covers all cases
via the following expansion:
```
capture := [|'&'|'&mut '] ident ['.' ident]*

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

This RFC should not be implemented as such a desugar. Rather, the two following
changes might be made:
- Borrowck rules are altered so that capture-by-reference allows simultaneous
  borrowing of disjoint fields.
- Codegen and borrowck are altered so that move closures capture only used
  fields.

## Examples of an ideal implementation

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
- The closure moves and captures `foo.a` but not `foo.b`.

```rust
let _a = &mut foo.a;
move || foo.b;
```
- Borrowck passes because `foo.a` and `foo.b` are disjoint.
- The closure moves and captures `foo.b`.

# Drawbacks
[drawbacks]: #drawbacks

This RFC does ruin the intuition that *all* variables named within a closure are
captured. I argue that that intuition is not really necessary, and killing it is
worth the cost.

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

- How exactly does codegen change?
- Depending on implementation, captured pointers may no longer be exclusive,
  careful with LLVM hints?
- Can borrowck be simplified as a result of this RFC?
- What unresolved questions do people have about this RFC?
