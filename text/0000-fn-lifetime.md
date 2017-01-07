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

There are three steps

1. Bind `'fn` in every closure or function body, representing the lifetime of the body.
   This is equivalent to adding an additional `'fn` lifetime parameter to every function that cannot be used in the function's type (argument declaration or where clause), nor manually instantiated by a caller.
   Closure literals currently do not allow explicit parameters, but the desugaring would be the same if they did.
   Note that the `'fn` lifetimes will shadow each other, whereas we don't currently allow lifetime shadowing.

2. Allow items inside functions to use lifetimes, including `'fn`, bound by the enclosing functions.
   Unclear how this interfacts with current well-formedness rules.

3. Change axioms of borrowing so it is fine if the lifetime is already entered, as long as the borrow does not outlive it *from* the point of borrowing *after*. For example:
   ```rust
   fn foo<'a>() {
       /* do stuff */
       let a = 0;
       let b: &'a i32 = &a;
   }
   ```
   This is currently prohibited because the function body is entered before a is initialized (and thus able to be borrowed).
   Under the new rules, this would be allowed because `a` is uninitialized when the function happens, and the "past" before the borrow begins can be ignored.

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

# Unresolved questions
[unresolved]: #unresolved-questions

- Should we allow `break 'fn ..` with the same semantics as `return ..`.
  In [RFC 1624](https://github.com/rust-lang/rfcs/blob/master/text/1624-loop-break-value.md) we got breaking with a value out of loops, so the value part is already established.
  This provides one canonnical syntax for breaking out of a loops or functions, leaving `return` just a convenience.
  In a future with full lifetime ascription, this would further generalize to supporting all labeled blocks, not just loops and functions.

  A downside is `continue 'a` will still only make sense when `'a` is bound to a loop, and `break 'a` won't make sense when `'a` is a lifetime parameter.
  That means users will need to understand their are three tiers of lifetimes: loop, block (including `'fn`), and, parameter, where each is less usable than the last.
  Today loop lables and lifetimes are disjoint in that neither can be used where the other is expected, though they do share the same syntax.
