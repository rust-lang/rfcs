- Feature Name: `cfg_attribute_in_tuple_type`
- Start Date: 2023-11-23
- RFC PR: [rust-lang/rfcs#3532](https://github.com/rust-lang/rfcs/pull/3532)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Let's make it more elegant to conditionally compile tuple type declarations and pattern matches by allowing cfg-attributes directly on their elements.

# Motivation
[motivation]: #motivation

### Consistency

Currently, there is limited support for conditionally compiling tuple type declarations:

```rust
type ConditionalTuple = (u32, i32, #[cfg(feature = "foo")] u8);
```

```
error: expected type, found `#`
 --> <source>:1:36
  |
1 | type ConditionalTuple = (u32, i32, #[cfg(feature = "foo")] u8);
  |                                    ^ expected type
```

As with [RFC #3399](https://rust-lang.github.io/rfcs/3399-cfg-attribute-in-where.html), some workarounds exist, but they can result in combinatorial boilerplate:

```rust
// GOAL: 
// type ConditionalTuple = (
//     u32, 
//     i32, 
//     #[cfg(feature = "foo")] u8,
//     #[cfg(feature = "bar")] i8,
// );

// CURRENT:
#[cfg(all(feature = "foo", feature = "bar"))]
type ConditionalTuple = (u32, i32, u8, i8);
#[cfg(all(feature = "foo", not(feature = "bar")))]
type ConditionalTuple = (u32, i32, u8);
#[cfg(all(not(feature = "foo"), feature = "bar"))]
type ConditionalTuple = (u32, i32, i8);
#[cfg(all(not(feature = "foo"), not(feature = "bar")))]
type ConditionalTuple = (u32, i32);
```

Rust already supports per-element cfg-attributes in tuple *initialization*. The following is legal Rust code and functions as expected, even though the resulting type of `x` can't be expressed very easily:

```rust
pub fn main() {
    let x = (1u32, 4i32, #[cfg(all())] 23u8);
    println!("{}", x.2) // Output: 23
}
```

Similarly, cfg-attributes are permitted on types in tuple structs, like so:

```rust
pub struct SomeStruct(u32, #[cfg(feature = "foo")] bool);
```

So it makes sense to support it in regular tuple type declaration as well.

### Use Cases

While structs support cfg-attributes on their members, tuples serve an important purpose in a number of applications that can't easily be replicated with structs. One common example is for achieving variadic-like behavior for constructing and accessing struct-of-array (SoA) data structures. These data structures break large data blocks into modular data components in individual contiguous memory blocks for reusable composition and optimizations via SIMD and improved cache behavior. This is especially prevalent in Entity Component System libraries like [bevy](https://docs.rs/bevy_ecs) and [hecs](https://docs.rs/hecs). For example, to perform a world query in hecs, the user constructs an iterator using a type-tuple like so:

```rust
for (id, (number, &flag)) in world.query_mut::<(&mut i32, &bool)>() {
  if flag { *number *= 2; }
}
```

Tuples have a number of unique advantages in this paradigm. For one, they avoid boilerplate due to their ability to be anonymously constructed on the fly. Additionally, tuples can be concatenated and joined (e.g. via the [tuple](https://docs.rs/tuple) crate). This allows more advanced ECS libraries and other similar tools to provide support for pre-determined bundles of components, or use tuple nesting to group logic and functionality. One could theoretically define functions like `query_mut1<T0>`, `query_mut2<T0, T1>`, and so on, but the ergonomics of the tuple approach win out in practice.

In this situation, cfg-attributes come into play when building ECS archetypes (a pre-determined collection of components for a type of entity) for different platforms or deployment targets. Say for example that we were creating a multiplayer asteroids game in an Entity Component System. If we wanted to statically define our archetypes at compile-time (as is the case in [gecs](https://docs.rs/gecs)), it might look something like this:

```rust
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    SpriteComponent,
    AudioComponent,
)>;
```

Since this is a multiplayer game, we may want some components to exist solely on the server or on the client, both for security reasons and also for optimization or performance reasons. The sprite and audio components for example serve no purpose on the server as the server does not render graphics or play audio. In games in other languages, it is common practice to use conditional compilation to avoid putting code in various build targets that serve no purpose, waste resources, or potentially leak information to cheaters. So in this case we will restrict these two components to the `client` feature, like so:

```rust
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    #[cfg(feature = "client")] SpriteComponent,
    #[cfg(feature = "client")] AudioComponent,
)>;
```

Additionally, we need some components to handle serializing the network state, performing dead reckoning, and sending that information to the client from the server. So we will add a `StateStorageComponent` and a `DeltaCompressionComponent`, and restrict those to the server, since the client does not perform these calculations and we want to avoid giving clients this information in order to help confound cheaters.

```rust
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    #[cfg(feature = "client")] SpriteComponent,
    #[cfg(feature = "client")] AudioComponent,
    #[cfg(feature = "server")] StateStorageComponent,
    #[cfg(feature = "server")] DeltaCompressionComponent,
)>;
```

Finally, we want some debug information for diagnosing physics and damage calculation issues. We build a component to store this intermediate data, but we don't want to ship it in the final game because it's just for aid in development. We'll create a `DebugDrawComponent` and add it to our ship archetype as well, but only when the game is built in editor mode because it's quite expensive to do these extra calculations and draw debug information every frame.

```rust
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    #[cfg(feature = "client")] SpriteComponent,
    #[cfg(feature = "client")] AudioComponent,
    #[cfg(feature = "server")] StateStorageComponent,
    #[cfg(feature = "server")] DeltaCompressionComponent,
    #[cfg(feature = "editor")] DebugDrawComponent,
)>;
```

This represents our archetype with the various common and situational components based on its build and deployment target. With this decoration each component is decorated with the context in which it appears, and requires no inference or indirection via macros to generate or read. By comparison, here is how this would be written in Rust today, keeping in mind that a build could be any combination of client, server, and editor for development and debugging purposes (akin to Unreal Engine's "play in editor" feature):

```rust
#[cfg(all(feature = "client", feature = "server", feature = "editor"))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    SpriteComponent,
    AudioComponent,
    StateStorageComponent,
    DeltaCompressionComponent,
    DebugDrawComponent,
)>;

