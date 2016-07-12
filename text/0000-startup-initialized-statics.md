- Start Date: 2016-07-12
- RFC PR #: 
- Rust Issue #: 

# Summary

Introduce the ability to initialize (i.e. mutate) static items (even non-mut ones) at the beginning of main in a compiler-guaranteed safe manner.

# Motivation

Even though mutable statics can be initialized safely at runtime in a couple of ways, they have drawbacks:

- By using the lazy initialization idiom. Drawback: incurs a small performance overhead by having to check that the item is initialized every time it is used.
- By using the RAII idiom in the form of initialization guards, where the existence of an initialization guard variable is a guarantee that the initialization has happened. Drawback: very poor ergonomics.

Instead, I propose a safe runtime initialization system for statics which causes no overhead to access-times and is also ergonomic.

# Detailed design

## Language changes

I propose the following two additions:

### 1. Static item attribute: `startup_initialized(path::to::function)`
```rust
#[startup_initialized(init_my_data)]
static DATA: [i8; 3] = [1,2,3];
```
The `startup_initialized` attribute prevents the following static (a so-called *"startup initialized static"*) from being allocated on read-only memory so that it is possible to mutate it at runtime (albeit in a restricted manner). The attribute takes one argument which specifies a full path to a function (a so-called *"startup initialization function"*) which will be called at the beginning of main to initialize the associated static, or rather, to re-initialize it, since a const expression (here `[1,2,3]`) has already been used to initialize the static at that point. The associated startup initialization function must lie within the same crate and it must have a `startup_initialization` attribute attached to it. Multiple different startup initialized statics may have the same startup initialization function.

### 2. Function attribute: `startup_initialization`
```rust
#[startup_initialization]
fn init_my_data() {
    DATA[0] = 11;
}
```
The `startup_initialization` attribute takes no arguments and it can only be attached to a function (a so-called *"startup initialization function"*) that takes no arguments and has an empty return. A user cannot call such a function directly, but a call can happen implicitly and only either in the beginning of main function or after one or more calls to other startup initialization functions, the first of which being called in the beginning of main. All startup initialized statics, which have a given startup initialization function as the argument to their `startup_initialized` attribute (the so called *"associated startup initialized statics"*), are considered mutable inside the body of said function so that they can be initialized by using runtime functionality. A startup initialization function may also access (either directly or indirectly through a function call) other (non-associated) startup initialized statics, even if they're in other crates, but the dependency on other startup initialized statics must not be circular. For example, given the startup initialization functions `func_x` and `func_y`, and their respective associated startup initialized statics `data_x` and `data_y`, if `func_x` accesses `data_y` and `func_y` accesses `data_x`, then this forms a circular dependency which would result in a compile-time error indicating the responsible startup initialization functions. All function calls made by a startup initialization function (either directly or indirectly) must be statically dispatched. The previous restriction is not strictly necessary, and could be relaxed later to allow dispatching dynamically given that the compiler is able to exhaustively list all the functions that could possibly be called. Marking a startup initialization function as `pub` should result in a warning saying something like *"visibility has no bearing on startup initialization functions"*.

## A possible implementation

It's getting tedious to write *"startup initialization"* and *"startup initialized"*, so from now on, I'll shorten both of them to SI.

Inside the body of an SI function, all of its associated SI statics are made mutable. It is as if the following kind of transformation happened:

From:
```rust
type Data = [i8; 3];

#[startup_initialized(init_my_data)]
static DATA: Data = [1,2,3];

#[startup_initialization]
fn init_my_data() {
    DATA[0] = 11;
    mutate(&mut DATA);
}

fn mutate(_: &mut Data) {}
```
To:
```rust
type Data = [i8; 3];

#[startup_initialized(init_my_data)]
static DATA: Data = [1,2,3];

#[startup_initialization]
fn init_my_data() {
    let tmp: &mut Data = unsafe { std::mem::transmute(&DATA) };
    (*tmp)[0] = 11;
    mutate(&mut (*tmp));
}

fn mutate(_: &mut Data) {}
```
Remember that SI statics are never placed in read-only memory, which makes the transmute to `&mut` not be undefined behaviour (I think).

In order to both catch circular dependencies between SI functions and to be able to determine the order in which SI functions should be called at the beginning of main, the compiler must keep track of which SI statics each function and each static item (through a reference) may potentially access. An SI static must not be accessed (neither read nor written to) before its associated SI function is called (and while said SI function call is running, we allow all functions to read and write to the SI static if they take it as a `&mut` argument as long as they don't store a long-lived mutable reference to it, but I think Rust's borrow checking rules prevent this from happening).

The compiler should have a way to attach metadata to functions and static items. Specifically the metadata should contain information about which SI functions must be called before the function is allowed to be called or before the static item is allowed to be accessed. This information would be in the form of a set of some kind of unique identifiers which can irrefutably identify SI functions both in the current crate and in other, library, crates. From now on, I'll call this metadata information a *"dependency set"* (it's also a set in the mathematical sense that there are no duplicate elements). Each static item's (both regular and startup initialized ones') dependency set should include the associated SI functions of all the SI statics that the static item references (the static item may be a reference to an SI static or it may be a struct which has a field that's a reference to an SI static or it may be an SI static itself etc. [yes, I'm using the verb *"references"* a bit loosely here, but in some sense a static X **is** a reference to X, albeit an automatically dereferencing one]). Whenever a function is compiled, the compiler would analyze the function and determine its dependency set to be a union of the dependency sets of all the functions it may potentially call **and** the dependency sets of all the static items it may potentially access. One exception to the previous rule is that for any given SI function, that SI function itself would be removed from its own dependency set (a function can't depend on itself being called before it is called for the first time). Another exception to the rule is that for main function, this analysis is not performed nor any dependency set created.

Dynamic dispatch makes it difficult to know what functions could potentially be called and what statics might potentially be accessed. That's why I suggest that for the initial version, we disallow calling functions through trait objects and function pointers in SI functions. And therefore, although I previously described a dependency set as a simple set (with zero or more elements), now we see that dependency sets need to also have a possible state which represents "unknown" (think of `Option::None`). If a function may potentially either call a function through dynamic dispatch or call a function that has "unknown" dependency set, then its dependency set would be set to "unknown". If an SI function has "unknown" dependency set, that results in a compile-time error.

At the beginning of the main function, all SI functions of the current crate and all SI functions of all the extern crates it depends upon are called implicitly by the compiler, once each. The order in which the SI functions are called is determined by their dependency sets. Given an SI function X, all SI functions in X's dependency set must be called before X is called.

# Drawbacks

I don't know, but I imagine that this could have a negative impact on compile times.

# Alternatives

The best alternative I can think of is not doing this and relying on lazy initialization.

# Unresolved questions

Can the compiler prove that it knows about all the possible functions that could be called through a certain dynamic dispatch? (I simply don't know enough about this stuff)
