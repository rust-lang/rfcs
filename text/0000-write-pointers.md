- Feature Name: parital_initialization_and_write_references

- Start Date: 2018-08-29

- RFC PR: (leave this empty)

- Rust Issue: (leave this empty)

# Summary

[summary]: #summary

This RFC aims to allow direct initialization for optimization, and partial struct and enum initialization, for ergonomics. It will do so through the usage of a two new reference type, `&out T` and `&uninit T` (the name is not important to me).

# Motivation

[motivation]: #motivation

The builder pattern was created as a way to try and solve the issue of not having partial initialization, but it has problems with large structs, and that the `*Builder` struct must necessarily be larger than the target struct, null-ptr optimizations not-withstanding. Also, it is very expensive to move large structs, and relying on the optimizer to optimize out moves isn't very good, `&out T` could serve as a way to directly place things into the desired memory location and to maintain write-only memory. `&uninit T` will serve the purpose of partial initialization and direct initialization.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

`&out T` is a write-only reference to `T`, where `T: Copy`. The bound is necessary as it is not safe to overwrite `Drop` types without invoking the destructor. Does not run destructors on write. See [unresolved questions](#unresolved-questions) for a more in depth explanation. \
`&uninit T` is a write-once reference to `T`, after the first write, it is allowed to read the values (behaves exactly like `&mut T`). Does not run destructors on the first write. \

For all examples, I will use these two structs

```Rust
struct Foo { a: u32, b: String, c: Bar }
#[derive(Clone, Copy)]
struct Bar { d: u32, e: f32 }

impl std::ops::Drop for Foo {
    fn drop(&mut T) {
        println!("Dropping Foo {}", foo.b);
    }
}
```

## `&uninit T`

Using `&uninit T`, we can do partial initialization and directly initialize.
```Rust
let x: Foo;

*(&uninit x.a) = 12;
*(&uninit x.b) = "Hello World".to_string();
*(&uninit x.c.d) = 11;
*(&uninit x.c.e) = 10.0;
```
This works because when we take an `&uninit` to `x.a`, we are implicity also taking an `&uninit` to `x`, and the dot operator will not attempt to read the memory location anywhere in `x`.

For ease of use, you can simply write
```Rust
let x: Foo;

x.a = 12;
x.b = "Hello World".to_string();
x.c.d = 11;
x.c.e = 10.0;
```
and the compiler will infer that all of these need to use `&uninit`, because `x` was not initialized directly.

### Restrictions

#### Storing

You cannot store `&uninit T` in any way, not in structs, enums, unions, or behind any references. So all of these are invalid.

```Rust
fn maybe_init(maybe: Option<&uninit T>) { ... }
fn init(ref_to_write: &mut &uninit T) { ... }
struct Temp { a: &uninit Foo }
```

#### Conditional Initialization

One restriction to `&uninit T` is that we cannot conditionally initialize a value. For example, none of these are allowed.
```Rust
let x: Foo;
let condition = ...;

if condition {
    x.a = 12; // Error: Conditional partial initialization is not allowed
}
```
```Rust
let x: Foo;
let condition = ...;

while condition {
    x.a = 12; // Error: Conditional partial initialization is not allowed
}
```
```Rust
let x: Foo;

for ... {
    x.a = 12; // Error: Conditional partial initialization is not allowed
}
```
Because if we do, then we can't gaurentee that the value is in fact initialized.

Note, that this is not conditionally initializing `x.e`, because by the end of the `if-else` block, `x.e` is guaranteed to be initialized.
```Rust
let x: Bar;

x.d = 10;

if { ... any condition ... } {
    x.e = 1.0;
} else {
    x.e = 0.0;
}
```

### Using partially initialized variables

```Rust
let x: Bar;
x.d = 2;

// This is fine, we know that x.d is initialized
x.d.pow(4);
if x.d == 16 {
    x.e = 10.0;
} else {
    x.e = 0.0;
}
// This is fine, we know that x is initialized
assert_eq!(x.e, 10.0);
```

### Functions and closures

You can accept `&uninit T` as arguments to a function or closure.

```Rust
fn init_foo(foo: &uninit Foo) { ... }
let init_bar = |bar: &uninit Bar| { ... }
```

But if you do accept a `&uninit T` argument, you must write to it before returning from the function or closure.

```Rust
fn valid_init_bar_v1(bar: &uninit Bar) {
    bar.d = 10;
    bar.e = 2.7182818;
}
fn valid_init_bar_v2(bar: &uninit Bar) {
    // you must dereference if you write directly to a &uninit T
    // This still does not drop the old value of bar
    *bar = Bar { d: 10, e: 2.7182818 };
}
fn invalid_init_bar_v1(bar: &uninit Bar) {
    bar.d = 10;
    // Error, bar is not completely initialized (Bar.e is not initialized)
}

fn invalid_init_bar_v2(bar: &uninit Bar) {
    bar.d = 10;
    if bar.d == 9 {
        return; // Error, bar is not completely initialized (Bar.e is not initialized)
    }
    bar.e = 10.0;
}
```

If a closure captures a `&uninit T`, then it becomes a `FnOnce`, because of the write semantics, the destructors will not be run the first time.

```Rust
let x: Foo;

let init = || x.a = 12; // init: FnOnce()  -> ()
```

**Note on Panicky Functions:**
If a function panics, then all fields initialized in that function will be dropped. No cross-function analysis will be done.

## `&out T`

Using `&out T`, we can directly initialize a value and guarantee to write only behavior. \
That would add give a memory location to write to directly instead of relying on move-elimination optimizations.

```Rust
#[derive(Clone, Copy)]
struct Rgb(pub u8, pub u8, pub u8);
/// This abstraction that exposes a Frame Buffer allocated by the OS, and is unsafe to read from
struct FrameBuffer( ... );

impl FrameBuffer {
    /// initializes the FrameBuffer in place
    fn new(&uninit self) { ... }

    /// gets a write only refernce to pixel at position (row, col)
    fn write_to_pixel(&mut self, row: usize, col: usize) -> &out Rgb {
         ...
    }
}
```
This could be used like this
```Rust
let buffer;
FrameBuffer::new(&uninit buffer);

*buffer.write_to_pixel(0, 0) = Rgb(50, 50, 255);
*buffer.write_to_pixel(10, 20) = Rgb(0, 250, 25);
/// ...
```

**Note:** `Rgb` is `Copy`, if it wasn't we could not gaurentee that we can safely overwrite it

## Constructors and Direct Initialization

Using `&uninit` we can create constructors for Rust!
```Rust
struct Rgb(u8, u8 ,u8);

impl Rgb {
    fn init(&uninit self, r: u8, g: u8, b: u8) {
        self.0 = r;
        self.1 = g;
        self.2 = b;
    }
}

let color: Rgb;
color.init(20, 23, 255);
```

and we can do direct initialization
```Rust
impl<T> Vec<T> {
    pub fn emplace_back(&mut self) -> &uninit T {
        ... // magic to allocate space and create reference
    }
}
```

and maintain write-only buffers
```Rust
struct WriteOnly([u8; 1024]);

impl WriteOnly {
    pub fn write(&out self, byte: u8, location: usize) {
        self.0[location] = byte; // currently not possible to index like this, but we could imagine a IndexOut, that will handle this case
    }
}
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

**NOTE** This RFC does NOT aim to create new raw pointer types, so no `*out T` or `*uninit T`. There is no point in creating these.

## Rules of `&uninit T`

`&uninit T` should follow some rules in so that is is easy to reason about `&uninit T` locally and maintain soundness
- `&uninit T` follows the same rules as `&mut T` for the borrow checker
- `&uninit T` can only be assigned to once
    - After being written to `&uninit T` are promoted to a `&mut T`
- Writing does not drop old value.
    - Otherwise, it would not handle writing to uninitialized memory 
    - More importantly, dropping requires at least one read, which is not possible with a write-only reference
- You cannot reference partially initialized memory
```Rust
let x: Bar;

fn init_bar(bar: &uninit Bar) { ... }
fn init_u32(x: &uninit u32) { ... }

x.e = 10.0;

// init_bar(&uninit x); // compile time error: attempting to reference partially initialized memory
init_u32(&uninit x.d); // fine, x.d is completely uninitialized.
```
- Functions and closures that take a `&uninit T` argument must initialize it before returning
    - You cannot return an `&uninit T`
- You can take a `&uninit T` on any `T` that represents uninitialized memory, for example: only the first is ok.
```Rust
let x: Foo;
let y = &uninit x;
```
```Rust
let x: Foo = Foo { a: 12, b: "Hello World".to_string() };
init(a: &uninit Foo) { ... }
init(&uninit x); // this function will overwrite, but not drop to the old value of x, so this is a compile-time error
```

## Rules of `&out T`

`&out T` should follow some rules in so that is is easy to reason about `&out T` locally and maintain soundness
- `&out T` follows the same rules as `&mut T` for the borrow checker
- Writing does not drop old value.
    - Dropping requires at least one read, which is not possible with a write-only reference
- You can take a `&out T` on any `T: Copy`
    - because destructors are never run on write, `T: Copy` is necessary to guarantee no custom destructors. This bound can be changed once negative trait bounds land, then we can have `T: !Drop`. Changing from `T: Copy` to `T: !Drop` will be backwards compatible, so we can move forward with just a `T: Copy` bound for now.

## Coercion Rules

`&T` - (none) // no change \
`&mut T` - `&T`, `&out T` if, `T: Copy` \
`&out T` - (none) // for similar reasons to why `&T` does not coerce \
`&uninit T` - `&out T` if `T: Copy` and `&T` or `&mut T` once initialized.

## `self`

We will add `&uninit self` and `&out self` as sugar for `self: &uninit Self` and `self: &out Self` respectively. This is for consistency with `&self`, and `&mut self`

## Panicky functions in detail

Because we can pass `&uninit T` and `&out T` to functions, we must consider what happens if a function panics. For example:
```Rust
fn init_foo_can_panic(foo: &uninit Foo) {
    foo.b = "Hello World".to_string();
    foo.a = 12;
    
    if foo.a == 12 {
        // When we panic here, we should drop all values that are initialized in the function.
        // Nothing could have been initialized before the function because we have a &uninit T
        panic!("Oh no, something went wrong!");
    } 

    foo.c = Bar { d = 10, e = 12.0 };
}

fn out_bar_panics(foo: &out Bar) {
    // When we panic here we drop here we don't ever drop any value behind a &out because &out can never have a destructor, it doesn't matter
    panic!("Oh no, something went wrong!");
}

let x: Foo;

init_foo_can_panic(&uninit x);

let x: Bar;

out_bar_panics(&out x); // when we take a &out, we are asserting that the old value doesn't need to drop, and doesn't matter. This is fine because Bar is Copy and does not have a destructor.
```

# Drawbacks

[drawbacks]: #drawbacks

 - This is a significant change to the language and introduces a lot of complexity. \
 - Partial initialization can be solved entirely through the type-system as shown [here](https://scottjmaddox.github.io/Safe-partial-initialization-in-Rust/). But this does have its problems, such as requiring an unstable feature (untagged_unions) or increased size of the uninitialized value (using enums).

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

## `T: !Drop` for `&out T`

Once negative trait bounds become stable, the bounds for `&out T` will change to `T: !Drop`. But this does not seem like the correct bound, see [here](#unresolved-questions) for why.

## Allow Drop types to be partially initialized

Then they would only be dropped if all of their fields are initialized

## Placement-new

Placement new would help, with initializing large structs.

## As sugar

This could be implemented as sugar, where all fields of structs that are partially initialized are turned into temp-variables that are then passed through the normal pipeline.

For example
```Rust
let x: Bar;
x.d = 10;
x.e = 12.0;
```

would desugar to

```Rust
let x: Bar;
let xd = 10;
let xe = 12.0;
x = Bar { d: xd, e: xe };
```

But this would not be able to replace placement new as it can't handle `&uninit T` through function boundaries. Also this would not solve the problem of direct-initialization.

# Prior art
[prior-art]: #prior-art

Out pointers in C++, (not exactly the same, but similar idea)

`&out T` in  C#

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - What is the correct bounds for `T` in `&out T`? conservatively, `T: Copy` works and is sound. But as [@TechnoMancer](https://internals.rust-lang.org/u/TechnoMancer) pointed out, `T: !Drop` is not the correct bound. For example, `Wrapper(Vec<String>)`, clearly cannot be overwritten safely, because `Vec<String>`, must drop do deallocate the `Vec`, but `Wrapper` itself does not implement drop. Therefore either a new trait is needed (but unwanted), or we must keep the `Copy` bound.


---

edit:
Added Panicky Function sub-section due to [@rkruppe](https://internals.rust-lang.org/u/rkruppe)'s insights

added `&out T` by C# to prior arts and alternative syntax due to [@earthengine](https://internals.rust-lang.org/u/earthengine)'s suggestion

removed lots of unnecessary spaces and newlines

edit 2:

Incorporating [@gbutler](https://internals.rust-lang.org/u/gbutler)'s proposal of splitting `&uninit T` into `&out T` and `&uninit T`

edit 3:

Used [@gbutler](https://internals.rust-lang.org/u/gbutler)'s example of FrameBuffer that interfaces hardware for `&out T`

edit 4:

Fixed example for `&out T`.

---
I would like to thank all the people who helped refine this proposal to its current state: [@rkruppe](https://internals.rust-lang.org/u/rkruppe), [@earthengine](https://internals.rust-lang.org/u/earthengine),  [@gbutler](https://internals.rust-lang.org/u/gbutler),
and [@TechnoMancer](https://internals.rust-lang.org/u/TechnoMancer) thank you!