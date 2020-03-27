- Feature Name: dyn-assoc
- Start Date: 2020-03-19
- [RFC PR](https://github.com/rust-lang/rfcs/pull/2886 "GitHub pull request for this RFC")
- Rust Issue: N/A

# Summary
[summary]: #summary

This RFC proposes a relaxation of the trait object safety rules. More specifically, it specifies a minor syntax addition for accessing associated functions and constants from a trait object, as long as the size of the results is known at compile time (achieved by using `Box<dyn Trait>` instead of `Self` in associated functions and methods).

# Motivation
[motivation]: #motivation

The most basic use case is retreiving trait implementation specifics for values which are not `Copy` and are borrowed mutably elsewhere when the query occurs. For a sensible example, consider the [`num-traits` crate](https://crates.io/crates/num-traits "num-traits on Crates.io"), more specifically, the trait methods which retrieve constants, as in [the `Num` trait](https://docs.rs/num-traits/*/num_traits/trait.Num.html "Trait num_traits::Num"). Obviously, the `Num` trait cannot be made into a trait object in present Rust. If having dynamically-dispatched numbers feels like an overkill asking for compile-time generics, imagine custom numeric types which have a considerably bigger size than a pointer with different possible implementations. A port of [BigBit](https://github.com/amitguptagwl/bigbit "BigBit standard for numbers of arbitrary size and compact Unicode strings") is one example of such types.

Another use case is making "object safe trait constructors" possible. For this one, consider a trait for backends of a system, and a (global, non-associated) function which chooses the right backend based on certain factors and returns a (heap-allocated, i.e. boxed/`Rc`-ed) trait object with a bound to the backend trait. The backend trait would then have a trait constructor and methods for the backend instance. This RFC proposes the possibility of having the constructor and the methods in a single trait (remember that in present Rust, having a non-object-safe trait as a supertrait makes the subtrait also non-object-safe). The constructors, however, cannot possibly be called from a trait object in this case — the backend creation function would use `match` (or any other form of branching) to select the backend and then call the constructor on a concrete type, with the constructor being an implemented associated function of a trait. The point of having the constructor as an associated function in a trait allows for a clearer description of intent.

A more interesting use case is having bounds to traits like `Clone` in trait objects. While not useful on their own, these can bring trait objects closer to compile-time generics, which tend to have multiple trait bounds to support just enough functionality to perform the required tasks. This requires the implementation of [this proposal](https://github.com/rust-lang/rfcs/issues/2035 "Trait objects for multiple traits — RFC issue").

Functions logically related to a trait but not immediately requiring an instance of a type implementing said trait are also a potential use case. **Certain other use cases are mentioned in further sections.**

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Traits with associated functions can be made into trait objects. The associated functions can then be accessed using method syntax and therefore are subject to deref coercion, meaning that this will work:
```rust
// In present Rust, this trait is not object-safe, something that this RFC changes.
trait ThingDoer {
    fn do_a_thing();
}
struct HWThingDoer;
impl ThingDoer for HWThingDoer {
    fn do_a_thing() {
        println!("Did a thing using the GPU at insane speed!");
    }
}
struct DefaultThingDoer;
impl ThingDoer for DefaultThingDoer {
    fn do_a_thing() {
        println!("Did a thing entirely on the CPU.");
    }
}

fn main() {
    let mut thingdoer: Box<dyn ThingDoer> = Box::from(HWThingDoer);
    // Even though do_a_thing() does not take &self, thingdoer is used to dynamically dispatch the concrete implementation, which is why method syntax is used.
    thingdoer.do_a_thing();
    thingdoer = Box::from(DefaultThingDoer);
    thingdoer.do_a_thing();
}
```
This can be used to retrieve `'static` state of a trait implementation without the need to immutably borrow the trait object used to locate its underlying type, which becomes a clarity problem if it's already mutably borrowed in the same scope. One good example of this kind of usage is having a backend trait optionally support certain extensions, the support for which would be determined by runtime factors (or in an even more complex case by both compile time and runtime factors). This makes things easier when working with OpenGL or Vulkan, for example (or any other Khronos API, really, they seem to a certain API design style), because these have the concept of hardware extensions, some of which are then included into the core specification and can optionally be used by applications if they are developed for a lower version of the API but still wish to have access to the features on newer hardware.

## Mentions of the implementing type in associated functions
[mentions-of-the-implementing-type-in-assoc-fn-guide]: #mentions-of-the-implementing-type-in-associated-functions

A much more frequent use case of associated functions is for adding constructors, i.e. returning `Self`. But because `Self` in the case of trait objects is unsized, constructors in traits should return some kind of wrapper which makes these trait objects sized, making it possible to store the result on the stack. The best candidate for this job is `Box`, since it's the simplest way of allocating something on the heap from safe code, which is subject to a zero-cost conversion into a reference counter. `Box` also exposes an interface for casting trait objects down to a concrete type, which can be used to store the object entirely on the stack if the selection of possible types is known at development time (eg. if the trait and its implementors are `pub(crate)`).

To be a valid trait object, the return type of these functions should be `Box<dyn Trait>` instead of `Box<Self>`, since `Self` in trait refers to the **concrete type implementing the trait** instead of the trait as a trait object.

## Associated constants
Associated constants work in a similar way:
```rust
trait PreciseThing {
    const PRECISION: usize;
}
impl PreciseThing for f32 {
    const PRECISION = 23;
}
impl PreciseThing for f64 {
    const PRECISION = 53;
}

fn main() {
    let mut precisething: Box<dyn PreciseThing> = Box::from(0.0_f32); // f32 has a 23-bit mantissa.
    println!("Our precise thing has {} bits of precision", precisething.PRECISION);
    precisething = Box::from(0.0_f64); // ...and f64 has 53 bits in its mantissa.
    println!("And now it has {} bits", precisething.PRECISION);
}
```
This can be used by library authors to selectively accept trait objects if certain constants match certain criteria, as well as for introspecting implementation details at runtime, as shown with the floating-point mantissa example.

## Generic contexts
[generic-contexts]: #generic-contexts
There's only one case when we don't have direct access to the trait object: compile-time generics. This example demonstrates the issue:
```rust
trait Foo {
    const CONST: usize;
    
    fn assoc();
}
// Long story short, we're taking a generic argument
// without a vtable, but we need to do dynamic dispatch.
fn foo<T: Foo + ?Sized>() {
    let _ = T::CONST;
    T::assoc();
}

fn main() {
    foo::<dyn Foo>();
}
```
We need to somehow provide `foo<T>()` with a vtable for the trait object, but we don't have one. With the `trait_object.associated_function()` syntax (or the alternatives described in the [unresolved questions][unresolved-questions] section), there's the trait object which we use to perform dynamic dispatch in the calling function — we're calling associated functions and are looking up constants from the **trait** of the trait object, **having the trait object available to also be used to have trait methods called**. But in this case, we don't have a trait object to start with. As a result, if we're calling a function/method/whatever with generic arguments **and the callee uses the associated members** and are passing in `dyn Foo`, the compiler emits an error: "no trait object provided to perform dynamic dispatch", highlighting the exact associated function which requires a trait object with "trait object required here" like some other errors do.

This issue also exists when using the generics on types:
```rust
trait Foo {
    const MY_NAME: &'static str;
}
struct MyBox<T: Foo + ?Sized>(Box<T>);

impl<T: Foo + ?Sized> MyBox<T> {
    fn get_name() -> &'static str {
        T::MY_NAME
    }
}
```
In this case, the compiler **detects that the type is using the associated members of `T`** and, if we then try to give `dyn Foo` to it, produces the a similar error: "cannot use dyn Trait as a generic parameter because the type requires a trait object to (call associated function|use associated constant)", with a similar highlight as above.

If we have a trait object, however, and we want to use compile-time generics, we can use it directly as the generic parameter:
```rust
trait Foo {
    const CONST: usize;
    
    fn assoc();
}
// Long story short, we're taking a generic argument
// without a vtable, but we need to do dynamic dispatch.
fn foo<T: Foo + ?Sized>() {
    let _ = T::CONST;
    T::assoc();
}

struct DefaultFoo;'
impl Foo for DefaultFoo {
    const CONST: usize = 42;
    
    fn assoc() {
        println!("I'm doing great, thank you!");
    }
}

fn main() {
    // We'd have a constructor here if the implementor wasn't a unit type:
    let tobject: Box<dyn Foo> = Box::from(DefaultFoo);
    foo::<tobject>();
}
```
Here, we're saying to the compiler: "take this trait object and use it to resolve all usage of associated constants/functions in this function I'm calling". Same with types with generic arguments.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The main point of technical discussion around this feature is how would dynamic dispatch work if there's no `&self`. Consider the first example from the [guide-level section][guide-level-explanation]:
```rust
trait ThingDoer {
    fn do_a_thing();
}
struct HWThingDoer;
impl ThingDoer for HWThingDoer {
    fn do_a_thing() {
        println!("Did a thing using the GPU at insane speed!");
    }
}
struct DefaultThingDoer;
impl ThingDoer for DefaultThingDoer {
    fn do_a_thing() {
        println!("Did a thing entirely on the CPU.");
    }
}

fn main() {
    let mut thingdoer: Box<dyn ThingDoer> = Box::from(HWThingDoer);
    thingdoer.do_a_thing();
    thingdoer = Box::from(DefaultThingDoer);
    thingdoer.do_a_thing();
}
```
What's going on here, exactly? How does the program find the implementation if we don't have access to `&self`? **We actually do.** Since dynamic dispatch happens in `main` (the calling function), rather than a stub `do_a_thing` "disambiguator" which reads the trait object and redirects execution according to its vtable. Simply put, the entire technical idea of this RFC is *if we have access to the trait object when we're doing dynamic dispatch, why do we pretend that the dynamically dispatched implementation needs `&self` in order to understand what to do when it actually doesn't?*

## Associated constants
It's not any harder for associated constants, since these can be simply stored in the vtable along with the methods and associated functions. The only reason why this might become an implementation issue is that the vtable lookup code can no longer assume that all elements have the same size alignment. It's of high doubt that this assumption is ever made anywhere inside the compiler, though.

## Mentions of the `Self` type in methods and associated functions
As [explained back in the guide section][mentions-of-the-implementing-type-in-assoc-fn-guide], `Self` as a concrete type is still not allowed in traits if these wish to stay object-safe. Trait objects of a trait in the declaration of that exact same trait are perfectly fine, and even [do compile in present Rust](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=45c35ab9274c22b323a9016796c635a5):
```rust
trait Thing {
    fn make_a_friend(&self) -> Box<dyn Thing>;
}
struct SimpleThing;
impl Thing for SimpleThing {
    fn make_a_friend(&self) -> Box<dyn Thing> {
        Box::from(SimpleThing)
    }
}
struct BetterThing;
impl Thing for BetterThing {
    fn make_a_friend(&self) -> Box<dyn Thing> {
        Box::from(BetterThing)
    }
}

fn main() {
    let sithing: Box<dyn Thing> = Box::from(SimpleThing);
    let bething: Box<dyn Thing> = Box::from(BetterThing);
    let sithing2 = sithing.make_a_friend();
    let bething2 = sithing.make_a_friend();
}
```
Supporting the actual `Self` type [might still be introduced in a later RFC][future-possibilities].

## Generic contexts
As [described][generic-contexts] in the guide-level explanation, we can put trait objects themselves in the generics to get associated members to work in functions which do not take the trait object as an argument and are not dynamically dispatched from the side which has the trait object. How does this work?

For functions with generic arguments, the vtable (not the trait object, only its vtable) is passed as an invisible argument on the stack/registers, which is present only in the morphomization of the function which takes a trait object in its generics. If a function has more than one `?Sized` generic argument, the morphomization (generated at compile time) which requires multiple trait objects should take multiple invisible vtable arguments. The arguments are automatically filled out when the callee puts  the trait object(s) into the angle brackets (if they don't provide all the trait objects required and fill some with `dyn Trait`, an error is produced as described back in the guide-level section).

For types with generic arguments, the vtables exist inside instances of that type as invisible private fields. Like the vtable function arguments, these are only visible to the compiler (but they obviously contribute to the size, which remains known since vtable pointers are just pointers). If an associated function is called, however (no `self` — no access to the invisible vtable fields), the vtables should be provided as invisible vtable arguments, as with regular functions.

## Edge cases
None unresolved yet.

# Drawbacks
[drawbacks]: #drawbacks

None known yet.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The constantly mentioned alternative is declaring the associated functions in traits which are meant to be trait objects with a `&self` parameter, effectively converting all associated functions into trait methods. This becomes a major source of confusion if the trait object is mutably borrowed elsewhere while this "`&self`ified" method is called. This introduces lots of unnecessary artificial curly-bracket scopes which hurt readability in most cases.

That workaround is also wrong from a semantic standpoint, since trait methods by design should use the fields of the trait object in some way, either directly or indirectly. Again, an unused `&self` just to combat language limitations introduces confusion.

Last but not least, "`&self`ifying" works terribly when the trait and the code creating and using trait objects of it are in different crates made by different people. In this case, the library developer has to add `&self` to the associated trait functions, hurting the code using the trait without trait object, which loses the ability to use its associated functions without an actual object of that trait. As a result, library developers would be forced to have the associated functions in their struct's main `impl` block and "`&self`ified" wrappers around these in trait implementations. This is incomprehensibly hacky and non-idiomatic, seriously hurting library design in the long run but inapparent in the beginning of development.

# Prior art
[prior-art]: #prior-art

C++ also lacks this feature and has adopted the [virtual static idiom](https://wiki.c2.com/?VirtualStaticIdiom "Virtual Static Idiom") for this purpose. C++, however, allows reference/pointer aliasing, while in Rust safe code passing `&self` to associated functions might cause issues if the trait object is mutably borrowed when the associated function is called (in that case, static analysis behaves more conservatively than it should, since associated functions with `&self` slapped over them do not use `self` and therefore don't care if it's already mutably borrowed).

Java also works around this in a [hacky way](https://stackoverflow.com/a/8095482/7616532 "Why do we say that a static method in Java is not a virtual method? — Stack Overflow").

Delphi [seems to have](https://stackoverflow.com/a/248348/7616532 "Answer to a C# question on this topic") this sort of functionality, effectively replacing the factory pattern with this more readable and convenient idea.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## What would the syntax look like?
The only major unresolved question about this feature is the syntax. Currently, this document only mentions the idea chosen by the Rust forum: method/field syntax for associated functions and constants, respectively:
```rust
trait_object.CONSTANT
trait_object.associated_function()
function_with_generics::<trait_object>()
let gt: GenericType<trait_object>
```
While this cannot ever have the risk of trait methods and associated functions colliding their names (since these reside in the same namespace), this can confuse Rust beginners, since method syntax usually means that the function called somehow directly **operates** with the trait object. This wouldn't be a problem for those coming from a C++ background, where static methods on classes can be called on the instances using method syntax, even though the static function does not receive the `this` pointer.

The two alternative syntax constructs are:
```rust
// #1
trait_object::Self::CONSTANT
trait_object::Self::associated_function()
function_with_generics::<trait_object::Self>()
let gt: GenericType<trait_object::Self>

// #2, author's personal favorite
trait_object::impl::CONSTANT
trait_object::impl::associated_function()
function_with_generics::<trait_object::impl>()
let gt: GenericType<trait_object::impl>
```
Construct #1 also might seem confusing, since it's merely an interface towards the associated members of the trait implementation of the trait object rather than the actual type itself. Still, the syntax described in the above sections and alternative #1 are still left as possibilites, even though #2 is more likely to be the stabilized way.

# Future possibilities
[future-possibilities]: #future-possibilities

## Compatibility with other RFCs and editions
None of the syntax constructs described in the above sections seem to conflict with existing valid syntax, meaning that this RFC can even be implemented in the 2018 edition, let alone the proposed 2021 edition.

Conflicts with other RFCs' syntax additions are also not anticipated.

## Resulting new RFCs
Storing the actual size in the vtable and putting it on the stack to store the trait object without pointer indirection by adding and subtracting to navigate the stack at runtime instead of relying on fixed offsets from the stack base pointer should by itself be in its own, much more complex RFC (since the concept of `Sized` would no longer just mean that the size of a type is known at compile time), which might need to go in a future edition, since such a change to `Sized` semantics might be breaking. This will allow library developers to use `Self` instead of `Box<dyn Trait>` in traits, removing pointer overhead when compile-time generics are used.
