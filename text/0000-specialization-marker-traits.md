- Feature Name: Specialization on impls with no items
- Start Date: 2016-04-30
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Assume that any trait impl which provides no items specializes any other impl
which could apply to the same type, regardless of overlap.

# Motivation
[motivation]: #motivation

This RFC is intended to replace [RFC #1268][1268]
entirely. Since its acceptance, little progress has been made on that RFC due to
implementation concerns. With the advent of specialization, there are potential
alternatives which are potentially easier to implement, and also addresses the
drawbacks brought by that RFC.

The motivations for the feature in general are the same as the original RFC,
improving the ergonomics of implementing marker traits.

Some examples include:

- the coercible trait design presents at [RFC #91][91];
- the `ExnSafe` trait proposed in [RFC #1236][1236].

# Detailed design
[design]: #detailed-design

Much of the concern around [RFC #1268][1268] was related to difficulty of
implementation. From the "Unresolved questions" section of the original RFC:

> Today, we prefer to break down an obligation like
> `Foo: MarkerTrait` into component obligations (e.g., `Foo: Send`). Due to
> coherence, there is always one best way to do this. That is, there is a best
> impl to choose. But under this proposal, there would not be.

> similar concerns arise with the proposals around specialization, so it may be
> that progress on that front will answer the questions raised here

Specialization appears to have made some progress on this front. However,
specialization is primarily focused on details of the trait impl, not on the
trait itself. For that reason, it is proposed that we amend [RFC #1268][1268] to
a definition which is more compatible with specialization.

**Any trait impl which provides no items is assumed to specialize any other
trait impl which could apply to the same type, regardless of overlap**. At the
time of writing, the definition of a trait item is an associated const, type, or
method. The exact meaning of an item may change in the future as language
features are added, but the intention is that an impl providing no items means
that the body of the trait impl is empty.

[RFC #1210][1210] states the following constraint around allowing overlap:

> There has to be a way to decide which of two overlapping impls to actually use
> for a given set of input types.

This continues to hold with the proposed change, as an impl with no body is
interchangeable with the less specific impl. This can be demonstrated with
specialization as it exists today:

```rust
trait SayHello {
    fn hi();
}

trait Foo {}
trait Bar {}

impl<T: Foo> SayHello for T {
    default fn hi() {
        println!("Hello there");
    }
}

impl<T: Foo + Bar> SayHello for T {}
```

This change effectively has the same meaning as the original RFC, as an empty
trait body would be rejected for a trait that requires adding items. However,
there is an important difference in this meaning compared to the original.

Overlap would become allowed between impls on traits with items, as long as
neither overlapping trait provides any items. Expanding on the previous example,
the following would hold:

```rust
trait SayHello {
    fn hi();
}

trait Foo {}
trait Bar {}
trait Baz {}

impl<T: Foo> SayHello for T {
    default fn hi() {
        println!("Hello there");
    }
}

impl<T: Foo + Bar> SayHello for T {}
impl<T: Foo + Baz> SayHello for T {}
```

This means that the only drawback from [RFC #1268][1268] is removed. Adding a
defaulted item to a marker trait is no longer a breaking change. Adding items
without a default is already considered to be a breaking change today.

Other than overlap, the rules for what impls are accepted remain the same,
however. Any impl which is not a `default impl` must provide an implementation
for all required items unless they are provided by a less specific impl. That
means that the following code would not compile:

```rust
trait SayHello {
    fn hi();
}

trait Foo {}
trait Bar {}

impl<T: Foo> SayHello for T {
    default fn hi() {
        println!("Hello there");
    }
}

impl<T: Bar> SayHello for T {}
```

For any type `T` which implements `Bar`, but not `Foo`, the second impl is
incomplete, and hence would be rejected. However, if the item has a default at
the trait level, the code would compile:

```rust
trait SayHello {
    fn hi() {
        println!("Hi, there!");
    }
}

trait Foo {}
trait Bar {}

impl<T: Foo> SayHello for T {
    default fn hi() {
        println!("Hello there");
    }
}

impl<T: Bar> SayHello for T {}
```

Since `T: Bar` provides no items, it is assumed to specialize `T: Foo` if
permitted for the given type when checking for coherence. Typeck would be
satisfied for any type that fulfills `T: Foo`, `T: Bar`, or `T: Foo + Bar`.
During monomorphisation in codegen, if a type satisfies either `Foo` or
`Foo + Bar`, the code will print "Hello there". If a type satisfies `Bar` but
not `Foo + Bar`, it will print "Hi, there!".

The overall semantics are moderately complex, but become quite simple when you
think of them in the context of the original intention. When a trait does not
require any items, overlap is allowed.

Due to the semantics of specialization, there is one additional wrinkle, which
is that any `default impl` with no body would be accepted. This is quirky, but
appears to be harmless. In practice there is no reason to write an empty
`default impl` for anything. The effects of providing a `default impl` for a
trait with no items is intentionally left unspecified (and in fact is unclear
from the definition of specialization without this RFC).

# Drawbacks
[drawbacks]: #drawbacks

While this proposal is likely easier to implement, and more flexible than
[RFC #1268][1268], it is more complex and has edge cases that did not exist in
the original.

# Alternatives
[alternatives]: #alternatives

Continuing with [RFC #1268][1268] as written.

# Unresolved questions
[unresolved]: #unresolved-questions

Is this actually easier to implement than the original RFC? This RFC was written
as a result of attempting to implement [#1268][1268], and this proved to be much
more straightforward. However, this needs to be verified by someone more well
versed in the compiler's internals.

[1210]: https://github.com/rust-lang/rfcs/pull/1210
[1236]: https://github.com/rust-lang/rfcs/pull/1236
[1268]: https://github.com/rust-lang/rfcs/pull/1268
[91]: https://github.com/rust-lang/rfcs/pull/91
