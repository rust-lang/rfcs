- Feature Name: fn lifetime
- Start Date: 2017-01-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `'fn` lifetime that is bound to the scope of the body of the current
innermost function or closure.

# Motivation
[motivation]: #motivation

Doing this will enable us to declare that some values live longer than their
containing scope, allowing us to "allocate" recursive data structures (e.g.
linked lists) on the stack, as in:

```rust
struct Node<T> {
    data: T,
    next: Option<&'fn Node>,
}

let mut head : Node = Node { data: 0, None };
for i in iter {
    head = Node { data: i, Some(head) }
}
```

# Detailed design
[design]: #detailed-design

The compiler is extended to recognise the `'fn` lifetime, and bind the region
of the resulting type to the scope of the body of the current item or closure.

# Drawbacks
[drawbacks]: #drawbacks

This change makes a lifetime annotation change the behavior of the program
(although in the motivating case, it only changes the program from "fails to
compile" to "works"). The change incurs a considerable implementation cost,
although a more complete "lifetime ascription" feature has been discussed in
various places already (see the [Alternatives](alternatives) section for
further discussion.

# Alternatives
[alternatives]: #alternatives

- do nothing. Rust has been working quite well without this feature so far. It
is only of use in very specific situations (on the other hand those situations
are currently not well handled in Rust at all).

- lifetime ascription – this is the idea to allow labels on each block (instead
of only on loops) and treat the label names as lifetime designators. This is an
awesome teaching device, as it allows us to make lifetimes explicit *in working
code*, where we can now only use comments and pseudocode. Even with lifetime
ascription, this feature is still useful, because ascribing a lifetime to a
`fn` block gets awfully hard to parse, especially for a human. Consider
following example:

```Rust
fn somewhat_awkward<'a, T>(Foo<'a>) -> Box<T> + 'a 'wha { .. }
```

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions
[unresolved]: #unresolved-questions

What parts of the design are still TBD?
