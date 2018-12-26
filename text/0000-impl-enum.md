- Feature Name: impl_enum
- Start Date: 2018-12-25
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This is a proposal for a new interpretation of enum types, so they can get stored as their variant types and matched at compile time.
This simplyfies writing efficient generic code when using enums.

In the following this enum will be considered:

```rust
enum Enum {
    Variant1,
    Variant2(...),
    Variant3{...},
    ...
}
```

Now enums implicitly generate structs for each variant like this:

```rust
mod Enum {
    pub struct Variant1;
    pub struct Variant2(...);
    pub struct Variant3{...};
    ...
}
```

That's not the main feature of this proposal, but necessary to get it work.

Additinally, enums generate a type similar to traits for the enum, which are implemented by the structs, and also work in a similar way.
The current enums are the counterpart to trait objects then.

# Motivation
[motivation]: #motivation

Let's begin with an example:

```rust
use recieve_exit_event; // fn() -> bool

trait Updateable<E> {
    fn update(&self, event: E) -> bool;
}

enum Event {
    Idle,
    Exit,
}

struct Object;
impl Updateable<Event> for Object {
    fn update(&self, event: Event) -> bool {
        match event {
            Event::Idle => true,
            Event::Exit => false,
        }
    }
}

fn run() {
    let current_updatable = (&Object as &dyn Updateable<Event>);
    loop {
        if recieve_exit_event() {
            if !current_updatable.update(Event::Exit) {
                break;
            }
        }
        if !current_updatable.update(Event::Idle) {
            break;
        }
    }
}
```

This is a small implementation of an event loop. Inside the loop, new events are generated under certain conditions. Then they are passed to a method, which handles these events.

The event types will probably contain some information important for the events.

The event types are known at compile time, so it would be more efficient to write it like this:

```rust
use recieve_exit_event; // fn() -> bool

trait Updateable<IdleEvent, ExitEvent> {
    fn update_idle(&self, event: IdleEvent) -> bool;
    fn update_exit(&self, event: ExitEvent) -> bool;
}

mod Event {
    pub struct Idle;
    pub struct Exit;
}

use Event::*

struct Object;
impl Updateable<Idle, Exit> for Object {
    fn update_idle(&self, event: Idle) -> bool {
        true
    }
    fn update_exit(&self, event: Exit) -> bool {
        false
    }
}

fn run() {
    let current_updatable = (&Object as &dyn Updateable<IdleEvent, ExitEvent>);
    loop {
        if recieve_exit_event() {
            if !current_updatable.update_exit(Event::Exit) {
                break;
            }
        }
        if !current_updatable.update_idle(Event::Idle) {
            break;
        }
    }
}
```

This new approach will not create enum types just to match them directly afterwards, when it's still known, and removes a little unnecessary overhead.

But this comes with problems. When adding a new event type, it has to be added in many places instead of just once in the enum declaration and once in the `match` expression using the enum. Because it's an enum, the compiler will even warn, if you forget to add some branch, but not if you forget adding a new method, so this small performance benefit is most likely not worth it.

But this means, the helpful abstraction using enums is not a zero cost abstraction in this scenario.

`impl enum` will automatically define the more efficent code without losing the abstractions.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As told in the summary, an enum works like this:

```rust
mod Enum {
    pub struct Variant1;
    pub struct Variant2(...);
    pub struct Variant3{...};
    ...
}
```

There are still a few differences. Since enum struct fields are public by default, the fields of these structs have to be public by default, too.

These variant structs implement the enum, similar to how other types implement traits.

Enum types can just be used like trait types now:

* It's preferred to add a `dyn` modifier, when using an enum directly.
* It's also possible to use enums for generics additional to traits.
* Even using `impl Enum` is possible.

Like for `impl Trait`, using `impl Enum` as a return value will require all possible return values to be of the same enum variant of the specified enum.

## How matching works

Matching variant structs works different to matching normal structs:

```rust
struct Struct1;
struct Struct2(...);

match Struct2(...) {
    Struct1 => ...,
    ...
}

if let Struct2(...) = Struct1 {
    ...
}

while let Struct2(...) = Struct1 {
    ...
}
```

None of these examples will work using normal structs, but variant structs work different.
When two structs implement the same enum, only the matching branch stays. All other branches are removed at compile time.

