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
struct City {
    name: String,
}

struct State {
    city: Vec<City>,
    covid_deaths: u32,
}

struct Country {
   state: Vec<State>,
}
```

developer decides to make an inner item a reference:
```rust
struct City<'a> {
    name: &'a str,
}

struct State<'a> {
    city: Vec<City<'a>>,
    covid_deaths: u32,
}

struct Country<'a> {
   state: Vec<State<'a>>,
}
```
Everywhere in composition hierarchy I need to write 'a ... most of the times it is just boilerplate code ...

What if instead of writing manually we will specify reference fields with anonymous life-time:
```rust
struct City& {
    name: &str,
}

struct State& {
    cities: Vec<City&>,
    covid_deaths: u32,
}

struct Country& {
   state: Vec<State&>,
}
```

With this solution developer could just declar with `<type_name>["&"]` name that this structure could have used some references, just be attentive
Developer just could anons that this structure will use references some times without even using references inside:
```rust
struct City& {
    name: String,
}

struct State& {
    cities: Vec<City&>,
    covid_deaths: u32,
}

struct Country& {
   state: Vec<State&>,
}
```

Compiler underhood will generate the following code:
```rust
struct City&<'anon> {                    // 'anon is implicitly added life-time
    obj: &'anon str,
}

struct State&<'anon> {                   // 'anon is implicitly added life-time
    composite_obj: Vec<City&<'anon>>,    // 'anon is implicitly used here
    covid_deaths: i32,
}

struct Country&<'anon> {                 // 'anon is implicitly added life-time
   state: Vec<State&>,                  // 'anon is implicitly used here
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
