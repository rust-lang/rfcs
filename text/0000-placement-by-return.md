- Feature Name: placement-by-return
- Start Date: 2020-01-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Implement ["placement"](https://internals.rust-lang.org/t/removal-of-all-unstable-placement-features/7223) with no new syntax, by extending the existing capabilities of ordinary `return`. This involves [copying Guaranteed Copy Elision rules pretty much wholesale from C++17](https://jonasdevlieghere.com/guaranteed-copy-elision/), adding functions like `fn new_with<F: FnOnce() -> T, T: ?Sized>(f: F) -> Self` to Box and `fn push_with<F: FnOnce() -> T>(&mut self, f: F)` to Vec to allow performing the allocation before evaluating F, providing raw access to the return slot for functions as an unsafe feature, and allowing functions to directly return **Dynamically-Sized Types** (DSTs) by compiling such functions into a special kind of "generator".

Starting with the questions given at the end of the [old RFC's mortician's note](https://github.com/rust-lang/rust/issues/27779#issuecomment-378416911):

 * **Does the new RFC support DSTs? serde and fallible creation?** Yes on DSTs. On fallible creation, it punts it into the future section.
 * **Does the new RFC have a lot of traits? Is it justified?** It introduces no new traits at all.
 * **Can the new RFC handle cases where allocation fails? Does this align with wider language plans (if any) for fallible allocation?** Yes.
 * **are there upcoming/potential language features that could affect the design of the new RFC? e.g. custom allocators, NoMove, HKTs? What would the implications be?** Not really. `Pin` can have a `new_with` function just like anyone else, custom allocators would happen entirely behind this, true HKT's are probably never going to be added, and associated type constructors aren't going to affect this proposal since the proposal defines no new traits or types that would use them.

## Glossary

- **GCE:** [Guaranteed Copy Elision](https://stackoverflow.com/questions/38043319/how-does-guaranteed-copy-elision-work).
- **NRVO:** [Named Return Value Optimization](https://shaharmike.com/cpp/rvo/).
- **DST:** [Dynamically-Sized Type](https://doc.rust-lang.org/reference/dynamically-sized-types.html).
- **HKT:** [Higher-Kinded Type](https://stackoverflow.com/a/6417328/3511753).

# Motivation
[motivation]: #motivation

Rust has a dysfunctional relationship with objects that are large or variable in size. It can accept them as parameters pretty well using references, but creating them is unwieldy and inneficient:

* A function pretty much has to use `Vec` to create huge arrays, even if the array is fixed size. The way you'd want to do it, `Box::new([0; 1_000_000])`, will allocate the array on the stack and then copy it into the Box. This same form of copying shows up in tons of API's, like serde's Serialize trait.
* There's no safe way to create gigantic, singular structs without overhead. If your 1M array is wrapped in a struct, then the only safe way to dynamically allocate one is to use `Box::new(MyStruct::new())`, which ends up creating an instance of `MyStruct` on the stack and copying it to the box, 1M array included.
* You can't return bare unsized types. [RFC-1909](https://github.com/rust-lang/rfcs/blob/master/text/1909-unsized-rvalues.md) allows you to create them locally, and pass them as arguments, but not return them.

As far as existing emplacement proposals go, this one was written with the following requirements in mind:

* **It needs to be possible to wrap it in a safe API.** Safe API examples are given for built-in data structures, including a full sketch of the implementation for Box, including exception safety.
* **It needs to support already-idiomatic constructors like `fn new() -> GiantStruct { GiantStruct { ... } }`** Since this proposal is defined in terms of Guaranteed Copy Elision, this is a gimme.
* **It needs to be possible to in-place populate data structures that cannot be written using a single literal expression.** The `write_return_with` intrinsic suggested in this proposal allows this to be done in an unsafe way. Sketches for APIs built on top of them are also given in the [future-possibilities] section.
* **It needs to avoid adding egregious overhead in cases where the values being populated are small (in other words, if the value being initialized is the size of a pointer or smaller, it needs to be possible for the compiler to optimize away the outptr).** Since this proposal is written in terms of Guaranteed Copy Elision, this is a gimme. The exception of the "weird bypass functions" `read_return_with` and `write_return_with` may seem to break this; see the [example desugarings here](#How-do-the-return-slot-functions-work-when-the-copy-is-not-actually-elided) for info on how these functions work when the copy is not actually elided.
* **It needs to solve most of the listed problems with the old proposals.** Since this one actually goes the distance and defines when copy elision will kick in, it fixes the biggest problems that the old `box` literal system had. It is also written in terms of present-day Rust, using `impl Trait`, and with the current direction of Rust in mind.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If you need to allocate a very large data structure, or a DST, you should prefer using `Box::new_with` over `Box::new`. For example:

```rust
let boxed_array = Box::new_with(|| [0; 1_000_000]); // instead of Box::new([0; 1_000_000])
let boxed_data_struct = Box::new_with(DataStruct::new); // instead of Box::new(DataStruct::new())
```

The `new_with` function will perform the allocation first, then evaluate the closure, placing the result directly within it. The `new` function, on the other hand, evaluates the argument *then* performs the allocation and copies the value into it.

Similar functions exist for Vec and other data structures, with names like `push_with` and `insert_with`.

When writing constructors, you should create large data structures directly on the return line. For example:

```rust
// This function will allocate its return value directly in the return slot.
fn good() -> [i32; 1_000_000] {
    [1; 1_000_000]
}

// This function will return a raw slice, and can also be called as `Box::new_with(good_dst)`
fn good_dst() -> [i32] {
    let n = 1_000_000;
    [1; n]
}

// This function may or may not copy the array when it returns.
fn bad() -> [i32; 1_000_000] {
    let mut arr = [0; 1_000_000];
    for i in 0..1_000_000 {
        arr[i] = i;
    }
    arr
}

// This function will compile successfully with #![feature(unsized_locals)]
// but it will allocate the array on the stack, which is probably very bad.
fn bad_dst() {
    fn takes_dst(_k: [i32]) {}
    fn returns_dst() -> [i32] { let n = 1_000_000; [1; n] }
    takes_dst(returns_dst())
}
```

This is guaranteed to see through if expressions, function calls, literals, unsafe blocks, and the return statement. It is not guaranteed to see through variable assignment or break with value.

```rust
// "good" functions will write their results directly in the caller-provided slot
fn good() -> Struct {
    if something() { Struct { ... } } else { Struct { ... } }
}
fn good() -> Struct {
    loop { return Struct { ... } }
}
fn good() -> Struct2 {
    Struct2 { member: Struct { ... } }
}
// "bad" functions will not necessarily do that
fn bad() -> Struct {
    loop { break Struct { ... } }
}
fn bad() -> Struct {
    let q = Struct { ... };
    q
}
fn bad() -> Struct2 {
    let q = Struct { ... }
    Struct2 { member: q }
}
```

In other words, Rust does not currently guarantee Named Return Value Optimization.

In the rustonomicon, it should mention that the `write_return_with` intrinsic can be used to build a function that's equivalent to `bad()`:

```rust
use std::mem::{write_return_with, ReturnSlot};
use std::alloc::Layout;
fn not_bad() -> [i32] {
    unsafe {
        write_return_with(Layout::new::<[i32; 1_000_000]>(), |arr: *mut u8| {
            let arr: &mut [i32] = slice::from_raw_parts_mut(arr, 1_000_000);
            for i in 0..1_000_000 {
                arr[i] = i;
            }
            arr
        })
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When a function constructs a value directly in of the following places:

- A return statement.
- The last expression of the last block, which is implicitly used as a return statement.

Then the return value's observable address will not change, if the value's type is:

- Larger than an implementation-defined size (eg a register).
- Unsized.

(note: we could add a trait in a future RFC that would guarantee a type is always emplaced, even when it's register-sized, if use cases appear that need the "unchanged address" guarantee)

## Did you say I can return unsized types?

A function that directly returns an unsized type will be split into two functions, essentially as a special kind of generator:

- The first half will yield the layout of the memory needed to allocate the return value.
- The second half will write that return value into the allocated memory.

The compiler and unsafe code must ensure that the allocated memory passed to the second function matches the layout yielded by the first function.

```rust
// sugar
fn my_function() -> str {
    *"hello world"
}
fn function_that_calls_my_function() -> str {
    println!("Hi there!");
    my_function()
}

// desugar my_function
// this is written in order to aid understanding, not because these are good APIs
use std::alloc::Layout;
struct __MyFunction__Internal {
  local_0: &'static str,
}
impl __MyFunction__Internal {
  /* "unsize coroutines" are different from normal generators because their behavior is much more
    restricted. They yield exactly one Layout, and then they are finished by writing to a pointer. */
  unsafe fn start(state: *mut __MyFunction__Internal) -> Layout {
    /* As usual for generators, locals (including anonymous ones) get desugared into member
      variables of an anonymous struct. In this case, the local variable is just a string literal. */
    state.local_0 = "hello world";
    Layout::for_value("hello world")
  }
  unsafe fn finish(state: *mut __MyFunction__Internal, slot: *mut u8) -> &mut str {
    ptr::copy(state.local_0, slot.as_mut_ptr(), mem::size_of_val(state.local_0));
    /* Notice how I also have to return a properly-built fat pointer?
       This isn't very important for string slices, but it's mandatory for trait objects, because I need to
       supply a vtable. */
    str::from_raw_parts_mut(slot, state.local_0.len())
  }
}

// desugar function_that_calls_my_function
use std::alloc::Layout;
struct __FunctionThatCallsMyFunction__Internal {
  delegate: __MyFunction__Internal,
}
impl __FunctionThatCallsMyFunction__Internal {
  unsafe fn start(state: *mut __FunctionThatCallsMyFunction__Internal ) -> Layout {
    /* first, run everything leading up to the return */
    println!("Hi, there!");
    /* then, desugar the return (in this case, by delegating */
    __MyFunction__Internal::start(&mut state.delegate)
  }
  unsafe fn finish(state: *mut __FunctionThatCallsMyFunction__Internal, slot: *mut u8) -> &mut str {
    __MyFunction__Internal::finish(&mut state.delegate, slot)
  }
}
```

This interface ends up putting some pretty harsh limitations on what functions that return unsized types can do. An unsized type-returning function can always return the following kinds of expression:

* Constants and dereferencing constants (as shown above). These are desugared by yielding the layout of the literal and returning the literal by copying it.
* Directly returning the value of another function that also returns the same unsized type. These are desugared by forwarding, as shown above with `function_that_calls_my_function`.
* Variable-length array literals, similar to those in [RFC 1909](https://github.com/rust-lang/rfcs/blob/master/text/1909-unsized-rvalues.md). These are desugared by yielding the length variable, then returning the payload through ptr offsets and ptr writes.
* Unsized coercions. These are desugared by storing the sized type as function state, yielding the layout of the sized type, and returning by copying.
* Blocks, unsafe blocks, and branches that have acceptable expressions in their tail position.

As is typical for generators, these functions may need to be desugared into simple "state machines" if they return branches or have more than one exit point.

```rust
fn with_branch() -> dyn MyTrait {
    if coin_flip() {
        MyTraitImpl()
    } else {
        ComplicatedTraitImpl()
    }
}

// desugar with_branch
// this is written in order to aid understanding, not because these are good APIs
use std::alloc::Layout;
enum __WithBranch__Internal {
  S0(MyTraitImpl),
  S1(ComplicatedTraitImpl),
}
impl __WithBranch__Internal {
  unsafe fn start(state: *mut __WithBranch__Internal) -> Layout {
    if coin_flip() {
        *state = __WithBranch__Internal::S0(MyTraitImpl());
        Layout::new::<MyTraitImpl>()
    } else {
        *state = __WithBranch__Internal::S1(ComplicatedTraitImpl());
        Layout::new::<ComplicatedTraitImpl>()
    }
  }
  unsafe fn finish(state: *mut __MyFunction__Internal, slot: *mut u8) -> &mut dyn MyTrait {
    match *state {
        __WithBranch__Internal::S0(value) => {
            ptr::copy(&value, slot, mem::size_of::<MyTraitImpl>(value));
            &mut mem::transmute::<*mut u8, MyTraitImpl>(slot) as &mut dyn MyTrait
        }
        __WithBranch__Internal::S1(value) => {
            ptr::copy(&value, slot, mem::size_of::<ComplicatedTraitImpl>(value));
            &mut mem::transmute::<*mut u8, ComplicatedTraitImpl>(slot) as &mut dyn MyTrait
        }
    }
  }
}
```

More elaborate example:

```rust
fn with_multiple_exit_points() -> [i32] {
    while keep_going() {
        if got_cancelation() {
            return []; // returning constant
        }
    }
    let n = 100;
    [1; n] // returning variable-length-array expression
}

// desugar with_multiple_exit_points
enum __WithMultipleExitPoints__Internal {
  S0(),
  S1(i32, usize),
}
impl __WithMultipleExitPoints__Internal {
  unsafe fn start(state: *mut __WithMultipleExitPoints__Internal) -> Layout {
    while keep_going() {
        if got_cancelation() {
            *state = __WithMultipleExitPoints__Internal::S0();
            return Layout::for_value(&[]);
        }
    }
    let n = 100;
    *state = __WithMultipleExitPoints__Internal::S1(1, n);
    Layout::from_size_align_unchecked(n * mem::size_of::<i32>(), mem::align_of::<i32>())
  }
  unsafe fn finish(state: *mut __WithMultipleExitPoints__Internal, slot: *mut u8) -> &[i32] {
    match *state {
        __WithMultipleExitPoints__Internal::S0() => {
            slice::from_raw_parts_mut(slot, 0)
        }
        __WithMultipleExitPoints_Internal::S1(value, n) => {
            for i in 0..n { ptr::write(slot.offset(i), value) };
            slice::from_raw_parts_mut(slot, n)
        }
    }
  }
}
```

## How this works with sized types

Any function that returns a sized type can be trivially adapted to the "unsized return generator" interface of `start()` and `finish()`, by having start return a constant and have finish do all the work.

```rust
fn return_a_sized_value() -> i32 {
    println!("got random value");
    4 // determined by a fair dice roll
}

// desugar return_a_sized_value
// this is written in order to aid understanding, not because these are good APIs
use std::alloc::Layout;
struct __ReturnASizedValue__Internal;
impl __ReturnASizedValue__Internal {
  unsafe fn start(state: *mut __WithBranch__Internal) -> Layout {
    // there's only one possible layout for sized types; that's the definition of being "sized"
    Layout::new::<i32>()
  }
  unsafe fn finish(state: *mut __MyFunction__Internal, slot: *mut u8) -> &mut i32 {
    // just invoke the function and copy its return value into the slot
    // this mostly gets the behavior we want, but see below for info about copy elision
    ptr::copy(&return_a_sized_value(), slot, mem::size_of::<i32>());
    slot as *mut i32 as &mut i32
  }
}
```

This is how it would conceptually work whenever a facility designed to handle placement is invoked on a function that returns a sized value. This is not how it should be implemented, though, because we don't want a bunch of unnecessary copying.

## Absolutely minimum viable copy elision

To make sure this functionality can be used with no overhead, the language should guarantee some amount of copy elision. The following operations should be guaranteed zero-copy:

* Directly returning the result of another function that also returns the same type.
* Array and struct literals, including struct literals ending with an unsized value.
* Blocks, unsafe blocks, and branches that have acceptable expressions in their tail position.

These operations are "must-have" copy elision because they allow functions returning unsized types through unsafe methods to be composed with other functions without introducing overhead.

In the case of array and struct literals, the copy elision must be recursive. For example:

```rust
fn no_copy() -> Struct2 {
    Struct2 { member: Struct { member: getData() } }
}
```

In the above example, the data returned by `getData()` may be sized or unsized. Either way, it should not be copied at all (aside from its initial creation), whether when constructing the Struct, the Struct2, or returning the data.

Guaranteed Copy Elision only kicks in when a GCE-applicable expression is directly returned from a function, either by being the function's last expression or by being the expression part of a `return` statement.

## The unsafe method of writing return values

The `std::mem` module exposes two symmetrical intrinsics that can allow you to achieve Copy Elision in any circumstance.

### `write_return_with`

```rust
pub unsafe fn write_return_with<T, F: for<'a> FnOnce(*mut u8)>(f: F) -> T {
    write_unsized_return_with(Layout::new::<T>(), |p| {f(p); &mut *(p as *mut T)})
}
extern "intrinsic" pub unsafe fn write_unsized_return_with<T: ?Sized, F: for<'a> FnOnce(*mut u8) -> &mut T>(layout: Layout, f: F) -> T;
```

[write_return_with]: #write_return_with

Directly access the caller-provided return slot.

The return slot is an uninitialized memory space provided by a function's caller to write the return value, and implicitly passed as a pointer. A function which directly returns the result of evaluating another function, like `fn foo() -> ReturnedValue { bar() }`, may simply pass the slot along, thus avoiding expensive copies. In cases where it is impossible to implement a function by directly returning the value of a function call or a primitive expression, directly accessing the return slot and writing it directly may be necessary.

Return slots are not always used; small values like pointers, numbers, and booleans may be returned by filling a register instead. In this case, `write_return_with` will implicitly create a "return slot", pass a pointer to it to `f`, and then copy the return value from the "return slot" to the register.

The pointer returned by `write_unsized_return_with`'s callback should be the same as the one provided to it, along any necessary unsize information (such as the slice's length or the trait object's vtable pointer). Additionally, when using `write_unsized_return_with` for sized types, the provided layout must be exactly the same as the one produced by `Layout::new<T>()`. These two values must be correct even when they're redundant. A function that returns a slice may return a slice that is smaller than the requested allocation Layout, in case it is unable to predict the amount of data that will be available to it, but when producing a trait object, it must know exactly the right size for its allocation.

#### Panicking

`f` is allowed to panic. If it panics, the underlying memory should still be freed (the built-in collections are well-behaved), but the return slot implementation will assume that the allocated memory was not initialized, *and shall not call T's destructor*. If `f` allocates additional resources itself before panicking, it is responsible for freeing it.

If allocation fails, `write_return_with` may panic without calling `f`. The return slot may also be pre-allocated, resulting in the allocation failure before the call to `write_return_with` is reached.

IMPORTANT IMPLEMENTATION NOTE: Because of the way unsized return generators are codegenned as generators, it would be possible to tell that `write_unsized_return_with` wasn't actually panicking by wrapping its invocation in `catch_panic`. To ensure the user cannot do this, the closure passed to `catch_panic` must return a sized type; we still technically won't be unwinding through their stack frames, but we will be calling the drop functions with `is_panicking` set to true, so they won't be able to tell. Additionally, of course, the return slot for sized types is always pre-allocated, so this function will never panic in that case.

#### Example: zeroed_array

```rust
unsafe fn zeroed_array<T>(n: usize) -> [T] {
    let (array_layout, _) = Layout::new::<T>().repeat(n).unwrap();
    write_unsized_return_with(
        array_layout,
        |slot: *mut u8| {
            for i in 0 .. size_of::<T>() * n {
                *slot.offset(i) = 0;
            }
            slice::from_raw_parts::<'_, T>(slot, n)
        },
    )
}
```

### `read_return_with`

```rust
pub unsafe fn read_return_with<'a, T, F: FnOnce() -> T>(f: F, slot: &mut MaybeUninit<T>) {
    let finish = read_unsized_return_with(f);
    debug_assert!(finish.layout() = Layout::new::<T>());
    finish.finish(slot);
}
#[lang(read_unsized_return_with_finish)]
pub trait ReadUnsizedReturnWithFinish<T: ?Sized> {
    pub fn finish(self, &mut MaybeUninit<T>);
    pub fn layout(&self) -> Layout;
}
extern "intrinsic" pub unsafe fn read_unsized_return_with<'a, T: ?Sized, F: FnOnce() -> T>(f: F) -> impl ReadUnsizedReturnWithFinish<T>;
```

[read_return_with]: #read_return_with

Directly supply a return slot. See [`write_return_with`] for information on what return slots are.

#### Example: Box::new_with

```rust
struct BoxUninit<T: ?Sized> {
    p: Option<NonNull<MaybeUnint<T>>>,
}
impl<T: ?Sized> Drop for BoxUninit<T> {
    fn drop(&mut self) {
        if let Some(p) = self.p {
            System.dealloc(p.as_mut_ptr() as *mut u8, Layout::for_value(&*p);
        }
    }
}
struct Box<T: ?Sized> {
    p: NonNull<T>,
}
impl<T: ?Sized> Box<T> {
    fn new_with<F: FnOnce() -> T>(f: F) -> Self {
        unsafe {
            let mut uninit = BoxUninit { p: None };
            let state = read_unsized_return_with(f);
            let p = NonNull::from_mut_ptr(GlobalAlloc.alloc(finish.layout()));
            uninit.p = Some(p);
            state.finish(p.as_mut_ptr() as *mut MaybeUninit<T> as &mut MaybeUninit<T>);
            forget(uninit);
            Box { p }
        }
    }
}
impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        let layout = Layout::for_value(&*p);
        drop_in_place(&mut *p);
        System.dealloc(p.as_mut_ptr() as *mut u8, layout);
    }
}
```

#### Example: Vec::extend_from_raw_slice_with

Copy-and-paste this example to create `String::extend_from_raw_str_with`.

```rust
impl<T> Vec<T> {
    pub fn extend_from_raw_slice_with<F: FnOnce() -> [T]>(&mut self, f: F) {
        let finish = read_unsized_return_with(f);
        let layout = finish.layout();
        debug_assert_eq!(layout.align(), align_of::<T>());
        let count = layout.size() / size_of::<T>();
        self.0.reserve(count);
        let p = ((&mut self[..]) as *mut [u8]).offset(self.len());
        // this slice may be smaller than the given allocation, as described above
        let slice = finish.finish(p as *mut MaybeUninit<[u8]> as &mut MaybeUninit<[u8]>);
        debug_assert!(slice.len() <= count);
        self.set_len(self.len() + slice.len());
    }
}
```

#### Example: raw_as_bytes_with

```rust
mod str {
  /// This is a function adapter. Usage:
  ///
  ///     fn get_str() -> str;
  ///     let get_bytes = str::raw_as_bytes_with(get_str);
  ///     get_bytes()
  pub fn raw_as_bytes_with<Args, F: FnOnce<Args, Output=str>>(f: F) -> impl FnOnce<Args, Output=[u8]> {
    unsafe {
      struct ConvertFn<F>(F);
      impl<Args, F: FnOnce<Args, Output=str>> FnOnce<Args> for ConvertFn<F> {
        type Output = [u8];
        fn call(self, a: Args) -> [u8] {
          let finish = read_unsized_return_with(|| self.0.call(a));
          write_unsized_return_with(
            finish.layout(),
            |slot: &mut MaybeUninit<[u8]>| finish.finish(MaybeUninit::from_mut_ptr(slot.as_mut_ptr() as *mut str))) as *mut str as *mut [u8] as &mut [u8]
        }
      }
    }
  }
}
```

### Lint: incorrect use of `write_return_with`

All usage of `write_return_with` should call it directly in their return clause, or in an unsafe block directly in their return clause.

```rust
fn good() -> BigDataStructure {
    unsafe {
        write_return_with(Layout::new::<BigDataStructure>(), |slot| {
            populate_big_data_structure(slot);
        })
    }
}
fn bad() -> BigDataStructure {
    unsafe {
        let k = write_return_with(Layout::new::<BigDataStructure>(), |slot| {
            populate_big_data_structure(slot);
        });
        k
    }
}
fn also_bad_even_though_it_technically_works() -> BigDataStructure {
    unsafe {
        if coin_flip() {
            write_return_with(Layout::new::<BigDataStructure>(), |slot| {
                populate_big_data_structure(slot);
            })
        } else {
            unimplemented!()
        }
    }
}
```

Assigning the result of `write_return_with` to a local variable, like in `bad()`, is essentially a no-op. The data structure is being emplaced into `k`, and then it's being immediately copied on return. If you actually intend to manually initialize a local value, just use `MaybeUninit` like a normal person.

Since it's always possible to write a function with `write_return_with` in the return clause of a function or an unsafe block which is itself in the return clause of a function (if you need a conditional somewhere, you can just put the conditional *inside* the closure), the lint should just require you to do that.

### How do the return slot functions work when the copy is not actually elided?

When the copy is not elided (which is only ever the case for values with a statically known size), they simply use temporaries for their implementation. The return slot functions still have to "work" when the copy is not elided, so that they can be used in generic contexts.

```rust
// Imagine these functions being used only when T is sized and less than one pointer.
#[inline(always)]
unsafe fn write_return_with<T, F: FnOnce(*mut u8)>(f: F) -> T {
    let slot: MaybeUninit<T> = MaybeUninit::empty();
    f(&mut slot as *mut MaybeUninit<T> as *mut u8);
    slot.take()
}
#[inline(always)]
unsafe fn read_return_with<'a, T, F: FnOnce() -> T>(f: F, slot: *mut u8) {
    let value = f();
    ptr::write(&value, slot.get() as *mut T);
}
```

## New functions added to existing types

```rust
impl<T: ?Sized> Box<T> {
    fn new_with<F: FnOnce() -> T>(f: F) -> Self;
}
impl<T> Vec<T> {
    fn from_raw_slice_with<F: FnOnce() -> [T]>(f: F);
    fn push_with<F: FnOnce() -> T>(&mut self, f: F);
    fn insert_with<F: FnOnce() -> T>(&mut self, index: usize, f: F);
    fn extend_from_raw_slice_with<F: FnOnce() -> [T]>(&mut self, f: F);
}
impl <K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S> {
    /// Unlike the regular `insert` function, this one returns a `&mut` to the new one,
    /// not an optional old one.
    /// This is because the other one can't be returned without copying the old value
    /// from the map to the return slot, which is wasteful for large objects.
    fn insert_with<'a, FV: FnOnce() -> V>(&'a mut self, k: K, fv: FV) -> &'a mut V;
}
impl <K: Eq + Hash, V, S: BuildHasher> hash_map::OccupiedEntry<K, V, S> {
    fn insert_with<FV: FnOnce() -> V>(&mut self, k: K, fv: FV) -> V;
}
impl <K: Eq + Hash, V, S: BuildHasher> hash_map::VacantEntry<K, V, S> {
    fn insert_with<'a, FV: FnOnce() -> V>(&'a mut self, k: K, fv: FV) -> &'a mut V;
}
impl String {
    fn from_raw_str_with<F: FnOnce() -> str>(f: F);
    fn extend_from_raw_str_with<F: FnOnce() -> str>(&mut self, f: F);
}
mod str {
    // Yes, this is a generic function adapter, up to and including use of the unstable Fn* trait.
    // Use it like raw_as_bytes_with(fn_that_returns_raw_str)
    fn raw_as_bytes_with<Args, F: FnOnce<Args, Output=str>>(f: F) -> impl FnOnce<Args, Output=[u8]>;
}
```

# Drawbacks
[drawbacks]: #drawbacks

The biggest issue with Guaranteed Copy Elision is that it's actually kind of hard to *specify* it. The abstract machine, after all, doesn't specify the amount of memory that a function call occupies; that's part of the ABI. So how do you phrase GCE in an ABI-agnostic way? The C++ answer is "a convoluted combination of requirements about when the address of an object may change and when a move constructor may be called". The specification of GCE in the to-be-written Rust specification will probably be just as bad, since while it's pretty obvious how it works for unsized types (which cannot be moved without invoking an allocator, and we can certainly specify when that's allowed), we also want to guarantee it for sized types.

There have also been people recommending a more do-what-I-mean approach where `Box::new(f())` is guaranteed to perform copy elision. That would induce the absolute minimal churn, though how you'd handle intermediate functions like `fn foo(t: T) { box::new(t) }` is beyond me.

The third drawback, and I personally think this is the *worst* drawback, is that it's invisible. This means that when a user accidentally writes code that performs copies, it isn't always obvious that they messed up. There isn't any way to get existing code to achieve GCE without making it invisible, and I wanted to avoid churn in everyone's `new()` functions, as described in [rationale-and-alternatives]. Presumably, it can be solved using optional [lints].

Another major drawback is that it doesn't compose well with pattern matching (or `?`). Imagine you have a function `fn foo() -> Result<Struct, Error>` and you want to do something like `Box::new(foo()?)`, but you want to directly place it. This is impossible; not just because with closures `Box::new_with(|| foo()?)` won't do what you want, but any system where the function being called writes its entire return through a pointer cannot handle cases where the return is wrapped in a larger object (eg a Result). See [returning-unsized-results] for details.

Also, like all placement proposals, it involves adding a new API surface area to most of the built-in data structures. Since there are known problems with how this proposal works with error handling, the GCE-based version may end up being replaced with an approach that does.

Additionally, this proposal deliberately does not implement NRVO. This means people will end up writing stilted code, or just using the unsafe bypass functions, instead of doing what's most readable.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

At the heart of the rationale for this RFC is that, **if we want any kind of placement in Rust, there are only two possibilities:**

- Guaranteed Copy Ellision,

- Passing references to uninitialized memory.

This RFC chooses GCE. Everything else is a result of that choice.

## Vs passing uninitialized memory

There has been some discussion (note: if someone could find links, that'd be appreciated) of solving placement using references to undefined memory.

For instance, the following code:

```rust
let giantStruct = GiantStruct::new();
doSomething(&giantStruct);
```

could be replaced with

```rust
let giantStruct;
GiantStruct::init(&uninit giantStruct);
doSomething(&giantStruct);
```

(with the understanding that `&uninit` communicates that `giantStruct` should not be read, only written to)

There are several major drawback to this approach:

- It's less elegant. Data is created in an invalid state, then passed to a function. The GCE approach just returns the values the caller needs.

- It's not obvious what to do with `giantStruct` if `GiantStruct::init` returns early, panics, or calls a method of `GiantStruct` before it's done building the object. This is particularly relevant for `read`-like APIs, which will often fill only a part of the buffer passed to them. The GCE approach is simpler: it either returns usable data, or it panics.

- The caller has to know in advance how much memory to allocate; `giantStruct::init` has no obvious way to communicate how much memory it will need.

Overall, GCE just seems like the approach that is most idiomatic to Rust. Creating a large object is as simple as calling `GiantStruct::new`, like [existing code](https://github.com/rust-ammonia/ammonia/blob/a6f0cf7886653ce1a982aab1c522cd44eab1267a/src/lib.rs#L342-L355) does.

## Vs alternative ABI for returning Result

The biggest drawback to this proposal, like most placement proposals, is that it works poorly with Result. You can emplace a function returning `Result<T>` into a `Box<Result<T>>`, and you cannot emplace it into a `Box<T>`.

This proposal does not define an alternative ABI for results because it would be work poorly with `read_return_with` and `write_return_with`. Both of these functions expose the return slot as a raw pointer, which means the return slot has to be in a continuous region of memory. This proposal does not, strictly speaking, lock out alternative ABIs for Result, but any such ABI would need to solve the design problems described in the [future-possibilities] section.

Using an alternative ABI remains practical as a solution for the "peanut butter" conditional cost (the `return_with` functions will be rarely used in most Rust code, especially when the code is just propogating error flags), but the way these functions work precludes using an alternative ABI as a solution for placement.

## Vs older emplacement proposals

[The previous emplacement RFC was rejected for the following reasons](https://github.com/rust-lang/rust/issues/27779#issuecomment-378416911), most of which this new RFC addresses:

* The implementation does not fulfil the design goals

    * Place an object at a specific address

      This is literally what `read_return_with` does.

    * Allocate objects in arenas (references the original C++ goals)

      This is what `new_with` and related functions achieve.

    * Be competitive with the implementation in C++

      Passing a closure is essentially the same thing as passing an initializer list, so it should have the same performance as C++ emplacement.

 * The functionality of placement is unpredictable

   This is probably the least-well-addressed concern. This RFC spends a lot of its text spelling out when GCE can kick in, which means it should be possible for a coder (or a lint) to tell whether a value is being copied or not, but it's still invisible.

   The other way it tries to address predictability is by offering the explicit-mode `write_return_with` intrinsic. Unfortunately, it's unsafe.

 * Specific unresolved questions

    * make placement work with fallible creation

      This RFC honestly has terrible support for this, mostly because it's painted itself into a corner. The sized-value-returning functions all have to be emplaceable, and when they return a Result, they return it by writing it directly through a pointer all at once.

    * trait design

      The `ReadUnsizedReturnWithFinish` trait will probably be somewhat controversial. It is an unsafe abstraction, but that's not a good excuse for a poor design. OTOH, I don't know of a better design to use.

       For providing a way to allocate unsized types by return, I don't think its design could be any better: you're passing a layout, and getting a pointer that you can fill with your data.

       The sized type story is the annoying one, since we're using an abstraction that pretends you're allocating space at the callee-time when you really allocated the space before it was even called and the compiler has to generate a fake implementation just to keep up the illusion. The only-for-sized-types wrapper functions tries to paper over that, so it at least isn't requiring `write_return_slot_with` users to provide a Layout that only has one valid value anyway.

    * support for DST/unsized structs

      Check!

 * More speculative unresolved questions include:

    * better trait design with in the context of future language features

     I suppose the read/write return slot with functions would probably be affected by future proposals
     for custom unsized types. Since they use fat pointers, it should work no matter what they are, but
     there might eventually be a better way to handle that.

    * interaction between custom allocators and placement

      This interface should support any allocator, since it uses a "supply your own pointer" philosophy for `read_return_with` users like collections.

## Vs other possible tweaks to this RFC

GCE is integral to this proposal, as mentionned above. The choice of modeling the proposal after existing parts of C++17 was made for familiarity and ease of implementation. Clang already does this stuff in LLVM, so we know it's possible.

The weird bypass functions, `write_return_with` and `read_return_with`, both accept closures because the only alternatives I could think of involved even weirder calling convention hacks, even weirder type system hacks (`fn get_return_slot<T>() -> *mut T`, for example, would be required to ensure that `T` is the same as the return type of the current function), or new language features like Go's nameable return slot.

The idea of compiling functions that return unsized types into pseudo-generators is somewhat common; the version that inspired this proposal came from the Rust Discord server. [Thanks @rpjohnst](https://discordapp.com/channels/442252698964721669/443151225160990732/550731094027403285)!

This RFC supports returning DSTs for two reasons:

* It allows emplacing dynamic byte blobs, which is really important for use cases like Mio and Serde.
* It is one of the main requirements set out in the older RFC's mortician's note.

The specific design for DST returning is, of course, optimized for emplacement, and the main use case is for emplacing big byte blobs. It supports returning trait objects as well, but that's not really what it's for, and it kind of shows. The macros described in [future-possibilities] for implicit emplacement could help cover that use case.

# Prior art
[prior-art]: #prior-art

Any place where this proposal is ambiguous? Let me know and I'll try to make it [the same as C++17 Guaranteed Copy Elision](https://jonasdevlieghere.com/guaranteed-copy-elision/).

It was after I came up with the idea that I realized `write_return_with` is basically a feature in the Go language, but Go has dedicated syntax for it and zero-initializes the return slot. I don't know of any prior art for `read_return_with`; it's not really all that similar to "placement new", even though it's arguably achieving the same goal.

The `_with` suffix is based on functions like [`get_or_insert_with`](https://doc.rust-lang.org/stable/std/option/enum.Option.html#method.get_or_insert_with). They're basically filling the same role as C++ `emplace_` methods.

# Unresolved questions
[unresolved-questions]: #Unresolved-questions

Commence the bikeshedding for alternative names and designs for the new functions:

* `write_return_with` could be named `get_return_slot`, and `read_return_with` could be named `put_return_slot`.
  * Or we could use the existing names, but with "slot" in them, like `write_return_slot_with`.
  * I personally oppose having "slot" in the function names, because for small values that get returned in registers, there isn't actually a return slot anywhere.
  * `read_return_with` seems like a particularly bad name, since it writes to the supplied pointer.
* What happens if a `read_return_with` user simply doesn't call `finish()`? It would be as if it had panicked, but the destructors would be called with `is_panicking` set to false.
  * This is the best way to support non-panicking fallible allocation, so it should probably be allowed. Existing allowed functions cannot experience this problem, since they return sized types, sized types have a no-op `start()`, and do all of their work in `finish()`, they won't experience breakage. Additionally, this sort of happy nonsense is already possible in `async` functions and generators, so most data types will already be able to handle this. Just don't return an unsized type if you don't want to act like a stackless coroutine.
* Right now, the API exclusively uses raw pointers, for syntactic simplicity. Maybe it should use `MaybeUninit`?
* NRVO is not impossible. In fact, since the implementation of RVO is almost certainly going to be based on MIR or LLVM IR, it's almost impossible *not* to provide NRVO in cases where the "named" variant desugars to exactly the same MIR code as the "unnamed" variant that this proposal guarantees RVO for. How much do we want to guarantee, vs just "accidentally providing it" because of the way the compiler is implemented?

# Future possibilities
[future-possibilities]: #Future-possibilities

## Additional lints
[lints]: #Additional-lints

In an attempt to directly address the problem of "it's too implicit", an HIR lint might be used to detect functions that are copying large amounts of data around. Deciding the cutoffs for this thing sounds like a mess, and it should probably be off by default for just that reason. Worse yet, we're stuck deciding when to warn on a struct literal where the struct itself gets returned in-place, *but none of its components do*. Honestly, the more I think about it, the more it seems like the "big copy" lint is just a gigantic quagmire of edge cases.

Additionally, a few much-simpler lints can push users in the direction of getting GCE. For example, since break-with-value isn't GCE'ed but return short-circuiting is, a lint should recommend returning from top-level loops instead of breaking from them. Similarly, a lint should recommend assigning struct members inline instead of going through local variables.

## Safe abstractions on top of `write_return_with`

While this RFC specifies a bunch of cases where existing containers should incorporate `read_unsized_return_with` to allow in-place construction of container members, it doesn't specify any built-in functions that should use `write_(unsized_)return_with` exclusively.

Functions which return potentially-large data structures that they construct will probably wind up using it. For example, once type-level integers are provided, a function for constructing arrays in-place would be possible:

```rust
fn fill_array<T, F: Fn() -> T, const N: usize>(f: F) -> [T; N] {
    unsafe {
        write_return_with(|slot: *mut u8| {
            let start = slot as *mut T;
            let filled = Filled(start, start);
            for _ in 0 .. N {
                read_return_with(&f, filled.1 as *mut u8);
                filled.1 = filled.1.offset(1);
            }
            forget(filled);
        });
    }
}

// This struct is used to take care of freeing the already-created items
// in case `f` panics in the middle of filling the array.
struct Filled<T>(*mut T, *mut T);
impl<T> Drop for Filled<T> {
    fn drop(&mut self) {
        while self.0 < self.1 {
            drop_in_place(self.0);
            self.0 = self.0.offset(1);
        }
    }
}
```

## Synergy with RFC-1909

This proposal was written to be independent from
[RFC-1909 (unsized rvalues)](https://github.com/rust-lang/rfcs/blob/master/text/1909-unsized-rvalues.md).

While the two proposals cover similar concepts (directly manipulating unsized types), they're mostly orthogonal in implementation. RFC-1909 is about using alloca to allow unsized types in function parameters and locals, and explicitly excludes function returns. This proposal is about emplacement, which doesn't require `alloca`, can be done exclusively through interfaces like `Box::new_with`.

In fact, even if both RFCs were implemented and standardized, it's not clear whether unsized returns should be allowed to be implicitly stored in locals using `alloca`. This is because `alloca` always grows the stack size without reusing existing memory, which means that any code creating locals of generic types in a loop would lead excessive stack sizes when called with an unsized type, in a way that isn't clear when looking at the offending code alone.

Moreover, functions returning unsized types wouldn't be allowed to store them in locals:

```rust
// fn invalid() -> [i32] {
//     let n = 100;
//     let k = [1; n];
//     k // ERROR: cannot return unsized variable
// }
```

This is because `invalid()` is implicitly compiled into a generator; the compiler must generate a state machine struct to store its state between the call to `start()` and the call to `finish()`. Since k is stored in `alloca()`-created space, it can't be stored in a fixed-size state machine.

On the other hand, storing a function's unsized return into a unsized local may be useful for some cases, such as trait objects. (though the concerns about implicit `alloca()` calls still apply).

However, doing so with a dynamically-sized slices would create a serious performance risk, and should at least be linted against.

```rust
fn valid_but_terrible() {
    let n: [i32] = returns_slice();
    takes_slice(n);
}
```

## New macros for implicit emplacement

The new methods proposed in this RFC are explicitly opt-in. The user chooses to call `Box::new_with(|| getActualData(...))` instead of `Box::new(getActualData(...))`. This syntax has a few drawbacks: it's verbose, it's explicit for someone skimming the code, and since it's not the default syntax, developers are likely not to use it unless they absolutely need to, even in cases where it could help performance.

A mitigation might be to add `box!`, `rc!`, `arc!` (and so on) macros to construct these types of object, similarly to existing `vec!` macro.

The idea being that the macro would do the work of chosing whether to use the "construct then copy" variant or the "emplace" variant. New developers would just be told to use `rc!(my_data)` without having to worry about which variant is used.

## Returning unsized Results
[returning-unsized-results]: #Returning-unsized-Results

This RFC's biggest drawback is its inability to handle faillible emplacement.

As mentionned above, the syntax `Box::new_with(|| foo()?)` wouldn't be applicable, even disregarding layout problems. Most likely, special-case methods would have to be added (eg `<doTheThing>_with_result`), so that the final syntax would look like:

```rust
let my_box : Box<[i32]> = Box::new_with_result(|| foo())?;
```

However, there are several obstacles to this syntax:

- It would require establishing a representation for DSTs wrapped in enums. Currently the only way to make a custom DST is to add another DST at the end of a struct.

  There is no way to do the same with an enum; if we wanted to implement that feature, we would need to establish semantics for how these enums interact with the rest of the language. These semantics are beyond the scope of this RFC.

- A `new_with_result` is only possible if the function can internally allocate enough memory to store a result, while only keeping the range of memory storing the payload at the end of the call, discarding the discriminant and the error (and accounting for cases where the err variant of the Result is much heavier than the ok variant).

  While this is feasible with Box or Rc (which could, for instance, always allocate extraneous bytes before their stored data to accomodate the discriminant), it's harder for methods such as `Vec::push_with`, which require working on contiguous memory.

There are several potential solutions to the second problem:

### Split the discriminant from the payload

We could decide that enums (or at least a specific subset of enums, including Result, Option, and other specialized types) should be stored differently.

Instead of storing the discriminant next to the payload, it would be stored separately, eg in a register. References to these enums, at least in some cases, would be fat pointers, with the first word pointing to the payload, and the second word storing the discriminant.

Functions returning `Result` could then store their payload in the return slot, while returning the discriminant directly through a register; they could even take two return slots: one for the `Ok` variant and another for the `Err` variant.

There are multiple strategies to implement that change:

- Change the ABI of all enums, so that all references to enums are always fat pointers (including function parameters, struct members, etc). This would be the most naive implementation, and incur some heavy breaking changes. Among other things, it would severely impact how Results and Options and similar types are stored in containers.

- Treat enums as normal, contiguous chunks of data when storing them and passing them to functions; and treat them as fat pointers when returning them (in a way that is purely transparent to the user).

  This may have non-trivial implications, especially if we later implement NRVO. Since NRVO would allow the user to take references to a Result even as it is being emplaced, the Result's layout could no longer be abstracted away by the compiler.

On the other hand, some of these objections may be overblown. A more detailed analysis may reveal that transparent discriminant splitting would have trivial semantics; or it could reveal complex edge cases.

Such an analysis is left for a future RFC.

### Always store the discriminant as a suffix

We could add a guarantee that any enum of the form `Result<SomeData, Err>`, when set to its `Ok` variant, has to start with a valid representation of `SomeData`; with that rule, casting a pointer to a result to a pointer to its payload is always a no-op.

This is already the case for some types where the discriminant is implicit (eg `Option<Box<i32>>`), but it could be generalized by requiring that the determinant, if not elided, must aways be stored after the payload, even for unsized types.

This solution would make it possible to adapt methods like `Vec::push_with`, `Vec::extend_from_raw_slice_with`, where data is always appended to the end of contiguous memory and allocated data past the size of the added element can be written to, without overwriting existing objects.

It would not work with methods such as `Vec::insert_with`, except in special cases where the Result is known to have the same memory layout as its payload (eg `Result<Box<i32>, ()>`).

This solution would incur additional cache inefficiency (imagine a Result where the discriminant is stored after 10 MB of payload), though this isn't something average users would need to be worry about, and there would be easy mitigations for those who do.

### Should this RFC pick a strategy?

The two solutions proposed above both require language-wide changes.

One might argue that a proposal for placement that doesn't support faillible returns "out of the box" is flawed. To quote a reviewer:

> After all, we surely want to support them eventually. [...] If this RFC is accepted, but we later discover that supporting fallible allocators requires a completely different design, we'll end up having to maintain two new sets of APIs in all the collections, on top of the existing non-placement-aware APIs. One set of duplicate APIs will already be a (well-justified) burden for language learners; there's no need to add another!

That said, I believe that this RFC is an acceptable Minimum Viable Product. To put it bluntly, any solution for faillible placement will probably require months of debate and analysis work, that would unnecessarily slow down the core proposal.

(Also, while a lot of people have shown enthusiasm for "split the discriminant" solution, I personally believe this enthusiasm is partly due to them underestimating the amount of semantic work needed to implement it.)

I also believe that this RFC is a good base for future development. While I don't want to commit to any future solution for faillible placement, both solutions this RFC proposes are compatible with this RFC, and I believe that any potential solution would rely on GCE as well (for the reasons explained in [rationale-and-alternatives]).

## Integration with futures, streams, serde, and other I/O stuff

<details>This RFC does not compose terribly well with `Result`, `Option`, and other pattern-matching constructs; this makes it hard to use with async APIs. `Future` and `Stream`, in particular, wraps the result of polling in the `Poll` enum, so while it's certainly possible to allocate such a result into a `Box<Poll<T>>` without copying, its not possible to get a `Box<T>` without copying given the existing futures API.

Mapping doesn't work, either, because it will pass the payload of a future as a parameter, which means that futures have to allocate storage for their contents. Sized types work as usual, and `Future<[u8]>` would wind up allocating stack space for the byte blob in order to pass it to the mapping function as a move-ref, as described in [RFC 1901 "unsized rvalues"](https://github.com/rust-lang/rfcs/blob/master/text/1909-unsized-rvalues.md).

Until unsized Results are implemented, the only real option for zero-copy data handling, in any of these cases, is to write functions with errors treated as side-channels. None of this is hard, but it is annoying and not very dataflow-y. It's largely inherent to the goal of having zero-copy I/O while the type system doesn't support keeping pattern-matching semantics orthogonal to memory representation.

```rust
trait Read {
    /// Read into a raw slice. This allows the reader to determine the proper size to read, unlike the
    /// other `read` function, which allow the caller to do so.
    ///
    /// # Example
    ///
    ///     fn read_to_end<R: Read>(r: mut Read, v: &mut Vec<u8>) -> Result<(), Error> {
    ///     let mut prev_len = v.len();
    ///     let mut err = None;
    ///     v.extend_from_raw_slice_with(|| reader.read_to_raw_slice(&mut err));
    ///     while err.is_none() && v.len() != prev_len {
    ///         prev_len = v.len();
    ///         v.extend_from_raw_slice_with(|| reader.read_to_raw_slice(&mut err));
    ///     }
    ///     if let Some(err) { Err(err) } else { Ok(()) }
    ///     }
    fn read_to_raw_slice(&mut self, err: &mut Option<Error>) -> [u8] {
        unsafe {
            mem::write_unsized_return_with(
                // This function should be overloaded with better default buffer sizes.
                // For example, BufReader can just use the size of its own buffer,
                // and File can use fstat.
                Layout::from_size_align_unchecked(super::DEFAULT_BUF_SIZE, 1),
                |slot: *mut u8| {
                    let slice = slice::from_raw_parts(slot, super::DEFAULT_BUF_SIZE);
                    match self.read(slice) {
                        Ok(count) => {
                            assert!(count <= super::DEFAULT_BUF_SIZE),
                            slice[..count]
                        }
                        Err(e) => {
                            *err = e;
                            []
                        }
                    }
                }
            )
        }
    }
}
```

Presumably, the asynchronous systems could yield non-blocking Read implementations as streams or futures,
but, based on [previous discussion of what a good I/O abstraction would be](https://users.rust-lang.org/t/towards-a-more-perfect-rustio/18570), we probably want to avoid using a get-a-byte abstraction for everything.

```rust
/// A non-blocking, non-copying, unbounded iterator of values.
/// This trait should be used instead of Iterator for I/O backed streams,
/// since it can report error values separately from result values,
/// and it can be used with async functions and futures-based reactors.
trait Stream {
    type Item;
    type Error;
    /// Fetch the next batch of values.
    ///
    /// This function has six possible results:
    ///  * returns an empty slice, result is `Async::Pending` means the async operation is in progress
    ///  * returns an empty slice, result is `Async::Ready(None)` means the stream has terminated and `poll_next` should never be called again
    ///  * returns an empty slice, result is `Async::Ready(Some(_))` means there was a fatal error
    ///  * returns a non-empty slice, result is `Async::Pending` is a violation of the API contract and should lead to a panic
    ///  * returns a non-empty slice, result is `Async::Ready(None)` means that the underlying async operation is done, and the caller should call `poll_next` again immediately to get the rest of the data
    ///  * returns a non-empty slice, result is `Async::Ready(Some(_))` means that a fatal error occurred and the results should be ignored and dropped without further processing
    fn poll_next(&mut self, cx: &mut Context, result: &mut Async<Option<Error>>) -> [Item];
}
```

The requirements here are the same as Read:

* You need to be able to hand more than one value into some recipient, at once. Byte-at-a-time reading is not acceptable.
* You need to be able to implement the API using direct I/O, writing the result directly to wherever the consumer wants it written. Reading a Vec<> is not acceptable, though it seems easy enough to write an adapter that will convert to one. The goal here is to request a page and DMA directly into it (in this case, the implementation will probably have to use weird special-cases to fall back to software I/O if the slice isn't aligned).
* We want the same API to be usable for types other than `u8`.
* We need to be able to request our output buffer, scribble all over it, and then yield an error.
* It needs to allow infinite streams, so just returning a single raw slice and calling it done won't be enough.

That last point isn't so bad if you're just populating a Vec or other data structure that allows you to just append forever,
but many abstractions, like Serde, would actually have to directly support Streams themselves.

```rust
trait Serializer {
  /// This function uses an intermediate buffer by default,
  /// but third-party implementations can use a zero-copy implementation.
  /// The produced format is identical to using `serialize_bytes`.
  async fn serialize_byte_stream<S: Stream<Item=u8>>(self, stream: S) -> Result<Self::Ok, Self::Error> {
    let mut v = Vec::new();
    let mut e = Async::Ready(None);
    let mut prev_len = 1;
    while prev_len != v.len() {
      prev_len = v.len();
      // completely hand-waving the right way to await a stream,
      // by basically asking the stream for a Future<Output=()> and awaiting on it.
      await stream.unit();
      v.extend_from_raw_slice_with(|| stream.poll_next(&mut e));
    }
    if let Async::Ready(Some(e)) = e { return Err(e.into()); }
    self.serialize_bytes(&v[..])
  }
}
```
</details>

## Named Return Value Optimization

<details>In this context, NRVO refers to the ability to write code like:

```rust
let x = initial_state;
f(&mut x);
return x
```

and have it be guaranteed to never copy x.

Some use cases:

- Creating a trait object, calling some of its methods, then returning it.
- Creating a large zero-initialized array, setting a few of its elements, then returning it.
- Creating intermediate variables storing large/unsized objects that are part of a future return, without copying them around.

A future proposal implementing NRVO would need to address the following problems:

- Which patterns do or do not trigger NRVO.
- How to give the user feedback at to when NRVO is triggered.
- Whether to apply NRVO to multiple variables in the same function, eg:

```rust
// Should not be copied
let giantStruct1 = GiantStruct::new();
// Should not be copied either
let giantStruct2 = GiantStruct::new();
return (giantStruct1, giantStruct2);
```

### Lazy parameters

Lazy parameters would be a more powerful and wide-reaching feature, mirroring GCE and NRVO to streamline emplacement.

Instead of writing:

```rust
impl<T> Vec<T> {
    fn push_with<F: FnOnce() -> T>(&mut self, f: F);
}

some_vec.push_with(|| create_large_data(params));
```

the developer would write:

```rust
impl<T> Vec<T> {
    fn push(&mut self, lazy value: T);
}

some_vec.push(create_large_data(params));
```

which the compiler would silently rearrange, either by creating a closure, or by splitting `Vec::push` in two, and reserving space for `create_large_data` in the first half.

Both the main advantage and the main weakness of this feature is that it would be silent: on one hand, this obfuscates what really happens to the user; on the other hand, it would avoid API duplication; library developers wouldn't need to have `do_thing`, `do_thing_with`, and `do_thing_with_result` methods for every feature where they want emplacement. And emplacement would become the default, bringing performance improvements to everyone for cheap.

### Implicit reordering

If we want to push the concept of NRVO and lazy parameters to its extreme conclusion, we could also allow the compiler to silently reorder statements to guarantee zero-copy emplacement in situations where a variable is created before the data it's supposed to be emplaced in.

For instance, the following code:

```rust
some_vec.push_with(|| {
    let x = create_x();

    doThings(&mut x);
    doMoreThings(&mut x);

    x
});
```

could be replaced with:

```rust
let x = create_x();

doThings(&mut x);
doMoreThings(&mut x);

some_vec.push(x);
```

while still eliding copies.

The second code has the advantage of being easier to follow semantically. Values are created, set and mutated before being used, with no nesting involved.

On the other hand, the caveats mentionned with lazy arguments apply again, except on fire. This feature would essentially involve the compiler lying to the developer about how their code executes. This is nothing new in the context of compiler optimizations, but in the context of language semantics, it's not something to be considered lightly.
</details>
