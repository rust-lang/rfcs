- Feature Name: `field_projection`
- Start Date: 2022-09-10
- RFC PR: [rust-lang/rfcs#3318](https://github.com/rust-lang/rfcs/pull/3318)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce ways to refer to fields of structs via the type system.

# Motivation
[motivation]: #motivation

Accessing field information is at the moment only possible for macros. Allowing the type system to also access some information about fields enables writing code that generalizes over fields.
One important application is field projection. Rust often employs the use of wrapper types, for example `Pin<P>`, `NonNull<T>`, `Cell<T>`, `UnsafeCell<T>`, `MaybeUninit<T>` and more. These types provide additional properties for the wrapped type and often also logically affect their fields. For example, if a struct is uninitialized, its fields are also uninitialized. Giving the type system access to field information allows creating safe projection functions.

Current projection functions cannot be safe, since they take a projection closure that might execute arbitrary code. They also cannot automatically uphold type invariants of the projected struct. A prime example is `Pin`, the projection functions are `unsafe` and accessing fields is natural and often required. This leads to code littered with `unsafe` projections:
```rust
struct RaceFutures<F1, F2> {
    fut1: F1,
    fut2: F2,
}

impl<F1, F2> Future for RaceFutures<F1, F2>
where
    F1: Future,
    F2: Future<Output = F1::Output>,
{
    type Output = F1::Output;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match unsafe { self.as_mut().map_unchecked_mut(|t| &mut t.fut1) }.poll(ctx) {
            Poll::Pending => {
                unsafe { self.map_unchecked_mut(|t| &mut t.fut2) }.poll(ctx)
            }
            rdy => rdy,
        }
    }
}
```
Since the supplied closures are only allowed to do field projections, it would be natural to add `SAFETY` comments, but that gets even more tedious.

This feature is one important piece in the puzzle of safe and ergonomic pin projections. While it is not sufficient by itself, it enables experimentation with proc-macro-based implementations to solve the rest of the puzzle. Additionally this feature paves the way for general custom field projection which is useful for the following situations:
- volatile-only memory access (see example below),
- accessing fields of structs inside of raw pointers, `NonNull<T>`, `Cell<T>`, `UnsafeCell<T>`, `MaybeUninit<T>`,
- RCU interactions with locks (see [appendix][#rcu-interactions-with-locks]),


## RFC History

This RFC went through a couple of iterations and changed considerably from the initial proposal. The problem that the author intended to solve were `Pin` projections. In the Rust support for the Linux kernel, lots of types are self referential, because they contain circular, intrusive doubly linked lists. These lists have to be pinned, since list elements own pointers to the next and previous elements. Other datastructures are also implemented this way. Overall this results in most types having to be pinned and thus we have to deal with `Pin<&mut T>` constantly. Whenever one wants to access a field, they have to use the `unsafe` projection functions.

The currently preferred solution for this problem from the Rust ecosystem are [pin-project] and [pin-project-lite]. These are however unsuitable for use in the kernel. 
- [pin-project] cannot be used, since it requires `syn` and that is currently not used by the kernel which would add over 50k lines of Rust code.
- [pin-project-lite] does not depend on `syn`, but it has other problems: error messages are not useful, some use cases are not supported. Additionally the declarative macro is very complex and would be difficult to maintain.

Also, these solutions require the user to write `let this = self.project();` at the beginning of every function where one wants to project.

While exploring this problem, the Rust-for-Linux team discovered that they would like to use custom projections for using RCU together with locks. The author also discovered that these projections could be useful for other types.

The very first design only supported transparent wrapper types (i.e. only with a single field). The current version is the most general and minimal of all of the earlier designs.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Field Information

When defining a struct, the compiler automatically creates types for each field. This allows referencing fields via the type system. For example when we define the following struct:
```rust
struct Problem {
    info: String,
    count: usize,
}
```
Then the compiler creates two types, one for `info` and one for `count`. We cannot name these types normally, instead we use the `field_of!` macro:
```rust
type ProblemInfo<'a> = field_of!(Problem, info);
```
This field type also implements the `Field` trait that cannot be manually implemented. This trait provides some information about the field:
- which struct it belongs to,
- its own type,
- the offset at which the field can be found inside of the struct.

This is also generated for tuples, so you can use `field_of!((i32, u32), 0)` to get the type describing the `i32` element.

Since the `Field` trait cannot be implemented manually, you can be sure that a type implementing it actually refers to a field:
```rust
fn get_field<F: Field<Base = Problem>>(problem: &Problem) -> &F::Type {
    let ptr: *const Problem = problem;
    // SAFETY: `F` implements the `Field` trait and thus we find `F::Type` at `F::OFFSET` inside
    // of `ptr` that was derived from a reference.
    unsafe { &*ptr.cast::<u8>().add(F::OFFSET).cast::<F::Type>() }
}
```
There are a lot more powerful things that one can do using this type. For example field projections can be expressed safely. If we are often working with memory that has to be accessed volatile, then we might write the following wrapper type:
```rust
/// A pointer to memory that enforces volatile access.
pub struct VolatileMem<T> {
    ptr: NonNull<T>,
}

impl<T: Copy> VolatileMem<T> {
    pub fn get(&self) -> T {
        // SAFETY: `ptr` is always valid for volatile reads.
        unsafe { ptr::read_volatile(self.ptr.as_ptr()) }
    }

    pub fn put(&mut self, val: T) {
        // SAFETY: `ptr` is always valid for volatile writes.
        unsafe { ptr::write_volatile(self.ptr.as_mut_ptr()) }
    }
}
```
Now consider the following struct that we would like to put into our `VolatileMem<T>`:
```rust
#[repr(C)]
pub struct Config {
    mode: u8,
    reserved: [u8; 128],
}
```
If we want to write a new config, then we always have to write the whole struct, including the `reserved` field that is comparatively big. We can avoid this by providing a field projection:
```rust
impl<T> VolatileMem<T> {
    pub fn map<F: Field<Base = T>>(self) -> VolatileMem<F::Type> {
        Self {
            // SAFETY: `F` implements the `Field` trait and thus we find `F::Type` at `F::OFFSET`
            // inside of `ptr` that is always valid.
            ptr: unsafe {
                NonNull::new_unchecked(
                    self.ptr.as_ptr().cast::<u8>().add(F::OFFSET).cast::<F::Type>(),
                )
            },
        }
    }
}
```
Now in the scenario from above we can do:
```rust
let mut config: VolatileMem<Config> = ...;
config.put(Config::default());
let mut mode: VolatileMem<u8> = config.map::<field_of!(Config, mode)>();
mode.put(1);
```
And we will not have to always overwrite `reserved` with the same data.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

For every field of every non-packed struct and tuple, the compiler creates a unique, unnameable type that represent that field. These generated types will:
- have meaningful names in error messages (e.g. `Struct::field`, `(i32, u32)::0`), 
- implement the `Field` with accurate associated types and constants.

The `Field` trait will reside in `core::marker` and is:
```rust
/// A type representing a field on a struct.
pub trait Field {
    /// The type (struct or tuple) containing this field.
    type Base;
    /// The type of this field.
    type Type;
    /// The offset of this field from the beginning of the `Base` type in bytes.
    const OFFSET: usize;
}
```
This trait cannot be implemented manually and users are allowed to rely on the associated types/constants to be correct. For example the following code is allowed:
```rust
fn project<F: Field>(base: &F::Base) -> &F::Type {
    let ptr: *const Base = base;
    let ptr: *const u8 = base.cast::<u8>();
    // SAFETY: `ptr` is derived from a reference and the `Field` trait is guaranteed to contain
    // correct values. So `F::OFFSET` is still within the `F::Base` type.
    let ptr: *const u8 = unsafe { ptr.add(F::OFFSET) };
    let ptr: *const F::Type = ptr.cast::<F::Type>();
    // SAFETY: The `Field` trait guarantees that at `F::OFFSET` we find a field of type `F::Type`.
    unsafe { &*ptr }
}
```

Importantly, the `Field` trait should only be implemented on fields of non-`packed` types, since otherwise the above code would not be sound.

Another restriction is that unsized fields have dynamic offsets and thus cannot be statically known. So these fields types do not implement the `Field` trait, but the compiler generated type for the field still exists.

Users will be able to name this type by invoking the compiler built-in macro `field_of!` residing in `core`. This macro takes a type and an identifier/number for the accessed field:
```rust
macro_rules! field_of {
    ($struct:ty, $field:tt) => { /* compiler built-in */ }
}
```
Generics of the type have to be specified and the field has to be accessible by the calling scope:
```rust
pub mod inner {
    pub struct Foo<T> {
        a: usize,
        pub b: T,
    }
    type Ty = field_of!(Foo, a); // Compile error: missing generics for struct `Foo`
    type Ty = field_of!(Foo<()>, a); // OK
    type Ty = field_of!(Foo::<()>, b); // OK
    type Ty = field_of!(Foo<()>, c); // Compile error: no field `c` on type `Foo<()>`
}
type Ty = field_of!(Foo<()>, a); // Compile error: field `a` of struct `inner::Foo` is private
type Ty = field_of!(Foo<()>, b); // OK
type Ty<T> = field_of!(Foo<T>, b); // OK
type Ty<T> = field_of!((T, T, i32), 1); // OK
```

# Drawbacks
[drawbacks]: #drawbacks

Adds additional complexity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The presented approach is designed to be minimal and extendable. The `Field` trait can be extended and additional information such as the projection output can be added.

The `field_of!` macro avoids adding special syntax to refer to a field of a type and while it is not ergonomic, this can be changed by adding syntax later.


# Prior art
[prior-art]: #prior-art

## Crates

There are some crates that enable field projections via (proc-)macros:

- [pin-project] provides pin projections via a proc macro on the type specifying the structurally pinned fields. At the projection-site the user calls a projection function `.project()` and then receives a type with each field replaced with the respective projected field.
- [field-project] provides pin/uninit projection via a macro at the projection-site: the user writes `proj!($var.$field)` to project to `$field`. It works by internally using `unsafe` and thus cannot pin-project `!Unpin` fields, because that would be unsound due to the `Drop` impl a user could write.
- [cell-project] provides cell projection via a macro at the projection-site: the user writes `cell_project!($ty, $val.$field)` where `$ty` is the type of `$val`. Internally, it uses unsafe to facilitate the projection.
- [pin-projections] provides pin projections, it differs from [pin-project] by providing explicit projection functions for each field. It also can generate other types of getters for fields. [pin-project] seems like a more mature solution.
- [project-uninit] provides uninit projections via macros at the projection-site uses `unsafe` internally.
- [field-projection] is an experimental crate that implements general field projections via a proc-macro that hashes the name of the field to create unique types for each field that can then implement traits to make different output types for projections.

## Other languages

Java has reflection, which gives access to type information at runtime.

## RFCs
- [`ptr-to-field`](https://github.com/rust-lang/rfcs/pull/2708)

## Further discussion
- https://internals.rust-lang.org/t/cell-references-and-struct-layout/11564

[pin-project]: https://crates.io/crates/pin-project
[pin-project-lite]: https://crates.io/crates/pin-project-lite
[field-project]: https://crates.io/crates/field-project
[cell-project]: https://crates.io/crates/cell-project
[pin-projections]: https://crates.io/crates/pin-projections
[project-uninit]: https://crates.io/crates/project-uninit
[field-projection]: https://crates.io/crates/field-projection

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

## Use closures to improve ergonomics

One has to spell out the projected type for every projection. Closures could be used to make use of type inference where possible. We introduce a new closure type in `core::marker`:
```rust
pub trait FieldClosure<T>: Fn<T> {
    type Field: Field<Base = T>;
}
```
This trait is only implementable by the compiler. It is implemented for closures that
- do not capture variables
- only do a single field access on the singular parameter they have

Positive example: `|foo| foo.bar`
Negative examples:
- `|_| foo.bar`, captures `foo`
- `|foo| foo.bar.baz`, does two field accesses
- `|foo| foo.bar()`, calls a function
- `|foo| &mut foo.bar`, creates a reference to the field
- `|foo, bar| bar.baz`, takes two parameters

This trait makes calling a projection function a lot more ergonomic:
```rust
wrapper.project(|i| i.field)
// Instead of:
wrapper.project::<field_of!(Struct, field)>()
```
## Refer to fields via `Struct::field#foo`

We could make `Struct::field#foo` be equivalent to `field_of!(Struct, foo)`. In cases where there is no ambiguity, it would just be `Struct::foo`.

## Limited negative reasoning

There is the need to make the output type of `map` functions depend on properties of the projected field. In the case of `Pin`, this is whether the field is structurally pinned or not. If it is, then the return type should be `Pin<&mut F::Type>`, if it is not, then it should be `&mut F::Type` instead.

Negative reasoning would allow implementing the projection function with the correct type.

There have been some earlier RFCs about this topic. These cover more than is needed for making projections work:
- #1148
- #586

More information is also found in issue #1053.

## Operator syntax

Introduce a `Project` trait and a binary operator that is syntactic sugar for `Project::project($left, |f| f.$right)`. This would make projections even more ergonomic:
```rust
wrapper->field
// Instead of:
wrapper.project(|i| i.field)
// or
wrapper.project::<field_of!(Struct, field)>()
```

## Support misaligned fields and `packed` structs

Create the `MaybeUnalignedField` trait as a supertrait of `Field` with the constant `WELL_ALIGNED: bool`. This trait is also automatically implemented by the compiler even for packed structs.

## `enum` and `union` support

Both enums and unions cannot be treated like structs, since some variants might not be currently valid. This makes these fundamentally incompatible with the code that this RFC tries to enable. They could be handled using similar traits, but these would not guarantee the same things. For example, union fields are always allowed to be uninitialized.

If enums had variant types, then these variant types could be easily supported, as they are essentially just structs.

For unions we could add a supertrait of `Field` named `MaybeUninitField` that is implemented instead of the `Field` trait. Projection authors now can choose to allow these where it makes sense (e.g. `MaybeUninit`).

## Field macro attributes

To make things easier for implementing custom projections, we could create a new proc-macro kind that is placed on fields.

## Make the `Field` trait `unsafe` and implementable by users

We could make the `Field` trait `unsafe` and allow custom implementations.

# Appendix

## Field projections for Rust-for-Linux

### RCU interactions with locks

RCU (read-copy-update) is a special kind of synchronization mechanism used in the Linux kernel. It is mainly used with data structures based on pointers. These pointers need to be explicitly annotated (even in C). Further, the data structure must be constructed such that elements can be added/removed via a single atomic operation (e.g. swapping a pointer).

Readers and writers can access the data structure concurrently. Readers need to acquire the read lock before they read any RCU pointers. Writers can freely swap the pointers atomically, but when they want to free any objects that were in the data structure protected by RCU, they need to call `rcu_synchronize`. This function waits for all readers to relinquish any currently held locks, since then no reader can access the removed object. After `rcu_synchronize` returns, the writer can free the object.

The motivation for using RCU is performance. The way RCU is implemented in the kernel, acquiring the read lock is extremely cheap, it does not involve any atomics and in some cases even is a no-op. Additionally it never needs to wait. If writes are rare and reads are common, this improves performance significantly. To learn more about RCU, you can read [this](https://lwn.net/Articles/262464/) article.

Now onto a simple example that shows how using RCU could look like in Rust. In the example, we have a `Process` struct that contains a file descriptor table (fdt). This table is stored in a different allocation (via a `Box`) and the `Rcu` pointer wrapper struct marks that this pointer can only be accessed via RCU operations. Access to an instance of `Process` is serialized via a `Mutex`. But certain operations have to be executed very often and so locking and unlocking the mutex can get very expensive. In our case, we want to optimize fetching the current length of the FDT.
```rust
pub struct Process {
    fdt: Rcu<Box<FDT>>,
    id: usize,
    // other fields ...
}

pub struct FileDescriptorTable {
    len: usize,
    // other implementation details not important
}

impl Process {
    pub fn current() -> &'static RcuLock<Mutex<Process>> { todo!() }

    // Note the parameter type, an RcuLock is a lock wrapper providing
    // Rcu support for some lock types.
    pub fn get_fdt_len(self: &RcuLock<Mutex<Process>>) -> usize {
        // RcuLock only allows projections to fields of type `Rcu<T>`.
        let fdt: &Rcu<Box<FDT>> = RcuLock::project::<field_of!(Self, fdt)>(self);
        // Next we acquire the read lock.
        let rcu_guard: RcuGuard = rcu::read_lock();
        // To read an `Rcu` pointer, we need an `RcuGuard`.
        let fdt: &FDT = fdt.get(&rcu_guard);
        let len = fdt.len;
        // Dropping it explicitly ends the borrow of `fdt`.
        drop(rcu_guard);
        len
    }

    pub fn id(self: &RcuLock<Mutex<Process>>) -> usize {
        // When reading/writing a normal field, we have to use the `Mutex`:
        let guard: RcuLockGuard<MutexGuard<Process>> = self.lock();
        // We cannot give out `&mut` to `Rcu<T>` fields, since those are always
        // accessible immutably via the RcuLock projections, so we again have to
        // rely on projections here to guarantee soundness.
        let id: &mut usize = guard.project::<field_of!(Self, id)>();
        *id
    }

    pub fn replace_fdt(self: &RcuLock<Mutex<Process>>, new_fdt: Box<FDT>) {
        // We again obtain the `Rcu` pointer:
        let fdt: &Rcu<Box<FDT>> = RcuLock::project::<field_of!(Self, fdt)>(self);
        // When we want to overwrite an `Rcu` pointer, we can do so by just calling `set`:
        let old: RcuOldValue<Box<FDT>> = fdt.set(new_fdt);
        // Since readers might still be reading the old value (we have not yet called
        // `rcu_synchronize`) we must hold onto this object until we call `rcu_synchronize`.
        // However reading the old value is fine (i.e. `RcuOldValue<T>` implements `Deref<Target = T>`):
        let len = old.len;
        // But it does not implement `DerefMut`. On dropping an `RcuOldValue<T>`, `rcu_synchronize`
        // is called. But we can also get the old value out now:
        let old: Box<FDT> = old.sync(); // this will call `rcu_synchronize`.
    }

    // When the caller already owns the Rcu lock, then we can give out our FDT. Note that
    // the lifetime of the returned reference is the same as the guard and not of `self`.
    pub fn get_fdt<'a>(self: &RcuLock<Mutex<Process>>, guard: &'a RcuGuard) -> &'a FDT {
        let fdt: &Rcu<Box<FDT>> = RcuLock::project::<field_of!(Self, fdt)>(self);
        fdt.get(&guard)
    }
}
```