This already works, when not knowing the exact type, so writing generic functions, specialized on a single type, is easy.

Whereever required, enums are implicitly cast to dynamic enums, for example when calling a function, that takes a dynamic enum. This is required to stay backwards compatible. Casting back to enum variants directly is not possible. Match should be used for that, just as before.

## Enums in traits

Since enums cannot be extended after definition, it's allowed to make traits, which contain generic functions, into objects, in case the generics are required to be enums:

```rust
trait Trait {
    fn test<T: Enum>(&self, arg: Enum);
}
```

The vtable of this trait will contain a new entry for every variant of the enum type. In case of multiple enums as arguments, it will just generate even more entries. Using multiple dynamic enums for traits, that will be made into an object, should be avoided.

It may be useful to define a trait, specialized for multiple different enums. This will look like this:

```rust
trait Trait {
    enum Enum;
    fn test<T: Self::Enum>(&self, arg: T);
}
```

## Example


Using these new feature, writing an efficient version of the motivational example is now possible:

```rust
use recieve_exit_event; // fn() -> bool

// remove generic type from 
trait Updateable {
    // add associated enum
    enum Event;
    // require associated enum
    fn update<E: Event>(&self, event: E) -> bool; 
}

enum Event {
    Idle,
    Exit,
}

struct Object;
impl Updateable for Object {
    // update implementation to match new version of Update
    enum Event = Event;
    fn update<E: Event>(&self, event: E) -> bool {
        match event {
            Event::Idle => true,
            Event::Exit => false,
        }
    }
}

fn run() {
    let current_updatable = (&Object as &dyn Updateable<Event = Event>);
    loop {
        if recieve_exit_event() {
            if !current_updatable.update(Event::Exit) {
                break;
            }
        }
        if !current_updatable.update(Event::Idle) {
            break;
        }
    }
}
```

As it's easy to see, with this feature, there are almost no differences in the implementation, but the overhead from using enums is eliminated.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

It seems the explanation in the previous part is already detailed enough for most parts and moving parts down here will just make it less clear.

## Enums and traits

Enums and Traits can both be used as requirements for generic type arguments in the same way, but only one enum dependency is allowed. Else it may not be clear, how `match` works for this type.

# Drawbacks
[drawbacks]: #drawbacks

It's a big extension and may have little advantages.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

I had different design ideas first, but they were all a bit messy.

## Const enum

The first approach was using `const enum`. This would work just like an enum:

```rust
const enum Enum {...}
```

When defining an enum like this, all types of this enum are just structs, and using the name of the enum type works like an `impl Trait` and implicitly generates specialized versions. The problem is, it's less clear, when specialized versions are generated implicitly.

## Implicitly converting enum types to supertypes everytime

Another idea was to implicitly convert enum variant types to enum types, after creation.

This would ensure backwards compability, too, and would not add some implicit compile time optimizations in case a match is used on the created type.

In this version, the following expressions are just the same:

```rust
let value = EnumName::Variant1;
```
```rust
let value: EnumName = EnumName::Variant1;
```

But it would also be possible to do this, which is the only way to create specialized structs:

```rust
let value: EnumName::Variant1 = EnumName::Variant1;
```

But here it's confusing, why this version is selected, so I decided against.


# Prior art
[prior-art]: #prior-art

I didn't find any information about something like this in rust and also don't know about other languages having such a feature, since it's pretty specific to a current rust feature.
Being able to define multiple method implementations in one function and using compile time evaluation to select a version is also present in [scopes](scopes.rocks) in a powerful way.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

Will implicit casts work in a backwards compatible way?

Should it be allowed to implemnet methods and traits for variant structs?

Are variant types really public types at all?

Is it possible to access struct fileds from variant structs directly?

# Future possibilities
[future-possibilities]: #future-possibilities

When introducing associated enums, associated traits may also be useful, just for completeness. It shouldn't be a big deal, since they work pretty similar to associated enums, and even have less features.

When there are types for the variant struct anyway, it would be nice to add a way to extract the struct type directly instead of just the contents. This would solve another thing, people were already interested in.

It may be possible to add structs to multiple enums at once, but it's difficult to imagine a syntax, that does not forbid extending enums after creation.
