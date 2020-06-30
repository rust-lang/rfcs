- Feature Name: Hierarchic anonymous life-time
- Start Date: 2020-06-24
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

New use of anonymous life-time `'_` that implicitly added to current structure.

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

What if instead of writing manually we will specify reference fields with anonymous life-time:
```rust
struct CompositeObject {
    obj: &'_ SomeType,
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
struct CompositeObject<'anon> {             // 'anon is implicitly added life-time
    obj: &'anon SomeType,
}

struct BigObject<'anon> {                   // 'anon is implicitly added life-time
    composite_obj: CompositeObject<'anon>,  // 'anon is implicitly used here
    count: i32,
}

struct Application<'anon> {                 // 'anon is implicitly added life-time
   big_obj: BigObject<'anon>,               // 'anon is implicitly used here
}
```

Take a look at example with multiple anonymose life-times:
```rust
struct CompositeObject {
    obj0: &'_ SomeType,
    obj1: &'_ SomeType,
}

struct BigObject {
    composite_obj: CompositeObject,
    count: i32,
}

struct Application {
   big_obj: BigObject,
}
```
code will be translated to:
```rust
struct CompositeObject<'anon0, 'anon1> {              // 'anon0 and 'anon1 are implicitly added life-times
    obj0: &'anon0 SomeType,
    obj1: &'anon1 SomeType,
}

struct BigObject<'anon0, 'anon1> {                    // 'anon is implicitly added life-time
    composite_obj: CompositeObject<'anon0, 'anon1>,   // 'anon is implicitly used here
    count: i32,
}

struct Application<'anon0, 'anon1> {                  // 'anon is implicitly added life-time
   big_obj: BigObject<'anon0, 'anon1>,                // 'anon is implicitly used here
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

Not known at the current time

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design will help developers to iteravly play with library design, which should increase qualitty of the final library or application

# Prior art
[prior-art]: #prior-art

There was disscutions on this topic in https://internals.rust-lang.org/t/simplification-reference-life-time/12224/20
