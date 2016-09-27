- Start Date: (2016-08-03)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow methods which take uninhabited arguments to be omitted from trait
implementations.

# Motivation

Suppose we want to implement `Error` for `!`. One way to write this impl is
like this:

```rust
impl Error for ! {
    fn cause(&self) -> Option<&Error> {
        *self
    }

    fn description(&self) -> &str {
        *self
    }
}
```

Despite these methods having different return types, in both cases we can
simply dereference `self` and use the `!` type's ability to coerce to any type.
We can do something similar with `enum Void {}`:

```rust
impl Error for Void {
    fn cause(&self) -> Option<&Error> {
        match *self {
        }
    }

    fn description(&self) -> &str {
        match *self {
        }
    }
}
```

These function bodies never really execute. They only exist for the sake of
type-checking and they could be safely inferred to be the absurd function. Yet
we're forced to give them these rather wierd bodies which may look confusing to
newcomers and mislead them into thinking that there is actual code here.

What's worse, someone might instead try to write actual code in these bodies:

```rust
impl Error for ! {
    fn cause(&self) -> Option<&Error> {
        panic!();
    }

    fn description(&self) -> &str {
        "! error"
    }
}
```

This sort of thing is even more misleading because someone might think (in this
case) that they have panics in their code or that the string `"! error"` could
ever be printed. Ideally, the unreachable code lint would catch these, but it
doesn't currently and if it was enhanced in this way it would also catch the
orginal definitions given above. This means that there would be no to write
these methods at all without explicitly disabling the lint.

With this RFC, these two impls could instead be written simply as:

```rust
impl Error for ! {}
impl Error for Void {}
```

I'd like to encourage people to implement their traits for `!` whereever
there's a trivial impl. Otherwise people are likely to write libraries that
export traits that `!` could and should implement but doesn't.  To encourage
this it might be helpful to make these impls a one-liner which doesn't clutter
code with superfluous and meaningless method definitions.

Another way to think about this proposal is that we already allow infallible
pattern-matching in function argument lists. For example:

```rust
struct Foo { x: u32, y: u32 }
fn foo(Foo {x, y}: Foo) -> u32 { x + y }
```

This RFC would, in some sense, be an extension of this feature to unreachable
patterns.

# Detailed design

Allow method impls to be omitted from a trait impl if any of the argument types
are uninhabited. Note that this doesn't just include impls on `!`/`Void`.
Consider this simplified version of the `Handler` trait from `mio`.

```rust
trait Handler {
    type Timeout;

    fn timeout(&self, timeout: Self::Timeout);

    // Some other items omitted.
}
```

`mio` users can register a `Timeout` with their `Handler` and `mio` will call
the `timeout` method with the `Timeout` they gave it. If someone doesn't want
to use this feature of `Handler`s, the most sensible type to set `Timeout` to
is `!`. In this case, they should not be forced to define the `timeout` method
at all. By setting `Timeout = !` they've indicated that the method
should be, in some sense, deleted (or at least rendered unusable) similar to
how `!` can be used to "delete" unwanted enum variants.

As such, the impl should be able to be written simply as:

```rust
impl Handler for Foo {
    type Timeout = !;

    // Some other items omitted.
}
```

Note however that they could still write *dead* code which calls the uncallable
method. eg. `foo.timeout(panic!())`.

# Drawbacks

This might confuse people who don't know about this feature when they notice
that code compiles despite some method impls being apparently missing. However
I think having the method bodies missing completely aligns with the intuition
that `!` can be used to "delete" things like enum variants or, in this case,
method bodies. For example, for the type `Result<T, !>` it's fine to write
`match result { Ok(t) => t }` and have the `Err` variant missing completely (or
at least it will be once these changes are implemented).

# Alternatives

* Not do this.
* Make traits that have a trivial implementation for `!` have that impl
  automatically inferred. This would only solve the problem in some cases but
  would be a much more radical change overall.

# Related work

This is similar to the proposal to allow methods that can't be called due to
trait constraints to have their definitions omitted. See
[#20021](https://github.com/rust-lang/rust/issues/20021).

# Unresolved questions

None.

