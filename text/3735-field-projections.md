- Feature Name: `field_projection`
- Start Date: 2024-10-24
- RFC PR: [rust-lang/rfcs#3735](https://github.com/rust-lang/rfcs/pull/3735)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Field projections are a very general concept. In simple terms, it is a new operator that turns a
generic container type `C<T>` containing a struct `T` into a container `C<F>` where `F` is a
field of the struct `T`. For example given the struct:

```rust
struct Foo {
    bar: i32,
}
```

One can project from `&mut MaybeUninit<Foo>` to `&mut MaybeUninit<i32>` by using the new field
projection operator: 

```rust
impl Foo {
    fn initialize(this: &mut MaybeUninit<Self>) {
        let bar: &mut MaybeUninit<i32> = this->bar;
        bar.write(42);
    }
}
```

Special cases of field projections are [pin projections], or projecting raw pointers to fields
`*mut Foo` to `*mut i32` with improved syntax over `&raw mut (*ptr).bar`.

# Motivation
[motivation]: #motivation

Field projections are a unifying solution to several problems:
- [pin projections],
- ergonomic pointer-to-field access operations for pointer-types (`*const T`, `&mut MaybeUninit<T>`,
  `NonNull<T>`, `&UnsafeCell<T>`, etc.),
- projecting custom references and container types.

[Pin projections] have been a constant pain point and this feature solves them elegantly while at
the same time solving a much broader problem space. For example, field projections enable the
ergonomic use of `NonNull<T>` over `*mut T` for accessing fields.

In the following sections, we will cover the basic usage first. And then we will go over the most
complex version that is required for [pin projections] as well as allowing custom projections such
as the abstraction for RCU from the Rust for Linux project (also given below).

[pin projections]: https://doc.rust-lang.org/std/pin/index.html#projections-and-structural-pinning
[Pin projections]: https://doc.rust-lang.org/std/pin/index.html#projections-and-structural-pinning

## Ergonomic Pointer-to-Field Operations

We will use the struct from the summary as a simple example:

```rust
struct Foo {
    bar: i32,
}
```

References and raw pointers already possess pointer-to-field operations. Given a variable `foo: &T`
one can write `&foo.bar` to obtain a `&i32` pointing to the field `bar` of `Foo`. The same can be
done for `foo: *const T`: `&raw (*foo).bar` (although this operation is `unsafe`) and their mutable
versions.

However, the other pointer-like types such as `NonNull<T>`, `&mut MaybeUninit<T>` and
`&UnsafeCell<T>` don't natively support this operation. Of course one can write:

```rust
unsafe fn project(foo: NonNull<Foo>) -> NonNull<i32> {
    let foo = foo.as_ptr();
    unsafe { NonNull::new_unchecked(&raw mut (*foo).bar) }
}
```

But this is very annoying to use in practice, since the code depends on the name of the field and
can thus not be written using a single generic function. For this reason, many people use raw
pointers even though `NonNull<T>` would be more fitting. The same can be said about `&mut
MaybeUninit<T>`.

There are a lot of types that can benefit from this operation:
- `NonNull<T>`
- `*const T`, `*mut T`
- `&T`, `&mut T`
- `&Cell<T>`, `&UnsafeCell<T>`
- `&mut MaybeUninit<T>`, `*mut MaybeUninit<T>`
- `cell::Ref<'_, T>`, `cell::RefMut<'_, T>`
- `MappedMutexGuard<T>`, `MappedRwLockReadGuard<T>` and `MappedRwLockWriteGuard<T>`

## Pin Projections

The examples from the previous section are very simple, since they all follow the pattern `C<T> ->
C<F>` where `C` is the respective generic container type and `F` is a field of `T`.

In order to handle `Pin<&mut T>`, the return type of the field projection operator needs to depend
on the field itself. This is needed in order to be able to project structurally pinned fields from
`Pin<&mut T>` to `Pin<&mut F1>` while simultaneously projecting not structurally pinned fields from
`Pin<&mut T>` to `&mut F2`.

Fields marked with `#[pin]` are structurally pinned field. For example, consider the following
future:

```rust
struct FairRaceFuture<F1, F2> {
    #[pin]
    fut1: F1,
    #[pin]
    fut2: F2,
    fair: bool,
}
```

One can utilize the following projections when given `fut: Pin<&mut FairRaceFuture<F1, F2>>`:
- `fut->fut1: Pin<&mut F1>`
- `fut->fut2: Pin<&mut F2>`
- `fut->fair: &mut bool`

Using these, one can concisely implement `Future` for `FairRaceFuture`:

```rust
impl<F1: Future, F2: Future<Output = F1::Output>> Future for FairRaceFuture<F1, F2> {
    type Output = F1::Output;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let fair: &mut bool = self->fair;
        *fair = !*fair;
        if *fair {
            // self->fut1: Pin<&mut F1>
            match self->fut1.poll(ctx) {
                Poll::Pending => self->fut2.poll(ctx),
                Poll::Ready(res) => Poll::Ready(res),
            }
        } else {
            // self->fut2: Pin<&mut F2>
            match self->fut2.poll(ctx) {
                Poll::Pending => self->fut1.poll(ctx),
                Poll::Ready(res) => Poll::Ready(res),
            }
        }
    }
}
```

Without field projection, one would either have to use `unsafe` or reach for a third party library
like [`pin-project`] or [`pin-project-lite`] and then use the provided `project` function.

[`pin-project`]: https://crates.io/crates/pin-project
[`pin-project-lite`]: https://crates.io/crates/pin-project-lite

## Custom Projections

This proposal also aims to allow custom field projections. For example a custom pointer type for
"always valid pointers" i.e. mutable references that are allowed to alias and that have no
guarantees with respect to race conditions. Those would be rather annoying to use without field
projection, since one would always have to convert them into raw pointers to go to a field.

In this section, three examples are presented of custom field projections in the Rust for Linux
project. The first is volatile memory, a pointer that ensures only volatile access to the pointee.
The second is untrusted data, requiring validation before the data can be used for logic. And the
last example is a sketch for a safe abstraction (an API that provides only safe functions to use the
underlying feature) of RCU. It probably requires field projection in order to be able to provide
such a safe abstraction. Also note that this example requires to use field projection as pin
projections, so it is beneficial to read that section first.

### Rust for Linux Example: Volatile Memory

In the kernel, sometimes there is the need to solely access memory via volatile operations. Since
combining normal and volatile memory accesses will lead to undefined behavior, a safe abstraction is
required.

```rust
pub struct VolatileMem<T> {
    inner: *mut T,
}

impl<T> VolatileMem<T> {
    pub fn write(&mut self, value: T)
        // Required, since we can't drop the value
        where T: Copy
    {
        unsafe { std::ptr::write_volatile(self.inner, value) }
    }

    pub fn read(&self) -> T
        // Required, since we can't drop the value
        where T: Copy
    {
        unsafe { std::ptr::read_volatile(self.inner) }
    }
}
```

This design is problematic when `T` is a big struct and one is either only interested in reading a
single field or in modifying a single field.

```rust
#[derive(Clone, Copy)]
struct Data {
    x: i64,
    y: u64,
    /* imagine lots more fields */
}

let data: VolatileMem<Data> = /* ... */;
data.write(Data { /* ... */ });

// later in the program

// we only want to change `x`, but have to first read and then write the entire struct.
let d = data.read();
data.write(Data { x: 42, ..d });
```

This is a big problem, also for correctness, since in some applications of volatile memory, the
value of `data` might change after the read, but before the write. Additionally it is very
inefficient, when the struct is very big.

Any projection operation would have to be `unsafe`, because the pointer stored in `VolatileMem` is a
raw pointer and there is no way to ensure that the resulting, user-supplied pointer points to a
field of the original value.

But with custom field projections, one could simply do this instead:

```rust
data->x.write(42);
```

### Rust for Linux Example: Untrusted Data

In the Linux kernel, data coming from hardware or userspace is untrusted. This means that the data
must be validated before it is used for *logic* inside the kernel. Copying it into userspace is fine
without validation, but indexing some structure requires to first validate the index.

For the exact details, see the [untrusted data patch
series](https://lore.kernel.org/rust-for-linux/20240925205244.873020-1-benno.lossin@proton.me/). It
introduces the `Untrusted<T>` type used to mark data as untrusted. Kernel developers are supposed to
validate such data before it is used to drive logic within the kernel. Thus this type prevents
reading the data without validating it first.

One use case of untrusted data will be ioctls. They were discussed in version 1 in [this
reply](https://lore.kernel.org/rust-for-linux/ZvU6JQEN_92nPH4k@phenom.ffwll.local/) (slightly
adapted the code):
> Example in pseudo-rust:
> 
> ```rust
> struct IoctlParams {
>     input: u32,
>     ouptut: u32,
> }
> ```
> 
> The thing is that ioctl that use the struct approach like drm does, use the same struct if there's
> both input and output parameters, and furthermore we are not allowed to overwrite the entire
> struct because that breaks ioctl restarting. So the flow is roughly
> 
> ```rust
> let userptr: UserSlice;
> let params: Untrusted<IoctlParams>;
> 
> userptr.read(params);
> 
> // validate params, do something interesting with it params.input
> 
> // this is _not_ allowed to overwrite params.input but must leave it
> // unchanged
> 
> params.write(|x| { x.output = 42; });
> 
> userptr.write(params);
> ```
> 
> Your current write doesn't allow this case, and I think that's not good enough. The one I propsed
> in private does:
> 
> ```rust
> Untrusted<T>::write(&mut self, impl Fn(&mut T))
> ```

Importantly, we would like to only overwrite the `output` field of the `IoctlParams` struct. This is
the exact pattern that field projections can help with, instead of exposing a mutable reference to
the untrusted data via the `write` function, we can have:

```rust
impl<T> Untrusted<T> {
    fn write(&mut self, value: T);
}
```

In addition to allowing projections of `&mut Untrusted<IoctlParams>` to `&mut Untrusted<u32>`, thus
allowing to overwrite parts of a struct with field projections.

### Rust for Linux Example: RCU

RCU stands for read, copy, update. It is a creative locking mechanism that is very efficient for
data that is seldomly updated, but read very often. Below you can find a small summary of how I
understand it to work. No guarantees that I am 100% correct, if you want to make sure that you have
a correct understanding of how RCU works, please read the sources provided in the next section.

It requires quite a lot of explaining until I can express why field projection comes up in this
instance. However, in this case (similar to `Pin`) it is (to my knowledge) impossible to write a
safe API without field projections, so they would be invaluable for this use case.

#### RCU Explained

For a much more extensive explanation, please see <https://docs.kernel.org/RCU/whatisRCU.html>.
Since the first paragraph of the first section is invaluable in understanding RCU, it is quoted here
for the reader's convenience:

> The basic idea behind RCU is to split updates into “removal” and “reclamation” phases. The removal
> phase removes references to data items within a data structure (possibly by replacing them with
> references to new versions of these data items), and can run concurrently with readers. The reason
> that it is safe to run the removal phase concurrently with readers is the semantics of modern CPUs
> guarantee that readers will see either the old or the new version of the data structure rather
> than a partially updated reference. The reclamation phase does the work of reclaiming (e.g.,
> freeing) the data items removed from the data structure during the removal phase. Because
> reclaiming data items can disrupt any readers concurrently referencing those data items, the
> reclamation phase must not start until readers no longer hold references to those data items.

In C, RCU is used like this:
- the data protected by RCU sits behind a pointer,
- readers must use the [`rcu_read_lock()`](https://docs.kernel.org/RCU/whatisRCU.html#rcu-read-lock)
  and [`rcu_read_unlock()`](https://docs.kernel.org/RCU/whatisRCU.html#rcu-read-unlock) functions
  when accessing any data protected by RCU, within this critical section, blocking is forbidden.
- read accesses of the pointer must only be done after calling
  [`rcu_dereference(<pointer>)`](https://docs.kernel.org/RCU/whatisRCU.html#rcu-dereference).
- write accesses of the pointer must be done via [`rcu_assign_pointer(<old-pointer>,
  <new-pointer>)`](https://docs.kernel.org/RCU/whatisRCU.html#rcu-assign-pointer).
- before a writer frees the old value (i.e. it enters into the reclamation phase), they must call
  [`synchronize_rcu()`](https://docs.kernel.org/RCU/whatisRCU.html#synchronize-rcu).
- multiple writers **still require** some other kind of locking mechanism.

`synchronize_rcu()` waits for all existing read-side critical sections to complete. It does not have
to wait for new read-side critical sections that are begun after it has been called.

The big advantage of RCU is that in certain kernel configurations, (un)locking the RCU read lock is
achieved with absolutely no instructions.

#### A Safe Abstraction for RCU

In Rust, we will of course use a guard for the RCU read lock, so we have:

```rust
mod rcu {
    pub struct RcuGuard(/* ... */);

    impl Drop for RcuGuard { /* ... */ }

    pub fn read_lock() -> RcuGuard;
}
```

The pointers that are protected by RCU must be specially tagged, so we introduce the `Rcu` type. It
exposes the Rust equivalents of `rcu_dereference` and `rcu_assign_pointer` [^1]:

[^1]: Note that the requirement of not blocking in a critical RCU section is not expressed in code.
    Instead we use an external tool called [`klint`] for that purpose.

[`klint`]: https://rust-for-linux.com/klint

```rust
mod rcu {
    pub struct Rcu<P> {
        inner: UnsafeCell<P>,
        // we require this to opt-out of uniqueness of `&mut`.
        // if `UnsafePinned` were available, we would use that instead.
        _phantom: PhantomPinned,
    }
    
    impl<P: Deref> Rcu<P> {
        pub fn read<'a>(&'a self, _guard: &'a RcuGuard) -> &'a P::Target;
        pub fn set(self: Pin<&mut Self>, new: P) -> Old<P>;
    }

    pub struct Old<P>(/* ... */);
    
    impl<P> Drop for Old<P> {
        fn drop() {
            unsafe { bindings::synchronize_rcu() };
        }
    }
}
```

The `Old` type is responsible for calling `synchronize_rcu` before dropping the old value.

Note that `set` takes a pinned mutable reference to `Rcu`. This is important, since it might not be
obvious why there is pinning involved here. Firstly, we need to take a mutable reference, since
writers still need to be synchronized. Secondly, since there are still concurrent shared references,
we must not allow users to use `mem::swap`, since that would change the value without the required
compiler and CPU barriers in place.

Now to the crux of the issue and why field projection comes up here: A common use-case of RCU is to
protect data inside of a struct that is itself protected by a lock. Since the data is protected by
RCU, we don't need to hold the lock to read the data. However, locks do not allow access to the
inner value without locking it (that's kind of their whole point...). So we need a way to get to the
`Rcu<P>` without locking the lock. Using field projection, we would allow projections for fields of
type `Rcu` from `&Lock` to `&Rcu<P>`.

This way, readers can use field projection and the `Rcu::read` function and writers can continue to
lock the lock and then use `Rcu::set`.

#### RCU API Usage Examples

```rust
struct BufferConfig {
    flush_sensitivity: u8,
}

struct Buffer {
    // We also require `Rcu` to be pinned, because `&mut Rcu` must not exist (otherwise one could
    // call mem::swap).
    #[pin]
    cfg: Rcu<Box<BufferConfig>>,
    buf: Vec<u8>,
}

struct MyDriver {
    // The `Mutex` in the kernel needs to be pinned.
    #[pin]
    buf: Mutex<Buffer>,
}
```

Here the struct that is protected by the lock is `Buffer` and the data that is protected by RCU
inside of this struct is `BufferConfig`. To read the config, we now don't have to lock the lock,
instead we can read it using field projection:

```rust
impl MyDriver {
    fn buffer_config<'a>(&'a self, rcu_guard: &'a RcuGuard) -> &'a BufferConfig {
        let buf: &Mutex<Buffer> = &self.buf;
        // Here we use the special projections set up for `Mutex` with fields of type `Rcu<T>`.
        let cfg: &Rcu<Box<BufferConfig>> = buf->cfg;
        cfg.read(rcu_guard)
    }
}
```

To set the buffer config, one has to hold the lock:

```rust
impl MyDriver {
    fn set_buffer_config(&self, flush_sensitivity: u8) {
        // Our `Mutex` pins the value.
        let mut guard: Pin<MutexGuard<'_, Buffer>> = self.buf.lock();
        let buf: Pin<&mut Buffer> = guard.as_mut();
        // We can use pin-projections since we marked `cfg` as `#[pin]`
        let cfg: Pin<&mut Rcu<Box<BufferConfig>>> = buf->cfg;
        cfg.set(Box::new(BufferConfig { flush_sensitivity }));
        // ^^ this returns an `Old<Box<BufferConfig>>` and runs `synchronize_rcu` on drop.
    }
}
```

And of course one can still use other fields normally, but now requires field projection, since
`Pin<&mut T>` is involved:

```rust
impl MyDriver {
    fn read_to_buffer(&self, data: &[u8]) -> Result {
        let mut buf: Pin<Guard<'_, Buffer, MutexBackend>> = self.buf.lock();
        // This method allocates, so it must be fallible.
        // `buf.as_mut()->buf` again uses the field projection for `Pin` to yield a `&mut Vec<u8>`.
        buf.as_mut()->buf.extend_from_slice(data)
    }
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Rust Book Chapter: Field Projections

When programming in Rust, one often has the need to access only a single field of a struct. In the
usual cases of `&T` or `&mut T`, this is simple. Just use dot syntax and you can create a reference
to a field of the struct `&t.field`.

However, when one has a different type that "contains" or "points" at a `T`, one has to reach for
*field projections* via the *field projection operator* `->`. In this chapter, we will learn what
field projections are and how to use them for the most common types from the standard library. For
example for pointer-like types and [pin projections].

### Simple Uses of Field Projections

Let's say we have a big struct that doesn't fit onto the stack:

```rust
struct Data {
    flags: u32,
    buf: [u8; 1024 * 1024],
}
```

We would like to initialize the bytes in `buf` to `0xff` and `flags` should be `0x0f`. We start with
a new function returning memory on the heap:

```rust
impl Data {
    fn new() -> Box<Data> {
        let mut data = Box::new_uninit();
        {
            let data: &mut MaybeUninit<Data> = &mut *data;
```

Now we can use field projection to turn `&mut MaybeUninit<Data>` into `&mut MaybeUninit<u32>` that
points to the `flags` field:

```rust
            let flags: &mut MaybeUninit<u32> = data->flags;
            flags.write(0x0f);
```

And to initialize `buf`, we can do the same:

```rust
            let buf: &mut MaybeUninit<[u8]> = data->buf;
            let buf: &mut [MaybeUninit<u8>] = MaybeUninit::slice_as_bytes_mut(buf);
            MaybeUninit::fill(buf, 0xff);
        }
```

Now we only need to unsafely assert that we initialized everything.

```rust
        unsafe { data.assume_init() }
    }
}
```

A more general explanation of field projection is that it is an operation that turns a generic
container type `C<T>` containing a struct `T` into a container `C<F>` where `F` is a field of the
struct `T`.

#### Raw Pointers

Similarly to `&mut MaybeUninit<T>`, raw pointers also support projections. Given a raw pointer
`ptr: *mut Data`, one can use field projection to obtain a pointer to a field:
`ptr->flags: *mut u32`. Essentially `ptr->field` is a shorthand for `&raw mut (*ptr).field` (for
`*const` the same is true except for the `mut`). However, there is a small difference between the
two: the latter has to be `unsafe`, since `*ptr` requires that `ptr` be dereferencable. But field
projection is a safe operation and thus it uses [`wrapping_add`] under the hood. This is less
efficient, as it prevents certain optimizations. If that is a problem, either use `&raw [mut]
(*ptr).field` or create a custom pointer type that represents an always dereferencable pointer and
implement field projections using `unsafe`.

[`wrapping_add`]: https://doc.rust-lang.org/std/primitive.pointer.html#method.wrapping_add

Another pointer type that supports field projection is `NonNull<T>`. For example, if we had to add a
function that sets the `flags` field given only a `NonNull<Data>`, we could do so:

```rust
impl Data {
    unsafe fn set_flags_raw(this: NonNull<Self>, flags: u32) {
        let ptr: NonNull<u32> = this->flags;
        unsafe { ptr.write(flags) };
    }
}
```

#### `RefCell`'s References

Even the "exotic" references of `RefCell<T>` i.e. [`cell::Ref<'_, T>`] and [`cell::RefMut<'_, T>`]
are supporting field projection.

[`cell::Ref<'_, T>`]: https://doc.rust-lang.org/std/cell/struct.Ref.html
[`cell::RefMut<'_, T>`]: https://doc.rust-lang.org/std/cell/struct.RefMut.html

In this example, we create a buffer that tracks the various operations done to it for debug
purposes.

```rust
struct Buffer<T> {
    stats: RefCell<Stats>,
    buf: VecDeque<T>,
}

struct Stats {
    ops: Vec<Operation>,
    elements_pushed: usize,
    elements_popped: usize,
}
```

There are three operations, one for pushing a number of elements, one other for popping them and the
last one for peeking at the elements in the buffer.

```rust
enum Operation {
    Push(usize),
    Pop(usize),
    Peek,
}
```

When pushing and popping, we have a mutable reference to the buffer and could just use `Stats`
without the `RefCell`. But in the peek case, we only have a shared reference and still require to
record the statistic.

Pushing and popping are very simple:

```rust
impl<T> Buffer<T> {
    fn push(&mut self, items: &[T])
    where
        T: Clone,
    {
        let mut stats = self.stats.borrow_mut();
        stats.ops.push(Operation::Push(items.len()));
        stats.elements_pushed += items.len();
        self.buf.extend(items.iter().cloned());
    }

    fn pop(&mut self, count: usize) -> Option<Box<[T]>> {
        let count = count.min(self.buf.len());
        let mut stats = self.stats.borrow_mut();
        stats.ops.push(Operation::Pop(count));
        stats.elements_popped += count;
        if count == 0 {
            return None;
        }
        let mut res = Box::new_uninit_slice(count);
        for i in 0..count {
            let Some(val) = self.buf.pop_front() else {
              // we took the minimum above.
              unreachable!()
            };
            res[i].write(val);
        }
        Some(unsafe { res.assume_init() })
    }
}
```

Peeking also is rather easy:

```rust
impl<T> Buffer<T> {
    fn peek(&self) -> Option<&T> {
        self.stats.borrow_mut().ops.push(Operation::Peek);
        self.buf.front()
    }
}
```

Now we come to the part where we need field projections. We would like to be able to access the
operation statistics from other code. But because it is wrapped in `RefCell`, we cannot give a
reference out:

```rust
impl<T> Buffer<T> {
    fn stats(&self) -> &Vec<Operation> {
// error[E0515]: cannot return value referencing temporary value
        &self.stats.borrow().ops
//      ^-------------------^^^^
//      ||
//      |temporary value created here
//      returns a value referencing data owned by the current function
    }
}
```

That is because the value returned by `borrow` is placed on the stack and must be kept alive for
bookkeeping purposes of `RefCell` until the borrow ends. But using field projection, we can return a
`cell::Ref`:

```rust
impl<T> Buffer<T> {
    fn stats(&self) -> cell::Ref<'_, Vec<Operation>> {
        self.stats.borrow()->ops
    }
}
```

We could even hide the fact that the stats are implemented using `RefCell` using an opaque type:

```rust
impl<T> Buffer<T> {
    fn stats(&self) -> impl Deref<Target = Vec<Operation>> + '_ {
        self.stats.borrow()->ops
    }
}
```

### Complicated Field Projections

Field projection is even more powerful than what we have seen until now. The returned type of the
projection operator can even depend on the field itself!

This enables them to be used for making [pin projections] ergonomic. We will discuss how to use
this way of pin projection in the next section.

#### Pin Projections

For this section, you should understand what [pin projections] are. If not, then you can just skip
this section.

Structurally pinned fields are marked with `#[pin]` using the derive macro `PinProject`. For example
consider a future that alternatingly polls two futures:

```rust
#[derive(PinProject)]
struct FairRaceFuture<F1, F2> {
    #[pin]
    fut1: F1,
    #[pin]
    fut2: F2,
    fair: bool,
}
```

Now, it's possible to project a `fut: Pin<&mut FairRaceFuture>`:
- `fut->fut1: Pin<&mut F1>`
- `fut->fut2: Pin<&mut F2>`
- `fut->fair: &mut bool`

Using these, one can concisely implement `Future` for `FairRaceFuture` without any `unsafe` code:

```rust
impl<F1: Future, F2: Future<Output = F1::Output>> Future for FairRaceFuture<F1, F2> {
    type Output = F1::Output;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let fair: &mut bool = self->fair;
        *fair = !*fair;
        if *fair {
            // self->fut1: Pin<&mut F1>
            match self->fut1.poll(ctx) {
                Poll::Pending => self->fut2.poll(ctx),
                Poll::Ready(res) => Poll::Ready(res),
            }
        } else {
            // self->fut2: Pin<&mut F2>
            match self->fut2.poll(ctx) {
                Poll::Pending => self->fut1.poll(ctx),
                Poll::Ready(res) => Poll::Ready(res),
            }
        }
    }
}
```

### Implementing Custom Field Projections

There are two different ways to implement projections for a custom type:
- implementing the `Projectable` and `Project<F>` traits, or
- annotating it with `#[projecting]`.

They are used for pointers and container types respectively.

#### Pointer-Like

Pointer-like types can be projected using `Projectable` and `Project<F>`. For example, if we create
a custom reference type that simultaneously points at two instances of the same type.

```rust
pub struct DoubleRef<'a, T> {
    first: &'a T,
    second: &'a T,
}

impl<'a, T> DoubleRef<'a, T> {
    pub fn new(first: &'a T, second: &'a T) -> Self {
        Self { first, second }
    }

    pub fn read(&self) -> (T, T)
    where
        T: Copy
    {
        (*self.first, *self.second)
    }
}
```

We can now allow its users to use field projection by implementing the above mentioned traits:

```rust
impl<'a, T> Projectable for DoubleRef<'a, T> {
    type Inner = T;
}
```

The `Projectable` trait is what tells the compiler which types' fields to consider for projecting.
When you write `a->b`, then `a` has to implement `Projectable` in order for the compiler to know
which type to look up the field `b` for.

The actual projection operation is governed by `Project<F>`:

```rust
impl<'a, T, F> Project<F> for DoubleRef<'a, T>
where
    F: Field<Base = T>,
{
    type Output = DoubleRef<'a, F::Type>;

    fn project(self) -> Self::Output {
        DoubleRef {
            first: self.first.project(),
            second: self.second.project(),
        }
    }
}
```

The `Project<F>` trait governs projections for fields represented by the field type `F`. These field
types are generated for all[^3] fields of all structs. A field type always implements the
`UnalignedField` trait:

[^3]: Almost all, fields that don't have a size, are not included.

```rust
pub unsafe trait UnalignedField {
    type Base: ?Sized;
    type Type: ?Sized;
    const OFFSET: usize;
}
```

`Base` is set to the parent struct containing the field, `Type` is set to the type of the field
itself and `OFFSET` is the offset in bytes as returned by `offset_of!`.


With the above projections in place, users can write the following code:

```rust
struct Foo {
    bar: i32,
    baz: u32,
}

let x: &Foo = &Foo { bar: 42, baz: 43 };
let y: &Foo = &Foo { bar: 24, baz: 25 };
let d: DoubleRef<'_, Foo> = DoubleRef::new(x, y);
let bars: DoubleRef<'_, i32> = d->bar;
assert_eq!((42, 24), bars.read());
```

#### Simultaneous Projections

One important detail of the `Projectable` and `Project` traits is that they only enable support for
a single projection. So the value `x` will be consumed when doing `x->y`.

Simultaneous projections are governed by the `SimultaneousProjectable` and `SimultaneousProject`
traits. When projecting a value whose type implements these traits, it can be projected once for
each field. So if our `DoubleRef` implemented them, we could also check the value of the two `baz`
fields:

```rust
struct Foo {
    bar: i32,
    baz: u32,
}

let x: &Foo = &Foo { bar: 42, baz: 43 };
let y: &Foo = &Foo { bar: 24, baz: 25 };
let d: DoubleRef<'_, Foo> = DoubleRef::new(x, y);
let bars: DoubleRef<'_, i32> = d->bar;
let bazes: DoubleRef<'_, u32> = d->baz;
assert_eq!((42, 24), bars.read());
assert_eq!((43, 25), bazes.read());
```

Implementing these traits for `DoubleRef` looks like this:

```rust
impl<'a, T> SimultaneousProjectable for DoubleRef<'a, T> {
    type Inter = (*mut T, *mut T);

    fn start_projection(self) -> Self::Inter {
        (self.first, self.second)
    }
}

impl<'a, T, F> SimultaneousProject<F> for DoubleRef<'a, T>
where
    F: Field<Base = T>,
{
    type Output = DoubleRef<'a, F::Type>;

    unsafe fn project(inter: Self::Inter) -> Self::Output {
        DoubleRef {
            first: unsafe { &mut *inter.0.project() },
            second: unsafe { &mut *inter.1.project() },
        }
    }
}
```

A couple of important notes:
- we need to remove the `Project` implementation, since there is a blanket impl for types that
  implement `SimultaneousProject`.
- the `Inter` type has to implement `Clone` and the compiler will clone it for every projection the
  user requests.
- the compiler ensures that `project` will only be called at most once for each field via the
  projection operator.

#### Containers

The other kind of projections are that of *containers*, for example `UnsafeCell<T>` or
`MaybeUninit<T>`. They are governed by the `#[projecting]` attribute.

If we want to combine the two containers from the previous sections, we can do it in the following
way:

```rust
#[projecting]
#[repr(transparent)]
pub struct Opaque<T> {
    inner: UnsafeCell<MaybeUninit<T>>,
    // should be replaced by wrapping `inner` in `UnsafePinned` from
    // https://github.com/rust-lang/rfcs/pull/3467
    _phantom_pinned: PhantomPinned,
}

impl<T> Opaque<T> {
    pub fn new(value: T) -> Self {
        Self { inner: UnsafeCell::new(MaybeUninit::new(value)) }
    }

    pub fn uninit() -> Self {
        Self { inner: UnsafeCell::new(MaybeUninit::uninit()) }
    }

    pub fn write(&mut self, value: T) {
        self.inner.get_mut().write(value);
    }
}
```

Now `&Opaque<T>` can represent a reference that points at data from other languages, such as C.

The `#[projecting]` attribute changes the way the field types are generated for this type. Instead
of having a field type representing the `inner` field, this type will "project" through to the
generic parameter `T`, inheriting all fields that `T` has. So if we consider the type `Foo` from
above:

```rust
struct Foo {
    bar: i32,
    baz: u32,
}
```

Then there are two field types for `Opaque<Foo>`:
- one representing `bar` with `Base = Opaque<Foo>` and `Type = Opaque<i32>`
- the other representing `baz` with `Base = Opaque<Foo>` and `Type = Opaque<u32>`

So the both the base type and the type of the fields are wrapped with the container. Now users can
write:

```rust
fn init(foo: &mut Opaque<Foo>) {
    let bar: &mut Opaque<i32> = foo->bar;
    bar.write(42);
    foo->baz.write(24);
}
```

##### Limitations

A type can only be annotated with `#[projecting]` if it is also `#[repr(transparent)]`, because of
the following problem:

Assume that we could annotate the following container with `#[projecting]`:

```rust
#[projecting]
struct Container<T> {
    count: u64,
    value: T,
}
```

Now the memory layout of a `Container<Foo>` could look like this (one character represents one byte):

```text
|count---|bar-|baz-|
```

If we now want to project `&Container<Foo>` to `&Container<u32>`, we would have to project to `baz`.
But the problem is that the memory layout of `Container<u32>` looks like this:

```text
|count---|u32-|
```

Now projecting becomes impossible for two reasons:
- this layout is not contained as a direct sublayout of the above (ie with `count` mapped to `count`
  and `u32` mapped to `baz`),
- projecting for references is done simply via offsetting.

So is not possible to project to the correct layout with both of these constraints.

This is also the reason for why `Arc<T>` cannot be projected to `Arc<Field>` (the reference count is
stored in front of the `T`). However, it is possible to create an [`ArcRef<T>`](arcref) that
tracks the reference count separately from the field.

## Impact of this Feature

Overall this feature improves readability of code, because it replaces more complex to parse syntax
with simpler syntax:
- `&raw mut (*ptr).foo` is turned into `ptr->foo`
- using `NonNull<T>` as a replacement for `*mut T` becomes a lot better when accessing fields,

There is of course a cost associated with introducing a new operator along with the concept of field
projections.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Implementation Details

In order to facilitate field projections, several interlinked concepts have to be introduced. These
concepts are:

- [field types],
  - `Field` traits,
  - `field_of!` macro,
  - [`#[projecting]`](#projecting-attribute) attribute
- projection operator `->`,
  - `Project` trait,
  - `Projectable` trait,
  - `SimultaneousProject` trait,
  - `SimultaneousProjectable` trait,

To ease understanding, here is a short explanation of the interactions between these concepts. The
following subsections explain them in more detail, so refer to them in cases of ambiguity. The
projection operator `->` is governed by the `Project` trait that has `Projectable` as a super trait.
`Projectable` helps to select the struct whose fields are used for projection. Field types store
information about a field (such as the base struct and the field type) via the `UnalignedField`
trait and the `field_of!` macro makes it possible to name the [field types]. Finally, the
[`#[projecting]`](#projecting-attribute) attribute allows `repr(transparent)` structs to be ignored
when looking for fields for projection.

The traits `SimultaneousProject` and `SimultaneousProjectable` exist to support simultaneous
projections.

### Field Types
[field type]: #field-types
[field types]: #field-types

The compiler generates a compiler-internal type for every sized[^2] field of every struct and tuple.
These types can only be named via the `field_of!` macro that has the same syntax as `offset_of!`.
Only fields accessible to the current scope can be projected. These types are called *field types*.

[^2]: This restriction can be lifted in the future to include unsized types with statically known
    alignment, but that would have to be done in unison with adding support for those fields in
    `offset_of!`.

Field types implement the `UnalignedField` trait:

```rust
/// # Safety
///
/// In any instance of the type `Self::Base`, at byte offset `Self::OFFSET`, there exists a
/// (possibly misaligned) field of type `Self::Type`.
pub unsafe trait UnalignedField {
    type Base: ?Sized;
    type Type: ?Sized;

    const OFFSET: usize;
}
```

In the implementation of this trait, `Base` is set to the struct that the field is part of and
`Type` is set to the type of the field. `OFFSET` is set to the offset in bytes of the field in
the struct (i.e. `OFFSET = offset_of!(Base, ident)`).

For aligned fields (such as all fields of non-`#[repr(packed)]` structs), their field types also
implement the `Field` trait:

```rust
/// # Safety
///
/// In any well-aligned instance of the type `Self::Base`, at byte offset `Self::OFFSET`, there
/// exists a well-aligned field of type `Self::Type`.
pub unsafe trait Field: UnalignedField {}
```

In addition to all fields of all structs and tuples, field types are generated for
[`#[projecting]`](#projecting-attribute) container types as follows: given a type annotated with
[`#[projecting]`](#projecting-attribute) and a field contained in it that has itself field types:

```rust
#[projecting]
#[repr(transparent)]
pub struct Container<T> {
    inner: T,
}

struct Foo {
    bar: i32,
}
```

The type `Container<Foo>` inherits all fields of `Foo` with `Base` and `Type` adjusted accordingly
(i.e. wrapped by `Container`):

```rust
fn project<F: Field>(r: &F::Base) -> &F::Type;

let x: Container<Foo>;
let _: &Container<i32> = project::<field_of!(Container<Foo>, bar)>(&x);
```

The implementation of the `UnalignedField` trait sets the associated types and constant like this:
- `Base = Container<Foo>`
- `Type = Container<i32>`
- `OFFSET = offset_of!(Foo, bar)`

This can even be done for multiple levels: `Container<Container<Foo>>` also has a [field type] `bar`
of type `Container<Container<i32>>`. Mixing different container types is also possible.

Annotating a struct with [`#[projecting]`](#projecting-attribute) disables projection via that
structs own fields. Continuing the example from above:

```rust
struct Bar {}

// ERROR: `Container<Bar>` does not have a field `inner`. `Container<T>` is annotated with
// `#[projecting]` and thus the field types it exposes are changed to the wrapped type. `Bar` does
// not have a field `inner`.
type X = field_of!(Container<Bar>, inner);

struct Baz {
    inner: Foo,
}

// this refers to the field `inner` of `Baz`.
type Y = field_of!(Container<Baz>, inner);
// it has the following implementation of `UnalignedField`:
impl UnalignedField for Y {
    type Base = Container<Baz>;
    type Type = Container<Foo>;

    const OFFSET: usize = offset_of!(Baz, inner);
}
```

#### `Field` Traits

The fields trait are added to `core::marker` and cannot be implemented manually.

```rust
/// # Safety
///
/// In any instance of the type `Self::Base`, at byte offset `Self::OFFSET`, there exists a
/// (possibly misaligned) field of type `Self::Type`.
pub unsafe trait UnalignedField {
    type Base: ?Sized;
    type Type: ?Sized;

    const OFFSET: usize;
}

/// # Safety
///
/// In any well-aligned instance of the type `Self::Base`, at byte offset `Self::OFFSET`, there
/// exists a well-aligned field of type `Self::Type`.
pub unsafe trait Field: UnalignedField {}
```

The compiler automatically implements it for all [field types]. Users of the trait are allowed to rely
on the associated types and constants in `unsafe` code. So for example this piece of code is sound:

```rust
fn get_field<F: Field>(base: &F::Base) -> &F::Type
where
    // required to be able to `cast`
    F::Type: Sized,
    F::Base: Sized,
{
    let ptr: *const F::Base = base;
    let ptr: *const u8 = ptr.cast::<u8>();
    // SAFETY: `ptr` is derived from a reference and the `UnalignedField` trait is guaranteed to
    // contain correct values. So `F::OFFSET` is still within the `F::Base` type.
    let ptr: *const u8 = unsafe { ptr.add(F::OFFSET) };
    let ptr: *const F::Type = ptr.cast::<F::Type>();
    // SAFETY: The `Field` trait guarantees that at `F::OFFSET` we find a field of type `F::Type`.
    unsafe { &*ptr }
}
```

#### `field_of!` Macro

Also added to `core::marker` is the following built-in macro:

```rust
pub macro field_of($Container:ty, $($fields:expr)+ $(,)?) {
    /* built-in macro */
}
```

It has the same syntax as the `offset_of!` macro also supporting tuples. `field_of!` returns the
[field type] of the field `$fields` of the `$Container` struct or tuple type. It emits an error in
the following cases:

```rust
pub mod foo {
    pub struct Foo {
        bar: u32,
        pub baz: i32,
    }

    // Error: unknown field `barr` of type `Foo`
    type FooBar = field_of!(Foo, barr);
}

pub mod bar {
    // Error: field `bar` of type `Foo` is private
    type FooBar = field_of!(Foo, bar);
}
```

#### `#[projecting]` Attribute

The `#[projecting]` attribute can be put on a struct or union declaration. It requires that the type
is `#[repr(transparent)]` and there must be a unique non-zero-sized field (it is allowed to be
generic and thus not always non-zero-sized). Alternatively, it is allowed to be zero-sized, but then
must either have a single generic, or annotate the projected generic with `#[projecting]`.

So for example:

```rust
#[projecting]
#[repr(transparent)]
pub struct Container<T> {
    inner: T,
}

struct Foo {
    bar: i32,
}
```

Now `Container<Foo>` has a [field type] associated with `bar` implementing `Field` with:
- `Base = Container<Foo>`
- `Type = Container<i32>`
- `OFFSET = offset_of!(Foo, bar)`

Some more examples:

```rust
// This type is always zero-sized, but "contains" a field.
#[projecting]
#[repr(transparent)]
pub struct Container<T> {
    _phantom: PhantomData<T>,
}

#[projecting]
#[repr(transparent)]
pub struct Container<T> {
    // nesting multiple containers is fine.
    ctr: Container<Container2<T>>,
    // other zero-sized fields still are allowed:
    _variance: PhantomData<fn(T)>,
}

// multiple generics, but still only one field that is not always zero-sized.
#[projecting]
#[repr(transparent)]
pub struct Container<T, U> {
    inner: T,
    _phantom: PhantomData<U>,
}

// multiple generics and zero-sized
#[projecting]
#[repr(transparent)]
pub struct Container<#[projecting] T, U> {
    inner: PhantomData<T>,
    other: PhantomData<U>,
}
```

In the last two examples, if we're given `&Container<Foo, Bar>`, then the projection to `bar` has
the type `&Container<i32, Bar>`.

Here are some error examples:

```rust
// ERROR: missing `#[repr(transparent)]`
#[projecting]
pub struct Container<T> {
    inner: T,
}

// ERROR: no field to project onto found, the struct has no fields
#[projecting]
#[repr(transparent)]
pub struct Container {}

// ERROR: no generic type parameter found
#[projecting]
#[repr(transparent)]
pub struct Container {
    foo: Foo,
}

// ERROR: ambiguous projection generic
#[projecting]
#[repr(transparent)]
pub struct Container<T, U> {
    _phantom: PhantomData<(T, U)>,
}
```

### Field Projection Operator

The field projection operator `->` has the following syntax:

> **<sup>Syntax</sup>**\
> _ProjectionExpression_ :\
> &nbsp;&nbsp; [_Expression_] `->` _ProjectionMember_
>
> _ProjectionMember_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; [IDENTIFIER]
> &nbsp;&nbsp; | [TUPLE_INDEX]

[IDENTIFIER]: https://doc.rust-lang.org/reference/identifiers.html
[_Expression_]: https://doc.rust-lang.org/reference/expressions.html
[TUPLE_INDEX]: https://doc.rust-lang.org/reference/tokens.html#tuple-index

#### `[Simultaneous]Project[able]` Traits

The projection operator is governed by four traits added to `core::ops`:

```rust
pub trait Projectable: Sized {
    type Inner: ?Sized;
}

pub trait Project<F>: Projectable
where
    F: Field<Base = Self::Inner>,
{
    type Output;

    fn project(self) -> Self::Output;
}

// name-bikeshed needed
pub trait SimultaneousProjectable: Projectable {
    // name-bikeshed needed
    type Inter: Clone;

    fn start_projection(self) -> Self::Inter;
}

// name-bikeshed needed
pub trait SimultaneousProject<F>: SimultaneousProjectable
where
    F: Field<Base = Self::Inner>,
{
    type Output;

    /// # Safety
    ///
    /// This function may only be called once for each value of `Self::Inter` that is derived from
    /// a value of `Self` via [`SimultaneousProjectable::start_projection`] or cloning such a value.
    unsafe fn project(inter: Self::Inter) -> Self::Output;
}

impl<T, F> Project<F> for T
where
    T: SimultaneousProjectable<F>,
    F: Field<Base = T::Inner>,
{
    type Output = <T as SimultaneousProject<F>>::Output;

    fn project(self) -> Self::Output {
        // SAFETY: we only call one project function from the derived value of `start_projection`.
        unsafe { <T as SimultaneousProject<F>>::project(self.start_projection()) }
    }
}
```

`Project` is responsible for the actual projection operation while `Projectable` identifies if a
type has any kind of projection and for which fields there could be projections. So if the compiler
sees `x->y`, the type of `x` has to implement `Projectable` in order for the compiler to verify that
the associated type `Inner` of that impl has a field named `y`.

`SimultaneousProject` can be implemented instead of `Project` in order to allow projecting the same
expression for different fields at the same time. The `Inter` associated type of
`SimultaneousProjectable` is cloned for each such simultaneous projection (except the last).

#### Desugaring

When only a single projection operation using that variable is done, the desugaring is simpler. For
example:

```rust
struct T {
    field: F,
}

let t: C<T> = /* ... */;
let _ = t->field;

// becomes

let _ = Project::<field_of!(<C<T> as Projectable>::Inner, field)>::project_last(
   Projectable::start_projection(t),
);
```

The `C<T>` in the `C<T> as Projectable` comes from a type inference variable over the expression
`t`.

When the same projection base is used multiple times, the desugaring is as follows:

```rust
struct T {
    field: F,
    x: X,
    y: Y,
}

let t: C<T> = /* ... */;
let _ = t->field;
let _ = t->x;
let _ = t->y;

// becomes

let __inter_t = Projectable::start_projection(t);
let _ = unsafe {
    SimultaneousProject::<field_of!(<C<T> as Projectable>::Inner, field)>::project(
        __inter_t.clone()
    )
};
let _ = unsafe {
    SimultaneousProject::<field_of!(<C<T> as Projectable>::Inner, x)>::project(__inter_t.clone())
};
let _ = unsafe {
    SimultaneousProject::<field_of!(<C<T> as Projectable>::Inner, y)>::project(__inter_t)
};
```

Essentially, the compiler starts projecting the value and then re-uses the same `Inter` value for
the various projections, consuming it on the last one.

## Stdlib Field Projections

All examples from the guide-level explanation work when the standard library is extended with the
implementations detailed below.

The following pointer types get an implementation for `Projectable` with `Inner = T`. They support
projections for any field and perform the obvious offset operation.

- `*mut T`
- `*const T`
- `NonNull<T>`

The same is true for the following types, except that they only allow projecting aligned fields:

- `&T`, `&mut T`
- `cell::Ref<T>`, `cell::RefMut<T>`
- `MappedMutexGuard<T>`, `MappedRwLockReadGuard<T>` and `MappedRwLockWriteGuard<T>`


For example, `&T` would be implemented like this:

```rust
impl<'a, T: ?Sized> Projectable for &'a T {
    type Inner = T;
}

impl<'a, T: ?Sized> SimultaneousProjectable for &'a T {
    type Inter = *const T;

    fn start_projection(self) -> Self::Inter {
        self
    }
}

unsafe impl<'a, T: ?Sized, F> SimultaneousProject<F> for &'a T
where
    F: Field<Base = T>,
    // Needed to be able to `.cast` below
    F::Type: Sized + 'a,
{
    unsafe fn project(ptr: *const T) -> Self::Output {
        let ptr = ptr.cast::<u8>();
        let ptr = unsafe { ptr.add(F::OFFSET) };
        let ptr = ptr.cast::<F::Type>();
        unsafe { &*ptr }
    }
}
```

The following types get annotated with [`#[projecting]`](#projecting-attribute):

- `MaybeUninit<T>`
- `Cell<T>`
- `UnsafeCell<T>`
- `SyncUnsafeCell<T>`

### Pin Projections

In order to provide [pin projections], a new derive macro `PinProject` and a trait `PinField` is
required:

```rust
/// # Safety
///
/// - `Self::Projected` is set to either `&'a mut Self::Type` or `Pin<&'a mut Self::Type>`,
/// - `from_pinned_ref` must either be the identity function, or return the argument wrapped in
///   `Pin` (either with `Pin::new_unchecked` or `Pin::new`)
pub unsafe trait PinField: Field {
    type Projected<'a>;

    /// # Safety
    ///
    /// `r` must point at a field of the struct `Self::Base`. That struct value must be pinned.
    unsafe fn from_pinned_ref<'a>(r: &'a mut Self::Type) -> Self::Projected<'a>;
}
```

An example use is:

```rust
#[derive(PinProject)]
struct FairRaceFuture<F1, F2> {
    #[pin]
    fut1: F1,
    #[pin]
    fut2: F2,
    fair: bool,
}
```

It expands the above to:

```rust
struct FairRaceFuture<F1, F2> {
    fut1: F1,
    fut2: F2,
    fair: bool,
}

unsafe impl<F1, F2> PinField for field_of!(FairRaceFuture<F1, F2>, fut1) {
    type Projected<'a> = Pin<&'a mut F1>;

    fn from_pinned_ref<'a>(r: &'a mut F1) -> Pin<&'a mut F1> {
      unsafe { Pin::new_unchecked(r) }
    }
}

unsafe impl<F1, F2> PinField for field_of!(FairRaceFuture<F1, F2>, fut2) {
    type Projected<'a> = Pin<&'a mut F2>;

    fn from_pinned_ref<'a>(r: &'a mut F2) -> Pin<&'a mut F2> {
      unsafe { Pin::new_unchecked(r) }
    }
}

unsafe impl<F1, F2> PinField for field_of!(FairRaceFuture<F1, F2>, fut2) {
    type Projected<'a> = &'a mut bool;

    fn from_pinned_ref<'a>(r: &'a mut bool) -> &'a mut bool {
      r
    }
}
```

Now the only component that is left is an implementation of `Projectable` and `Project` for
`Pin<&mut T>`:

```rust
impl<'a, T: ?Sized> Projectable for Pin<&'a mut T> {
    type Inner = T;
}

impl<'a, T: ?Sized> SimultaneousProjectable for Pin<&'a mut T> {
    type Inter = *mut T;

    fn start_projection(self) -> Self::Inter {
        unsafe { Pin::into_inner_unchecked(self) }
    }
}
unsafe impl<'a, T, F> SimultaneousProject<F> for Pin<&'a mut T>
where
    F: UnalignedField<Base = T> + PinField,
{
    unsafe fn project(inter: Self::Inter) -> Self::Output {
        let r: *mut F::Type = <*mut T as Project<F>>::project(inter);
        let r = unsafe { &mut *r };
        <F as PinField>::from_pinned_ref(r)
    }
}
```

## Interactions

There aren't a lot of interactions with other features.

The projection operator binds very tightly:

```rust
*ctr->field = *(ctr->field);

&mut ctr->field = &mut (ctr->field);

ctr->field.foo() = (ctr->field).foo();

ctr.foo()->field = (ctr.foo())->field;

ctr->field->bar = (ctr->field)->bar;
```

# Drawbacks
[drawbacks]: #drawbacks

- [Pin projections] still require library level support via a proc macro and a trait solely for
  [field types].

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This proposal is a lot more general than just improving [pin projections]. It not only covers
pointer-like types, but also permits all sorts of operations generic over fields.

Not adding this feature will result in the proliferation of `*mut T` over more suitable pointer
types that better express the invariants of the pointer. The ergonomic cost of
`unsafe {  MyPtr::new_unchecked(&raw mut (*my_ptr.as_ptr()).field) }` is just too great to be useful
in practice.

While [pin projections] can be addressed via a library or a separate feature, not having them in the
language takes a toll on projects trying to minimize dependencies. The Rust for Linux project is
already using pinning extensively, since all locking primitives require it; a library solution will
never be as ergonomic as a language-level construct. Thus that project would benefit greatly from
this feature.

Additionally, safe RCU abstractions are likely impossible without field projections, since they
require being generic over the fields of structs.

Field projections are on first contact rather difficult to understand, especially the instantiation
as [pin projections]. However, they are a very natural operation, extending the already existent
features of raw pointers and references. Therefore they are fairly easy to adjust to; and in turn,
they provide a big increase in readability of the code, expressing the concept of field projection
concisely. The compiler changes are rather manageable, reusing several already existing systems,
thus increasing the maintenance burden only slightly if at all.

We could consider other operators rather than `->`. `->` has associations in C/C++ with performing a
dereference, while field projection doesn't necessarily perform a dereference. However, in C++ the
operator is also overloadable, so it isn't always a dereference. As an alternative to `->`, we could
consider `~` instead.

# Prior art
[prior-art]: #prior-art

Most importantly, see the [old field projection RFC](http://github.com/rust-lang/rfcs/pull/3318).
There also was a [pre-old-RFC
discussion](https://internals.rust-lang.org/t/pre-rfc-field-projection/17383/57) and the
list of crates in the next section is also from the old RFC.

## Crates

There are several crates implementing projections for different types.

- [pin projections]
  - [`pin-project`] provides pin projections via a proc macro on the type specifying the
    structurally pinned fields. At the projection-site the user calls a projection function
    `.project()` and then receives a type with each field replaced with the respective projected
    field.
  - [cell-project] provides cell projection via a macro at the projection-site: the user writes
    `cell_project!($ty, $val.$field)` where `$ty` is the type of `$val`. Internally, it uses unsafe
    to facilitate the projection.
  - [pin-projections] provides pin projections, it differs from [`pin-project`] by providing
    explicit projection functions for each field. It also can generate other types of getters for
    fields. [`pin-project`] seems like a more mature solution.
- `&[mut] MaybeUninit<T>` projections
  - [project-uninit] provides uninit projections via macros at the projection-site uses `unsafe`
    internally.
- multiple of the above
  - [`field-project`] provides projection for `Pin<&[mut] T>` and `&[mut] MaybeUninit<T>` via a macro
    at the projection-site: the user writes `proj!($var.$field)` to project to `$field`.
  - [`field-projection`] is an experimental crate that implements general field projections via a
    proc-macro that hashes the name of the field to create unique types for each field that can then
    implement traits to make different output types for projections.

[`field-project`]: https://crates.io/crates/field-project
[`cell-project`]: https://crates.io/crates/cell-project
[`pin-projections`]: https://crates.io/crates/pin-projections
[`project-uninit`]: https://crates.io/crates/project-uninit
[`field-projection`]: https://crates.io/crates/field-projection

## Blog Posts and Discussions

- [Design Meeting Field Projection](https://hackmd.io/@y86-dev/SkkfRkzWh)
- [Safe Cell field projection in
  Rust](https://www.abubalay.com/blog/2020/01/05/cell-field-projection)
- [Field projdection for `Rc` and
  `Arc`](https://internals.rust-lang.org/t/field-projection-for-rc-and-arc/15827)
- [Generic Field Projection](https://internals.rust-lang.org/t/generic-field-projection/16204)
- [Field Projection Use Cases](https://hackmd.io/@y86-dev/SkSB48hCR)

Blog posts about pin (projections):
- [Pinned places](https://without.boats/blog/pinned-places/)
- [Overwrite trait](https://smallcultfollowing.com/babysteps/series/overwrite-trait/)

## Rust and Other Languages

Rust already has a precedent for compiler-generated types. All functions and closures have a unique,
unnameable type.

In C++ there are field projections supported on `std::shared_ptr`, it consists of two pointers, one
pointing to the reference count and the other to the data. Making it possible to project down to a
field and still take a reference count on the entire struct, keeping also the field alive.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Syntax Bikeshedding

What is the right syntax for the various operations given in this RFC?

Ideally we would have a strong opinion when this feature is implemented. But the decision should
only be finalized when stabilizing the feature.

### Field Projection Operator

Current favorite: `$base:expr->$field:ident`.

Alternatives:
- use `~` instead of `->`

### Naming Field Types

Current favorite: `field_of!($Container:ty, $($fields:expr)+ $(,)?)` macro with `offset_of!` syntax.

Alternatives:
- Introduce a more native syntax on the level of types `$Container:ty->$field:ident` akin to
  projecting an expression.

### Declaring a Transparent Container Type

Current favorite: [`#[projecting]`](#projecting-attribute) attribute.

Alternatives:
- use `#[flatten]` instead.

## `Field` trait stability

Should we allow user implementations of the `Field` trait and have user-visible internals for it, or
should we make it more opaque and sealed to reserve the possibility of supporting enums or similar?

## Generalized enum projection

Is there a generalization of enum projection that allows for runtime-conditional projection, for
structures that sometimes-but-not-always have a given field? This could guarantee the type and
identity of the field, but require a projection to be validated against a runtime instance of the
value before confirming that the field exists and providing the projected field type. This would
allow enums, as well as runtime equivalents such as C-style unions with discriminants or similar
mechanisms for identifying variants at runtime.

## Other

- should the [`#[projecting]`](#projecting-attribute) attribute have an associated field attribute
  to mark the field that is projected onto?

# Future possibilities
[future-possibilities]: #future-possibilities

## Enums

Enums are difficult to support with the same framework as structs. The problem is that many
containers don't provide sufficient guarantees to read the discriminant (for example raw pointers
and `&mut MaybeUninit<T>`). However, for types that do provide sufficient guarantees, one could cook
up a similar feature. Let's call them *enum projections*. They could work like this: projecting is
done via a new kind of match operator:

```rust
enum MyEnum<F> {
    A(i32, String),
    B(#[pin] F),
}
type F = impl Future;
let x: Pin<&mut MyEnum<F>>;
match_proj x {
    MyEnum::A(n, s) => {
        let _: &mut i32 = n;
        let _: &mut String = s;
    }
    MyEnum::B(fut) => {
        let _: Pin<&mut F> = fut;
    }
}
```

I got this idea from reading the [Pinned places](https://without.boats/blog/pinned-places/) blog
post from boats. There, enum projections for pinned references (i.e. just pin projections) are
discussed.

Here `match_proj` would need to be a new keyword. I dislike the name and syntax, but haven't come up
with something better.

A similar issue comes up in the design of [deref patterns]. Since the types `Pin` and `MyEnum` are
distinct, they can be used to differentiate the kind of `match` the user wants to make. Thus making
it possible to only have the `match` operator and not a separate `match_proj` operator.

[deref patterns]: https://hackmd.io/4qDDMcvyQ-GDB089IPcHGg

## Arrays

Arrays can be thought of structs/tuples where each index is a field. Supporting them would simply
follow tuples. They might need additional syntax or just use the tuple syntax.

## Unions

Since field access for unions is `unsafe`, projection would also have to be `unsafe`. Since unions
are rarely used directly, this probably isn't important.

## More Stdlib Additions

Types that might be good candidates for [`#[projecting]`](#projecting-attribute):

- `ManuallyDrop<T>`

### `ArcRef<T>` for Stdlib
[arcref]: #arcref-t--for-stdlib

Using field projections, we can implement an `Arc` reference type, a pointer that owns a refcount on
an `Arc`, but points not at the entire struct in the `Arc`, but rather a field of that struct.

```rust
pub struct ArcRef<T: ?Sized> {
    ptr: NonNull<T>,
    count: NonNull<AtomicUsize>,
}

impl Drop for ArcRef<T: ?Sized> {
    fn drop(&mut self) {
        todo!()
        // decrement the refcount
    }
}

impl<T: ?Sized> Deref for ArcRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}
```

We can then make it have field projections:

```rust
impl<T: ?Sized> Projectable for ArcRef<T> {
    type Inner = T;
}

impl<T: ?Sized> SimultaneousProjectable for ArcRef<T> {
    type Inter = Self;

    fn start_projection(self) -> Self {
        self
    }
}

impl<T: ?Sized, F> SimultaneousProject<F> for ArcRef<T>
where
    F: Field<Base = T>
{
    type Output = ArcRef<F::Type>;

    unsafe fn project(inter: Self) -> Self::Output {
        // We give the refcount to the output, so we have to forget `self`.
        let this = ManuallyDrop::new(inter);
        ArcRef {
            ptr: NonNull::project(this.ptr),
            count: this.count
        }
    }
}
```

And to get an `ArcRef` from an `Arc`, we can also use field projections:

```rust
impl<T: ?Sized> Projectable for Arc<T> {
    type Inner = T;
}

impl<T: ?Sized> SimultaneousProjectable for Arc<T> {
    type Inter = ArcRef<T>;

    fn start_projection(self) -> ArcRef<T> {
        self.into_arc_ref()
    }
}

impl<T: ?Sized, F> Project<F> for Arc<T>
where
    F: Field<Base = T>
{
    type Output = ArcRef<F::Type>;

    unsafe fn project(inter: Self::Inter) -> Self::Output {
        unsafe { <ArcRef<T> as SimultaneousProject<F>>::project(inter) }
    }
}
```

Where `into_arc_ref` is implemented like this:

```rust
impl<T: ?Sized> Arc<T> {
    pub fn into_arc_ref(self) -> ArcRef<T> {
        let this = ManuallyDrop::new(self);
        let ptr = Arc::as_ptr(&*this);
        ArcRef {
            ptr,
            count: /* get ptr to strong refcount */
        }
    }
}
```

Maybe, the count ptr should also point to the weak count.

Now one can use it like this:

```rust
struct DataContainer {
    one: Data,
    two: Data,
}

struct Data {
    flags: u32,
    buf: [u8; 1024 * 1024],
}

let x = Arc::<DataContainer>::new_zeroed();
let x: Arc<DataContainer> = unsafe { x.assume_init() };

let one: ArcRef<Data> = x.clone()->one;
let two: ArcRef<Data> = x->two;

let flags: ArcRef<u32> = one->flags;
```

### `Cow<'_, T>`

For `Cow<'_, T>`, we need a new property for field types:

```rust
pub unsafe trait MoveableField: Field {
    fn move_out(base: Self::Base) -> Self::Type;
}
```

The `move_out` function is implemented by just moving out the field in question. Using this, we can
now implement field projections for `Cow<'_, T>`:

```rust
impl<'a, T: ?Sized + ToOwned<Owned = T>> Projectable for Cow<'a, T> {
    type Inner = T;
    type Inter = Self;

    fn start_projection(self) -> Self {
        self
    }
}

impl<'a, T: ?Sized + ToOwned<Owned = T>, F> Project<F> for Cow<'a, T>
where
    F: Field<Base = T>,
    F: MoveableField,
    F::Type: Sized,
{
    type Output = Cow<'a, F::Type>;

    fn project(inter: Self) -> Self::Output {
        match inter {
            Cow::Borrowed(this) => Cow::Borrowed(<&T as Project<F>>::project(this)),
            Cow::Owned(this) => Cow::Owned(F::move_out(this)),
        }
    }
}
```

### `Option<T>`

`Option<T>` is a bit of an interesting case, as it cannot be annotated with `#[projecting]`, since
it is not a transparent wrapper type. If we again consider an example struct:

```rust
struct Foo {
    bar: i32,
    baz: u32,
}
```

Then `Option<Foo>` does not have a field of type `Option<i32>`, since `Option` adds an additional
bit of information that needs to be represented in the raw bits of the type.

However, we can implement field projections for `Option<T>` when `T` has field projections
available. In the `None` case, we just project to `None` and in the `Some` case, we can use `T`'s
projection:

```rust
impl<T: Projectable> Projectable for Option<T> {
    type Inner = <T as Projectable>::Inner;
}

impl<T: SimultaneousProjectable> SimultaneousProjectable for Option<T> {
    type Inter = Option<<T as SimultaneousProjectable>::Inter>;

    fn start_projection(self) -> Self::Inter {
        self.map(T::start_projection)
    }
}

impl<T, F> Project<F> for Option<T>
where
    T: Project<F>,
    F: Field<Base = Self::Inner>,
    F::Type: Sized,
{
    type Output = Option<F::Type>;
    
    fn project(inter: Self::Inter) -> Self::Output {
        inter.map(<T as Project<F>>::project)
    }
}

// This probably overlaps with the impl above, but if the compiler is smart enough, it should know
// that they don't actually overlap.
impl<T, F> SimultaneousProject<F> for Option<T>
where
    T: SimultaneousProject<F>,
    F: Field<Base = Self::Inner>,
    F::Type: Sized,
{
    unsafe fn project(inter: Self::Inter) -> Self::Output {
        inter.map(|v| unsafe { <T as SimultaneousProject<F>>::project(v) })
    }
}
```

Now we are able to project for example `Option<&mut MaybeUninit<Foo>` to
`Option<&mut MaybeUninit<i32>>`.
