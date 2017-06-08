- Feature Name: Statically dispatched methods for trait objects with associated data
- Start Date: 2015-12-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC describes a way to efficiently implement statically dispatched methods for trait objects.
It allows statically dispatched methods to access implementer's fields directly.
Unlike others approaches it doesn't require implementers to have a specific memory layout.

It provides an alternative solution to one part of "efficient inheritance problem": https://internals.rust-lang.org/t/summary-of-efficient-inheritance-rfcs/494

# Motivation

Currently pretty much all calls of trait objects' methods are dynamically dispatched.
It is possible to statically dispatch methods defined in ```impl Trait {}``` block, but they are limited to calling other trait methods, so it's pretty much useless. In most cases it's actually better to make them dynamic because it will ensure that virtual table is only invoked once.
To make statically dispatched methods useful, we have to allow them to statically access instance data.

Unfortunately, it generally means that all instances must have compatible memory layout. This ties all implementations together and often makes it impossible to implement traits for types that weren't designed for this from the beginning.

This RFC follows 'associated fields' approach. It shows that if we restrict each trait to at most one associated field, then it makes it possible to relieve all layout restrictions on implementers while keeping ability to provide static access to this associated field.

Important properties of proposed solution:

* Doesn't impose any special requirements (like memory layout) on traits or structs
* Works fast in general cases (i.e. never falls back to dynamic dispatch)
* Works even faster in specific cases (i.e. you can't hand code it any better)
* Doesn't make traits redundant

# Detailed design

## Language changes

1. Add ability to define at most one associated data field in trait definition
2. Allow to access associated data in dynamic and static methods of the trait defining it
3. Allow implementers to map one data field to trait's associated data (by referring to ```self```, some field of it or some transmute of them)

## Simple use case

```rust
// trait Trait has associated data
// it can be referenced in default implementations of
// dynamic trait methods and in static trait methods
trait Trait {
    field assoc_data: usize;
    fn dynamic_dispatch(&self) { println!("default dynamic Trait {}", self.assoc_data) }
}
impl Trait {
    fn static_dispatch(&self) { println!("static Trait {}", self.assoc_data); }
}
// note that Foo and Bar have different memory layout
struct Foo {
    leading_data: usize,
    unrelated: bool,
}
struct Bar {
    unrelated: bool,
    trailing_data: usize,
}
// implementations bind associated data to actual locations
// implementation cannot access associated data to prevent aliasing
impl Trait for Foo {
    field assoc_data: self.leading_data;
    fn dynamic_dispatch(&self) { println!("overridden dynamic Foo {}", self.leading_data); }
}
impl Trait for Bar {
    field assoc_data: self.trailing_data;
    // use default implementation of Trait::default_dispatch
}
fn main() {
    let foo = Foo { leading_data: 1, unrelated: false };
    let bar = Bar { unrelated: false, trailing_data: 2 };
    let foobar: &Trait = &foo;
    foobar.dynamic_dispatch(); // prints "overridden dynamic Foo 1"
    foobar.static_dispatch(); // statically dispatched, prints "static Trait 1"
    let foobar: &Trait = &bar;
    foobar.dynamic_dispatch(); // prints "default dynamic Trait 2"
    foobar.static_dispatch(); // statically dispatched, prints "static Trait 2"
}
```

## Implementation

The only way to provide static access to trait fields is to ensure compatible memory layout.
Restricting associated data to at most one field means that the only thing that differs for different implementers is the offset to this associated data.

This offset can, in fact, be taken into account at compile time.

Currently trait object in Rust is defined in this way:
```rust
pub struct TraitObject {
    pub data: *mut (), // pointer to the beginning of the struct
    pub vtable: *mut (), // pointer to the vtable
}
```

If we change this definition for traits with associated data to contain pointer to *beginning of the associated data* instead, it will make it possible to access this data statically:
```rust
pub struct TraitObject {
    pub data: *mut TypeOfAssociatedData, // pointer to beginning of the associated data
    pub vtable: *mut (), // pointer to the vtable
}
```

Offset to the associated data has to be applied during creation of trait object, but is generally known at compile time.

Static methods of the trait will have direct access to associated data. Dynamic methods of trait implementers will be able to use positive and negative offsets to access other data.

## Trait inheritance

Each trait defines it's own associated data. It allows to map trait inheritance hierarchy onto structs aggregation hierarchy.

```rust
// Hierarchy of structs:
// -Vector
//  |-Transform
//  | \-Foo
//  \-Bar
struct Vector(f32, f32);
struct Transform {
    pos: Vector,
    rotation: f32,
}
struct Foo {
    transform: Transform,
}
struct Bar {
    unrelated: bool,
    some_vector: Vector,
}
// Hierarchy of traits:
// -HasPosition
//  \-HasTransform
trait HasPosition {
    field pos: Position;
}
trait HasTransform : HasPosition {
    field transform: Transform;
}
// hierarchies can be mapped onto each other
impl HasPosition for Foo {
    field pos: self.transform.pos;
}
impl HasTransform for Foo {
    field transform: self.transform;
}
impl HasPosition for Bar {
    field pos: self.some_vector;
}
```

In this example, given that no field reordering happens, trait objects created from ```Foo``` instances will contain pointers to ```Foo``` beginning, while trait objects created from ```Bar``` instances will contain pointer to the beginning of ```some_vector``` instead.

# Drawbacks

## Implementation complexity

Even though this proposal is pretty easy from language complexity standpoint, it notably increases complexity of trait objects implementation.
Increased complexity of trait objects, in turn, increases complexity of downcasting/upcasting and might make low-level debugging harder.

## Trait inheritance consistency

Currently inheriting trait has access to all methods of inherited trait.
This is not the case for associated data.

Inheriting trait can define its own associated data with different name and different type. Because of that it's probably better not to inherit any associated data at all.

This is probably not what user expects. Currently traits inherit all methods and associated items of base traits.

## Potential code duplication

Since trait object will point to different parts of the implementer depending on which trait it represents, it makes it hard to implement methods shared by all traits.

See this example:

```rust
trait Foo {
    field a: i8;
    fn act(&self);
}
trait Bar: Foo {
    field b: i8;
}

#[repr(C)]
struct FooBar {
    unrelated: bool,
    a: i8,
    b: i8,
    unrelated2: bool,
}
impl Foo for FooBar {
    field a: self.a;
    fn act(&self) { println!("{} {}", self.unrelated, self.unrelated2); }
}
impl Bar for FooBar {
    field b: self.b;
}

fn main() {
    let inst = FooBar { unrelated: false, a: 1, b: 2 };
    { 
        let fbr: &FooBar = &inst;
        fbr.act(); // fbr is a pointer to FooBar here
    }
    { 
        let foo: &Bar = &inst;
        foo.act(); // foo is a pointer to FooBar.a here (+vtable)
    }
    { 
        let bar: &Bar = &inst;
        bar.act(); // bar is a pointer to FooBar.b here (+vtable)
    }
}
```

There are two ways to solve this problem.

First of all, we can instantiate three separate functions: ```Foo::act```, ```Bar::act``` and ```FooBar::act```.

1. ```FooBar::act``` roughly as ```foo (void* ptr) { print_bool(ptr); print_bool(ptr+3); }```
2. ```Foo::act``` roughly as ```foo (void* ptr) { print_bool(ptr-1); print_bool(ptr+2); }```
3. ```Bar::act``` roughly as ```foo (void* ptr) { print_bool(ptr-2); print_bool(ptr+1); }```

Second, we can downcast in runtime. Only the most common function will be instantiated, ```Foo::act``` in this case. Translator will have to apply appropriate offset at the call site before calling it.

It should be pretty efficient for *struct->trait* downcast, but will be much trickier and most likely inefficient for *trait->trait* downcast. Trait downcasting will probably require virtual table read, which kind of defeats the purpose.

In any case, this only affects traits where associated data is not the first field in the struct. Compiler definitely should try to ensure that associated data is the first in the struct whenever possible.

It might be a good idea to introduce an attribute to ensure that associated data is, in fact, the first field of the struct. Something like ```#[first_field]``` (which was mentioned in https://github.com/rust-lang/rfcs/pull/223):
```rust
trait Foo {
    #[first_field] // requires all implementations to have associated data at the beginning
    field data: i32;
}
struct Bar {
    #[first_field] // ensures that data is at the beginning of the struct
    data: i32,
    unrelated: bool,
}
```

# Alternatives

There are plenty of alternative solutions proposed.

Rust issue: https://github.com/rust-lang/rfcs/issues/349

Discussion thread: https://internals.rust-lang.org/t/summary-of-efficient-inheritance-rfcs/494

Most important ones:

1. Enum based solutions: https://github.com/rust-lang/rfcs/pull/11 and https://github.com/rust-lang/rfcs/pull/142
2. Struct inheritance solutions: https://github.com/rust-lang/rfcs/pull/5 and https://github.com/rust-lang/rfcs/pull/9
3. Associated field solutions: https://github.com/rust-lang/rfcs/pull/223 and https://github.com/rust-lang/rfcs/pull/250

This particular RFC belongs to 'associated field solutions' group. The main difference is that this solutions satisfies several desirable properties at the same time. Please see *important properties* in *Motivation* chapter.

# Unresolved questions

This RFC focuses on efficient statically dispatched field access. For best user experience it should be paired with another language feature providing ergonomic field access without any implementation-induced restrictions. It can be a separate set of dynamic associated fields or, even better, some sort of properties. Internally it can be implemented as a syntactic sugar for getters/setters.

Properties are particularly interesting because they are much more flexible than simple data binding. In Rust property can be defined with four functions: a getter returning value, a getter returning reference, a getter returning mutable reference and a setter accepting value. Property doesn't have to provide all of them. It can easily be read-only or write-only.

This RFC does not propose any particular syntax for properties, but here's an example to illustrate the point:
```rust
trait Foo {
    field data: i32; // associated data, only one allowed, not inherited, static access
    property visible: bool; // syntax sugar for getters/setter, multiple properties allowed, inherited, dynamic access (still efficient because of monomorphisation)
    // property can implement only some of getters/setter
    property(get) valid: bool; // this one only implements value getter
}
struct Bar {
    data: i32,
    visible: bool,
}
impl Foo for Bar {
    field data: self.data;
    property visible: self.visible; // defines all 4 functions, getters share implementation
    property(get) valid: { self.visible && self.data == 0 } // custom getter
}
```

Properties make function calls implicit, which is a double-edged sword and might be undesirable for Rust. If so, then dynamic associated fields will do just fine.

Please note that neither ```field``` nor ```property``` have to be keywords. They can only be encountered in ```trait {}``` block or in ```impl for {}``` block. Neither of these blocks can contain stray identifiers. Therefore, it's semantically unambiguous to have identifiers with the same name.

It's also unclear which visibility should be applied to associated data. This RFC implies that it should only be visible from trait's static methods and default method implementations. Alternatively it can follow usual visibility rules, i.e. always be accessible from the same module and publicly accessible if prefixed with ```pub```.
