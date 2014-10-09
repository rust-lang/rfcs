- Start Date: 2014-10-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Elision of lifetime parameters in output position should be rejected when there
is an explicitly bound lifetime parameter in scope, e.g.

```rust
trait Foo<'a> {
    fn bar(&self) -> &int;
}
```

because it is not possible for the lifetime elision rules to correctly decide
whether this should be

```rust
trait Foo<'a> {
    fn bar<'b>(&'b self) -> &'b int;
}
```

or

```rust
trait Foo<'a> {
    fn bar(&self) -> &'a int;
}
```

Currently, the former choice is always taken.

# Motivation

The [lifetime elision RFC](https://github.com/rust-lang/rfcs/blob/master/active/0039-lifetime-elision.md)
proposed two rules for lifetimes in output positions:

* If there is exactly one input lifetime position (elided or not), that lifetime
is assigned to all elided output lifetimes.

* If there are multiple input lifetime positions, but one of them is `&self` or
`&mut self`, the lifetime of `self` is assigned to all elided output lifetimes.

These rules only consider lifetime parameters that are bound by the function
itself, rather than lifetime parameters that are already bound in scope of the
current `trait` or `impl` block. For example, the first rule claims that in a
case like

```rust
trait Foo<'a> {
    fn bar(self, s: &str) -> &int;
}
```

the correct default is to interpret this as

```rust
trait Foo<'a> {
    fn bar<'b>(self, s: &'b str) -> &'b int;
}
```

and the second rule claims that in a case like

```rust
trait Foo<'a> {
    fn bar(&self) -> &int;
}
```

the correct default is to interpret this as

```rust
trait Foo<'a> {
    fn bar<'b>(&'b self) -> &'b int;
}
```

There are real-world cases where you want the other choice of the output
lifetime parameter being `'a`. The realization that elision made the wrong
choice may only arise after attempting to use the function in a new context,
and the resulting error messages are not very good, as reported in [rust-lang/rust#17822](https://github.com/rust-lang/rust/issues/17822).

# Detailed design

This change simply modifies the rules from the [lifetime elision RFC](https://github.com/rust-lang/rfcs/blob/master/active/0039-lifetime-elision.md)
so that the elision of output lifetimes only occurs when there are no bound lifetimes.

* Each elided lifetime in input position becomes a distinct lifetime parameter.

* If there are no explicitly bound lifetimes in scope and there is exactly one
input lifetime position (elided or not), that lifetime is assigned to all elided
output lifetimes.

* If there are no explicitly bound lifetimes in scope and there are multiple
input lifetime positions, but one of them is `&self` or `&mut self`, the
lifetime of `self` is assigned to all elided output lifetimes.

* Otherwise, it is an error to elide an output lifetime.

Just like in the original lifetime elision RFC, these rules apply to both
`trait` and `impl` blocks.

Taken as-is, these rules imply that explicit lifetime parameters are required
for output lifetimes in the following cases:

* Functions that already have explicit lifetime parameters, e.g.

```rust
trait Foo {
    fn bar<'a>(&self, x: &'a int, y: &'a int) -> &int;
}
```

* Closure parameters, e.g.

```rust
trait Foo<'a> {
    fn bar(&self, f: |int| -> &int) -> &'a int;
}
```

* Nested functions, e.g.

```rust
impl<'a> A<'a> {
    fn bar(&self) -> &'a int {
        fn f(a: &int) -> &int {
            ...
        }

        ...
    }
}
```

# Drawbacks

This change will require more lifetime parameters to be written. However, it
will only require lifetime parameters to be written after they have already
appeared in scope, so it won't cause a user to encounter lifetime parameters
before they otherwise would.

Reversing the proposed change is backwards compatible, whereas making this
change is not backwards compatible.

# Alternatives

* There could be no changes made at all, and users will just have to deal with
the problems caused by the current behavior.

* Lifetime error messages could be improved to the point that they sufficiently
reduce the frustrations caused by the elision rules giving incorrect lifetimes.
This doesn't address the problem that the errors are not given when writing the
function that has incorrectly specified lifetimes; they are only given when
attempting to use the function, possibly after many other uses of the function
that were accepted.

* Develop a lifetime inference algorithm that looks at the implementations of
functions, determines unnecessarily narrow lifetime parameters, and warns the
user about them. This can't handle parameters in trait definitions. It can't
catch all cases, since the body of a function might have to change in order to
satisfy the wider lifetime parameters. There may also be multiple choices due to
multiple bound lifetime parameters.

# Unresolved questions

Some consequences of these modified rules were noted in the detailed design
section that might need to be revisited or refined.

Should the lifetime parameter in the following example be elided?

```rust
trait Foo<'a> {
    fn bar(self) -> &int
}
```

There is an unambiguous choice here, but it is currently rejected. Since this
can be addressed in the future in a backwards compatible way, it may be best
to leave it outside of the consideration of this RFC.
