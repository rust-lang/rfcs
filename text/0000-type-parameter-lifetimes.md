- Feature Name: Type parameter lifetimes
- Start Date: 2023-07-11
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `'Self` and `'T` lifetimes for every type parameter `T`.

# Motivation
[motivation]: #motivation

Rust types may embed lifetimes, for example:
```rust
trait Greet {
    fn greet(&self);
}

struct Person<'a> {
    name: &'a str,
}

impl<'a> Greet for Person<'a> {
    fn greet(&self) {
        println!("Hello, {}!", self.name)
    }
}
```

Where that lifetime is known we can already accomodate this:
```rust
impl<'a> Person<'a> {
    // Return value implicitly has lifetime 'a
    fn boxed(self) -> Box<Person<'a>> {
        Box::new(self)
    }

    // Return value explicitly has lifetime 'a
    fn as_boxed_greeter(self) -> Box<dyn Greet + 'a> {
        Box::new(self)
    }
}
```

In other cases we currently cannot. (According to [RFC 599], `Box<dyn Greet>` resolves to `Box<dyn Greet +'static>` in these contexts.)
```rust
trait BadBoxer: Greet {
    // Valid declaration, but not implementable for Person<'a>:
    fn as_boxed(self) -> Box<dyn Greet>;
}

// Valid function, not usable for Person<'a>:
fn box_static_greeter<G: Greet + 'static>(g: G) -> Box<dyn Greet> {
    Box::new(g)
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In a trait declaration or implementation, `'Self` refers to the lifetime of the implementing type. We may use this as follows.
```rust
trait Greet {
    fn greet(&self);

    fn as_boxed(self) -> Box<dyn Greet + 'Self>;
}

struct Person<'a> {
    name: &'a str,
}

impl<'a> Greet for Person<'a> {
    fn greet(&self) {
        println!("Hello, {}!", self.name)
    }

    // Here 'Self = 'a
    fn as_boxed(self) -> Box<dyn Greet + 'Self> {
       Box::new(self)
    }
}
```

For every type parameter `T`, `'T` refers to the lifetime of the type parameter.
```rust
// Here 'G is the lifetime of the input type:
fn box_greeter<G: Greet>(g: G) -> Box<dyn Greet + 'G> {
    Box::new(g)
}

#[test]
fn test_box_greeter() {
    fn foo(name: &str) -> Box<dyn Greet> {
        box_greeter(Person::new(name)).greet();
    }
    let _ = foo("Charlie");
}
````

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In trait declarations and trait implementations, lifetime `'Self` is introduced, resolving to the lifetime of `Self`. Currently `'Self` is not a legal name so this is not a conflict.

(NOTE: we only need `'Self` for trait definitions and implementations, but unless there is an issue I propose `'Self` be available for inherent implementations too.)

For every type parameter `T`, the lifetime `'T` is introduced. Due to naming conventions a conflict here is unlikely, but for backwards compatibility a new `'T` must not be introduced when an existing lifetime by that name is in scope (if this happens, it is recommended that a warning be emitted; potentially a future edition can make this an error).

# Drawbacks
[drawbacks]: #drawbacks

This introduces a bunch of new lifetimes in existing code scopes. Depending on method of implementation, this could impact compiler performance slightly.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We have managed without this facility for a long time, but as the above examples show it is not particularly hard to find a use-case. Personally see this as an obvious missing feature of the language which has negligible impact on perceived complexity (i.e. learnability).

Potentially this even improves learnability: `fn box_greeter<G: Greet>(g: G) -> Box<dyn Greet + 'G>` may be easier to explain than the hidden lifetime in `fn box_static_greeter<G: Greet + 'static>(g: G) -> Box<dyn Greet>`.

### Named lifetimes

This name system introduced above (`'Self`, `'T`) is possible since types and lifetimes have [distinct namespaces](https://doc.rust-lang.org/reference/names/namespaces.html), and will rarely cause conflicts since type names usually start uppercase while lifetime names are usually lowercase.

This system is not technically self-documenting, but it is *obvious*.

We do not define lifetime names for other types, e.g. `'String = 'static` or `for<'a> 'Person<'a> = 'a`. This is a minor limitation in that the lifetime of a parameterised type may not be obvious, but it should never be a limitation in practice (except possibly in macro-generated code). Further, `Person` is a type map (an unparameterised type); if we wanted to introduce `'Person` we would need to introduce "lifetime maps", which do not otherwise appear to be useful.

### Alternative syntax

An alternative would be to introduce some new syntax to take the lifetime of a type. This could not be a function (functions cannot return lifetimes) but could perhaps:

- Be a macro in the standard library? — `lifetime!(T)`
- Use some weird new syntax? — `'<T>`

Using a macro allows the possibility of documentation within the standard library (i.e. the feature is self-documenting). But *can* a macro even return a lifetime?

Using some weird new syntax is likely not the right approach: more to learn, more confusion since this is not a self-documenting feature.

### Lifetime elison

It is possible to elide lifetimes in many cases, to the point that they are often simply not required. We could perhaps modify [RFC 599] such that this compiles:
```rust
use core::fmt::Debug;
fn create<V: Debug>(v: V) -> Box<dyn Debug /* infer lifetime of V here */> {
    Box::new(v)
}
```

I am personally against this approach since there are only two mechanisms by which this might work:

1. Infer from the body of the function. This is robust but implies that the type declaration of the method is incomplete (also the case with `impl Trait`, though that is explicit). This also does not work for the trait method `Greet::as_boxed` above.
2. Make a wild guess based on the type parameters. This is fragile (like `#[derive(..)]`) and likely to break some existing code. Further, it begs the question, should trait method `Greet::as_boxed` return `Box<dyn Greet + 'static>` (because this is what you'd see in a free function, which is what this looks like; also because this is the least breaking option) or `Box<dyn Greet + 'Self>` (because effectively `Self` is a type parameter here)?

Further, lifetimes are already confusing enough to learn. I suspect further complicating resolution of the hidden lifetime in `Box<dyn Trait>` will not improve the learning experience.

# Prior art
[prior-art]: #prior-art

-

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Does a crater run with the proposed syntax (`'Self`, `'T`) cause many issues?
- Would the syntax proposed (`'Self`, `'T`) have a significant compilation performance impact?

# Future possibilities
[future-possibilities]: #future-possibilities

*Possibly* ban shadowing of type names with lifetime names in a new edition. (This is not required.)

[RFC 599]: https://github.com/rust-lang/rfcs/blob/master/text/0599-default-object-bound.md
