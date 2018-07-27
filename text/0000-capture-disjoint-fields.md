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
captured. This was simple to implement but is inconsistent with the rest of the
language because rust normally allows simultaneous borrowing of disjoint
fields. Remembering this exception adds to the mental burden of the programmer
and makes the rules of borrowing and ownership harder to learn.

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

This RFC does not propose any changes to the borrow checker. Instead, the MIR
generation for closures should be altered to produce the minimal capture.
Additionally, a hidden `repr` for closures might be added, which could reduce
closure size through awareness of the new capture rules *(see unresolved)*.

In a sense, when a closure is lowered to MIR, a list of "capture expressions" is
created, which we will call the "capture set". Each expression is some part of
the closure body which, in order to capture parts of the enclosing scope, must
be pre-evaluated when the closure is created. The output of the expressions,
which we will call "capture data", is stored in the anonymous struct which
implements the `Fn*` traits. If a binding is used within a closure, at least one
capture expression which borrows or moves that binding's value must exist in the
capture set.

Currently, lowering creates exactly one capture expression for each used
binding, which borrows or moves the value in its entirety. This RFC proposes
that lowering should instead create the minimal capture, where each expression
is as specific as possible.

This minimal set of capture expressions *might* be created by starting with the
existing capture set (one maximal expression per binding) and then iterativly
modifying and splitting the expressions by adding additional dereferences and
path components.

A capture expression is minimal if it produces...

- a value that is not a struct or borrowed struct (e.g. primitive, enum, union).
- a value that is used by the closure in its entirety (e.g. passed outside the closure).
- any other case where the expression can not be made more specific (see below).

Additionally capture expressions are not allowed to...

- call impure functions.
- make illegal moves (for example, out of a `Drop` type).
- *(comments please, am I missing any cases?)*.

Note that *all* functions are considered impure (including to overloaded deref
impls). And, for the sake of capturing, all indexing is considered impure *(see
unresolved)*. It is possible that overloaded `Deref::deref` implementations
could be marked as pure by using a new, marker trait (such as `DerefPure`) or
attribute (such as `#[deref_transparent]`). However, such a solution should be
proposed in a separate RFC. In the meantime, `<Box as Deref>::deref` could be a
special case of a pure function *(see unresolved)*.

Note that, because capture expressions are all subsets of the closure body,
this RFC does not change *what* is executed. It does change the order/number of
executions for some operations, but since these must be pure, order/repetition
does not matter. Only changes to lifetimes might be breaking. Specifically, the
drop order of uncaptured data can be altered.

We might solve this by considering a struct to be minimal if it contains unused
fields that implement `Drop`. This would prevent the drop order of those fields
from changing, but feels strange and non-orthogonal *(see unresolved)*.
Encountering this case at all could trigger a warning, so that this extra rule
could exist temporarily but be removed over the next epoc *(see unresolved)*.

## Reference Examples

Below are examples of various closures and their capture sets.

```rust
let foo = 10;
|| &mut foo;
```

- `&mut foo` (not a struct)

```rust
let a = &mut foo.a;
|| (&mut foo.b, &mut foo.c);
somefunc(a);
```

- `&mut foo.b` (used in entirety)
- `&mut foo.c` (used in entirety)

The borrow checker passes because `foo.a`, `foo.b`, and `foo.c` are disjoint.

```rust
let a = &mut foo.a;
move || foo.b;
somefunc(a);
```

- `foo.b` (used in entirety)

The borrow checker passes because `foo.a` and `foo.b` are disjoint.

```rust
let hello = &foo.hello;
move || foo.drop_world.a;
somefunc(hello);
```

- `foo.drop_world` (owned and implements drop)

The borrow checker passes because `foo.hello` and `foo.drop_world` are disjoint.

```rust
|| println!("{}", foo.wrapper_thing.a);
```

- `&foo.wrapper_thing` (overloaded `Deref` on `wrapper_thing` is impure)

```rust
|| foo.list[0];
```

- `foo.list` (indexing is impure)

```rust
let bar = (1, 2); // struct
|| myfunc(bar);
```

- `bar` (used in entirety)

```rust
let foo_again = &mut foo;
|| &mut foo.a;
somefunc(foo_again);
```

- `&mut foo.a`  (used in entirety)

The borrow checker fails because `foo_again` and `foo.a` intersect.

```rust
let _a = foo.a;
|| foo.a;
```

- `foo.a`  (used in entirety)

The borrow checker fails because `foo.a` has already been moved.

```rust
let a = &drop_foo.a;
move || drop_foo.b;
somefunc(a);
```

- `drop_foo` (owned and implements drop)

The borrow checker fails because `drop_foo` can not be moved while borrowed.

```rust
|| &box_foo.a;
```

- `&<Box<_> as Deref>::deref(&box_foo).b` (pure function call)

```rust
move || &box_foo.a;
```

- `box_foo` (`move` forces full capure of `box_foo`, since it can not be
  destructured)

# Drawbacks
[drawbacks]: #drawbacks

This RFC does ruin the intuition that all variables named within a closure are
*completely* captured. I argue that that intuition is not common or necessary
enough to justify the extra glue code.

# Rationale and alternatives
[alternatives]: #alternatives

This proposal is purely ergonomic since there is a complete and common
workaround. The existing rules could remain in place and rust users could
continue to pre-borrow/move fields. However, this workaround results in
significant useless glue code when borrowing many but not all of the fields in
a struct. It also produces a larger closure than necessary which could make the
difference when inlining.

# Unresolved questions
[unresolved]: #unresolved-questions

- How to optimize pointers. Can borrows that all reference parts of the same
object be stored as a single pointer? How should this optimization be
implemented (e.g. a special `repr`, refinement typing)?

- Any reason for non-overloaded index-by-constant to be pre-evaluated? It is
technically pure. Could this be left as an implementation/optimization
decision?

- How to signal that a function is pure. Is this even needed/wanted? Any other
places where the language could benefit?

- Should `Box` be special?

- Drop order can change as a result of this RFC, is this a real stability
  problem? How should this be resolved?
