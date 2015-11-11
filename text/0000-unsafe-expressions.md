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
    1 => if unsafe { senders.get_unchecked(0).send(value).is_err() } {
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

## Operator Precedence

The `unsafe` keyword would have similar precedence to any other prefix unary operator.
As a demonstration, consider the following expressions equivalent...

`unsafe a.m().n()` => `unsafe { a.m().n() }`
`unsafe a[i].foo()` => `unsafe { a[i].foo() }`
`unsafe a[i].foo() + 1` => `unsafe { a[i].foo() } + 1`
`(unsafe a.()).n()` => `unsafe { a.m() }.n()`

We have another similar (unstable) keyword in the language already: `box`. It would
make sense to tie these two together with the same precedence in the language.

One modification to the above may be to give it the precedence of the `||` closure
operator, which would make it more greedy:

`unsafe a[i].foo() + 1` => `unsafe { a[i].foo() + 1 }`

# Drawbacks

Less friction to the ergonomics of using `unsafe` is not necessarily a good thing,
we certainly don't want to encourage its use. I believe this change would still
be beneficial in general, however.

This may encourage `unsafe` to be more coarse-grained or greedy, rather than
making the scope of the `unsafe` explicit, one needs to be aware of its operator
precedence as discussed above. However, I've found the "minimal unsafe" style to not
be common in the greater rust ecosystem, which more often means the example
may be written as such:

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

... where much more of the code than is necessary is covered by `unsafe` while the
unrelated `_ => {}` block would normally not want it at all. Note that this issue
exists in other contexts as well, such as `unsafe` functions themselves having their
body implicitly be `unsafe`.


# Unresolved questions

- Is there any reason this proposal will not work from a grammar or syntax perspective?
- Are there any strange interactions with the language here that haven't been addressed?
- Does this conflict with or complicate parsing due to the `unsafe fn x()` declaration syntax?
- See the discussion at [rust-lang/rust#21192](https://github.com/rust-lang/rust/issues/21192)
  for opinions on the exact precedence of keyword unary operators.
