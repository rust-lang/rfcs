- Feature Name: fn lifetime
- Start Date: 2017-01-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `'fn` lifetime that is bound to the scope of the body of the current innermost function or closure.

# Motivation
[motivation]: #motivation

Doing this will enable us to declare that some values live longer than their containing scope, allowing us to "allocate" recursive data structures (e.g. linked lists) on the stack, as in:

```rust
fn foo() {
    struct Node<T> {
        data: T,
        next: Option<&'fn Node>,
    }

    let mut head : Node = Node { data: 0, &None };
    for i in iter {
        head = Node { data: i, Some(head) }
    }
}
```

# Detailed design
[design]: #detailed-design

The compiler is extended to recognise the `'fn` lifetime, and bind the region of the resulting type to the scope of the body of the current item or closure.

# Drawbacks
[drawbacks]: #drawbacks

This change makes a lifetime annotation change the behavior of the program (although in the motivating case, it only changes the program from "fails to compile" to "works").
The change incurs a considerable implementation cost, although a more complete "lifetime ascription" feature has been discussed in various places already (see the [Alternatives](alternatives) section for further discussion.

# Alternatives
[alternatives]: #alternatives

- do nothing. Rust has been working quite well without this feature so far.
  It is only of use in very specific situations (on the other hand those situations are currently not well handled in Rust at all).

- lifetime ascription – this is the idea to allow labels on each block (instead of only on loops) and treat the label names as lifetime designators.
  This is an awesome teaching device, as it allows us to make lifetimes explicit *in working code*, where we can now only use comments and pseudocode.
  Even with lifetime ascription, this feature is still useful, because ascribing a lifetime to a `fn` block gets awfully hard to parse, especially for a human.
  Consider following example:

  ```Rust
  fn somewhat_awkward<'a, T>(Foo<'a>) -> Box<T> + 'a 'wha { .. }
  ```

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions
[unresolved]: #unresolved-questions

- Should we allow `break 'fn ..` with the same semantics as `return ..`.
  In [RFC 1624](https://github.com/rust-lang/rfcs/blob/master/text/1624-loop-break-value.md) we got breaking with a value out of loops, so the value part is already established.
  This provides one canonnical syntax for breaking out of a loops or functions, leaving `return` just a convenience.
  In a future with full lifetime ascription, this would further generalize to supporting all labeled blocks, not just loops and functions.

  A downside is `continue 'a` will still only make sense when `'a` is bound to a loop, and `break 'a` won't make sense when `'a` is a lifetime parameter.
  That means users will need to understand their are three tiers of lifetimes: loop, block (including `'fn`), and, parameter, where each is less usable than the last.
  Today loop lables and lifetimes are disjoint in that neither can be used where the other is expected, though they do share the same syntax.

What parts of the design are still TBD?
