- Feature Name: `unsafe_expr`
- Start Date: 2015-10-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Proposal to extend the syntax of the `unsafe` keyword to be followed by an expression
rather than a block as it is currently defined.


# Motivation

`unsafe` is often used for single method calls, and requires one to wrap the expression
in curly braces. It looks ugly, makes for strange-looking inline blocks, causes code
to drift rightward in block indenting, and in general just doesn't feel very ergonomic.


# Detailed design

Let's take a look at a trivial example that makes use of unsafe:

```rust
match senders.len() {
    1 => if unsafe { senders.get_unchecked(0) }.send(value).is_err() {
        // ...
    },
    _ => {},
}
```

The inline braces are awkward and feel strange from a formatting perspective.
Now, suppose `unsafe` accepted an expression rather than a block.

```rust
match senders.len() {
    1 => if unsafe senders.get_unchecked(0).send(value).is_err() {
        // ...
    },
    _ => {},
}
```

We've removed the strange punctuation, and all works as usual. This change
would be backwards-compatible, as it will allow all existing Rust code to compile.


# Drawbacks

Less friction to the ergonomics of using `unsafe` is not necessarily a good thing,
we certainly don't want to encourage its use. I believe this change would still
be beneficial in general, however.

This encourages `unsafe` to be more coarse-grained, rather than wrapping the
single function call in its own block. However, I've found this style to not
be common in the greater rust ecosystem, which more often means the example
is written as such:

```rust
match senders.len() {
    1 => if unsafe { senders.get_unchecked(0).send(value).is_err() } {
        // ...
    },
    _ => {},
}
```

or worse, like so:

```rust
unsafe {
    match senders.len() {
        1 => if senders.get_unchecked(0).send(value).is_err() {
            // ...
        },
        _ => {},
    }
}
```

Note that this issue exists in other contexts as well, such as `unsafe` functions
themselves.


# Unresolved questions

- Is there any reason this proposal will not work from a grammar or syntax perspective?
- Are there any strange interactions with the language here that haven't been addressed?
- Does this conflict with or complicate parsing due to the `unsafe fn x()` declaration syntax?
