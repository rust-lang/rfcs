- Start Date: 2014-05-15
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

The lambda syntax should produce a uniquely typed callable object based on the captures.

# Motivation

Rust's closures should be replaced with a more flexible design acting as sugar for the semantics
elsewhere in the language. It should be possible to capture by-value, along with moving around or
even returning the resulting closure type. Closures should not imply indirect calls, but rather
provide the same choice between static and dynamic already provided by the choice between trait
bounds on generics and trait objects. This feature is provided as part of C++11, and C++14
introduces the necessary support for returning closures.

# Drawbacks

This will require a significant amount of implementation work. The change to by-value capture
semantics will be backwards incompatible, requiring a significant amount of code to be ported.

# Detailed design

This feature would be implemented as sugar over existing ones, so the design is quite simple.

Consider the following closure definition:

```rust
let x = 5; |a, b| a * x + b + x
```

This will be expanded to an instance of a unique type, essentially acting as the following type
definition and construction, but with an unspeakable anonymous type and inaccessible fields:

```rust
struct UniqueClosure_$uniqueid {
    x: int
}

impl |int, int| -> int for UniqueClosure_$uniqueid {
    fn call(&mut self, a: int, b: int) -> int {
        a * self.x + b + self.x
    }
}

UniqueClosure_$uniqueid { x: x }
```

The captures will be performed by-value, so non-`Copy` types would be moved into the closure by
default. Since references and mutable references are values, closure expressions do not need any
extra complexity. The resulting type will be `Send` if all the captures are `Send` along with having
the other built-in traits as appropriate just as a normal `struct` definition would.

The current borrowed closures will be directly replaced by the borrowed trait object syntax: `&mut
|u32, bool| -> f64`. It should be possible for any user-defined type to implement this special
trait.

Capturing a mutable reference (`&mut T`) will be special-cased to perform a reborrow (`&mut *x`) in
order to make closures more flexible under the current type system.

# Alternatives

An alternative is leaving the by-reference capture semantics, preventing closures from being
returned if local state is captured. The main motivation behind this proposal is to allow these
patterns, so lack of by-value capture cannot be seriously considered. It would also lead down the
path of closures being more than just sugar, so the existing struggle with closure-specific
soundness and code generation issues would continue.

Variadic generics would remove the need for the trait to be special, but the syntactic sugar will
always be desirable. Closures will exist only to provide sugar, and it makes sense to provide type
sugar in addition to the expression sugar. It would simply be replacing the existing type syntax for
closures, so it has a low cost and high return.

## Syntactic sugar for by-reference and by-mutable-reference captures

The need to capture by reference is uncommon, as shown by an analysis of the existing code. Since
references are first-class values, no special syntax is required. However, it would be possible to
add sugar in the future.

The `ref` and `ref mut` markers are already used in patterns to destructure by-reference, and
re-using these for closure captures would be unambiguous.

Consider the example above again, this time with a by-reference capture:

```rust
let x = 5; |a, b| a * x + b + *(ref x)
```

This would be sugar for the following snippet:

```rust
let x = 5; { let ref_x = &x; |a, b| a * x + b + *ref_x }
```

Capturing the same variable with a different capture strategy would just introduce a new field
inside the closure object to store it. Since this can be defined in terms of existing features, it
will eliminate the need to treat closure captures as special cases.

The usage of `ref` and `ref mut` has the virtue of being unambiguous, introducing no extra keywords
and avoiding a noisy capture list.

# Unresolved questions

It will be possible to return these closures from other closures. However, regular functions
currently require a concrete type signature so this proposal alone is not enough to return them. The
restriction on functions could be relaxed to permit the return type to be an anonymous type
implementing a trait, or in other words an "unboxed trait object". This is future work for another
proposal.

The current generics syntax makes passing an unboxed closure uglier than the current closures. This
could be improved by building on the "unboxed trait object" concept introduced above and allowing
the following:

```rust
fn foo(x: |u64, u64| -> u64) { ... }
```

However, this is also out of the scope of this proposal.

It might be possible to fit `proc` into this system by making use of `self` and `~self`, but the
regular `call` method can not safely move out of the captured environment. A separate trait would be
required, perhaps using the reserved `once` keyword as a prefix to the closure type sugar. It could
also be done via a future implementation of variadic generics without the sugar, but it would be
significantly uglier and would still be a magical lang item.