#[cfg(all(not(feature = "client"), feature = "server", feature = "editor"))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    StateStorageComponent,
    DeltaCompressionComponent,
    DebugDrawComponent,
)>;

#[cfg(all(feature = "client", not(feature = "server"), feature = "editor"))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    SpriteComponent,
    AudioComponent,
    DebugDrawComponent,
)>;

#[cfg(all(not(feature = "client"), not(feature = "server"), feature = "editor"))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    DebugDrawComponent,
)>;

#[cfg(all(feature = "client", feature = "server", not(feature = "editor")))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    SpriteComponent,
    AudioComponent,
    StateStorageComponent,
    DeltaCompressionComponent,
)>;

#[cfg(all(not(feature = "client"), feature = "server", not(feature = "editor")))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    StateStorageComponent,
    DeltaCompressionComponent,
)>;

#[cfg(all(feature = "client", not(feature = "server"), not(feature = "editor")))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
    SpriteComponent,
    AudioComponent,
)>;

#[cfg(all(not(feature = "client"), not(feature = "server"), not(feature = "editor")))]
type ShipArchetype = EcsArchetype<(
    TransformComponent,
    VelocityComponent,
    PhysicsComponent,
    ColliderComponent,
    EngineComponent,
    HealthComponent,
    WeaponComponent,
    EnergyComponent,
)>;
```

This would likely need to be generated via macro in practice, and the macro itself would have to parse the cfg-attributes to produce these combinatorial outputs. However, macros aren't an easy fix in all positions where tuples are supported (e.g. as type arguments), and so even with macros this would create levels of indirection and require alias definitions. The hecs query example above could not easily have an element conditionally gated via a macro without first declaring an alias for that query's tuple type outside of the position where the query iteration occurs. This is because doing so would likely require the macro to be able to generate code outside of its immediate context to function (i.e. to branch based on each cfg-attribute involved).

In addition to supporting cfg-attributes in tuple declarations, this RFC proposes supporting these attributes in pattern matching as a proper counterpart. For example:

```rust
let (a, #[cfg(something)] b) = my_tuple;
```
or
```rust
struct MyStruct(i32, #[cfg(something)] u32); // Already supported

fn foo(x: MyStruct) {
    let MyStruct(a, #[cfg(something)] b) = x; // Not yet supported
}
```
or
```rust
match my_tuple {
    (val, #[cfg(something)] Some(other)) => { ... },
}
```

To continue the client-server analogy above, in a hecs-like ECS query, it is useful to conditionally define components on a client- or server-only basis. For example, if the server needed to run additional logic and store additional state when an object updates its movement, one might accomplish this like so:

```rust
type MovementQuery = (Position, Rotation, #[cfg(server)] Authority);

fn do_update(ecs: &mut Ecs) {
    for components in ecs.query::<MovementQuery>().iter_mut() {
        let (position, rotation, #[cfg(server)] authority) = components;
        let _output = update_movement(position, rotation);
        
        #[cfg(server)]
        update_authority(authority, position, rotation, _output);
    }
}
```

Permitting cfg attributes in both the tuple definition, and in the pattern matching to extract tuple elements, allows code like this to conditionally branch without the risk of combinatorial explosion as above.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Tuple type declarations can use cfg-attributes on individual elements, like so:

```rust
type MyTuple = (
    SomeTypeA,
    #[cfg(something_a)] SomeTypeB,
    #[cfg(something_b)] SomeTypeC,
)
```

and in other situations where tuple types are declared, such as in function arguments. These will conditionally include or exclude the type in that tuple (affecting the tuple's length) based on the compile-time evaluation result of each `#[cfg]` predicate.

Similarly, cfg-attributes can be used in pattern matching, like so:

```rust
let (a, #[cfg(something)] b) = my_tuple;
```
and
```rust
match my_tuple {
    (val, #[cfg(something)] Some(other)) => { ... },
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes changing the syntax of the `TupleType` (See [[type.tuple.syntax]](https://doc.rust-lang.org/stable/reference/types/tuple.html#r-type.tuple.syntax)) to include `OuterAttribute*` before each occurrence of `Type`. These attributes can decorate each individual type (up to the comma or closing paren). Similarly, for pattern matching, this RFC proposes adding `OuterAttribute*` before each `Pattern` in `TuplePattern` (See [[patterns.tuple.syntax]](https://doc.rust-lang.org/stable/reference/patterns.html#r-patterns.tuple.syntax)) and `TupleStructPattern` (See [[patterns.tuple-struct.syntax]](https://doc.rust-lang.org/stable/reference/patterns.html#r-patterns.tuple-struct.syntax)). This would work similarly to the `OuterAttribute*` in `StructPatternField` (See [[patterns.struct.syntax]](https://doc.rust-lang.org/stable/reference/patterns.html#r-patterns.struct.syntax)),

In practice, at least within the scope of this RFC, only cfg-attributes need to be supported in these new `OuterAttribute`s.

# Drawbacks
[drawbacks]: #drawbacks

As with any feature, this adds complication to the language and grammar. Conditionally compiling tuple type elements can be a semver breaking change, but not any more than with the already existing workarounds.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

(See [RFC #3399](https://rust-lang.github.io/rfcs/3399-cfg-attribute-in-where.html) for a similar write-up.)

The need for conditionally compiling tuple types can arise in applications with different deployment targets or that want to release builds with different sets of functionality (e.g. client, server, editor, demo, etc.). It would be useful to support cfg-attributes directly here without requiring workarounds to achieve this functionality. Macros, proc macros, and so on are also ways to conditionally compile tuple types, but these also introduce at least one level of obfuscation from the core goal and can't be used everywhere a tuple can be. Finally, tuples can be wholly duplicated under different cfg-attributes, but this scales poorly with both the size and intricacy of the tuple and the number of interacting attributes (which may grow combinatorically), and can introduce a maintenance burden from repeated code.

It also makes sense in this instance to support cfg-attributes here because they are already supported in this manner for tuple initialization and for tuple struct declaration, as well as for individual fields in pattern matching on structs.

# Prior art
[prior-art]: #prior-art

I'm not aware of any prior work in adding this to the language. Other forms of this kind of cfg-attribute support exist elsewhere in
the language. For example in tuple structs (illustrated above), and in pattern matching on non-tuple structs, where matched fields can already be conditionally gated, like so:

```rust
struct MyStruct {
    a: u32,
    #[cfg(true)]
    b: u32,
}

fn foo(x: MyStruct) {
    let MyStruct{
        a,
        #[cfg(true)]
        b,
    } = x;
}
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are currently no unresolved questions for this RFC.

# Future possibilities
[future-possibilities]: #future-possibilities

I believe this change is relatively self-contained, though I also think it's worth continuing to look for additional places where support for cfg-attributes makes sense to add. Conditional compilation is very important, especially in some domains, and requiring workarounds and additional boilerplate to support it is not ideal. A more detailed enumeration of inconsistencies with cfg-attributes and comma-terminated fragments can be found [on this HackMD page](https://hackmd.io/@recatek/S1NO5ZXHT). If this RFC is accepted, only two reasonable use cases for cfg-attributes on comma-terminated fragments would remain uncovered.
