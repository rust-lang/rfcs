- Feature Name: self life-time
- Start Date: 2020-06-24
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

New 'self named life-time that implicitly bound to life-time of current structure

# Motivation

Motivation is to simplify iterative development and improving refactoring of the code

Sometimes during refactoring such code:
```rust
struct CompositeObject {
    obj: SomeType,
}

struct BigObject {
    composite_obj: CompositeObject,
    count: i32,
}

struct Application {
   big_obj: BigObject,
}
```

developer decides to make obj of SomeType as reference in CompositeObject type:
```rust
struct CompositeObject<'a> {
    obj: &'a SomeType,
}

struct BigObject<'a> {
    composite_obj: CompositeObject<'a>,
    count: i32,
}

struct Application<'a> {
   big_obj: BigObject<'a>,
}
```
Everywhere in composition hierarchy I need to write 'a ... most of the times it is just boilerplate code ...

What if instead of writing manually we will introduce the 'self life-time:
```rust
struct CompositeObject {
    obj: &'self SomeType,
}

struct BigObject {
    composite_obj: CompositeObject,
    count: i32,
}

struct Application {
   big_obj: BigObject,
}
```

Code much simpler and more maintainable than fighting with named life-times in composite hierarchy

Compiler underhood will generate the following code:
```rust
struct CompositeObject<'self> { // 'self is implicit life-time of CompositeObject
    obj: &'self SomeType,
}

struct BigObject<'self> { // 'self is implicit life-time of BigObject
    composite_obj: CompositeObject<'self>, // Assign 'self of BigObject to CompositeObject
    count: i32,
}

struct Application<'self> { // 'self is implicit life-time of Application
   big_obj: BigObject<'self>, // Assign 'self of Application to BigObject
}
```

On user side call should be like this:
```rust
fn make_app(config: &Config) -> Application;
```
or
```rust
fn make_app(config: &Config) -> Application<'_>;
```

# Drawbacks
[drawbacks]: #drawbacks

It could conflict with existing 'self life-time in some crate

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design will help developers to iteravly play with library design, which should increase qualitty of the final library or application

# Prior art
[prior-art]: #prior-art

There was disscutions on this topic in https://internals.rust-lang.org/t/simplification-reference-life-time/12224/20
