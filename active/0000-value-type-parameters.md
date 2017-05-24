- Start Date: 2014-07-20
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Values should be able to be passed as type parameters. The `[T, ..n]` fixed length array should be changed to use a value as a type parameter.

# Motivation

* Uniform behavior for `[u8, ..n]`

```rust
impl<const n: uint> Show for [u8, ..n] { ... }
```

* Traits for constructing literals

```rust
trait StringLit {
    fn new<const str: [u8, ..n], const n: uint>() -> Self
}
```

* Compile-time specialization

* Describing behavior of types

```rust
struct String<const null_terminated: bool> { ... }

struct HashTable<const inline_table_entries: uint> { ... }
```

* Intrinsics to read/write from LLVM address spaces

```rust
unsafe fn read_address_space<const address_space: u32, T>(dst: &mut T, target: *const T);
unsafe fn write_address_space<const address_space: u32, T>(src: &T, target: *mut T);
```

# Detailed design

## Which types to allow as values in type parameters

We should only allow copyable types here.
Initially I suggest using only allow primitive singular non-pointer types and retain the possibly to expand to more copyable types if desired.

## Which operations to allow

Any operations which we allow on values passed as type parameters the compiler must execute at compile-time.
We should limit these operations to only the safe ones, otherwise execution becomes more complex (it would require an hardware VM emulating the target, like QEMU).
Running the code in QEMU or LLVM IR VM seems like an easy way to avoid duplicating code for execution, 
but it requires the ability to only compile part of program.

Instead I suggest we use an interpreter which can only execute constructors,
references to other value type parameters, and constant static items which also satifies these rules.
It should also support all the operations already allowed in `[T, ..<expr>]`.
We can later extend the interpreter to more operations as needed.

# Drawbacks

The drawback is that it introduces more complexity to the compiler, although some of that complexity already exists because of the builtin fixed-size array.

# Alternatives

The alternative is to model values at the type level, which is awkward and slow.

For integers you can use a trick where you pass `[u8, ..<num>]` and extract the number using `size_of`.
This even works with negative integers. Doing any of this will make you feel awful though.
Let's hide the madness in a macro!
```rust
#![feature(macro_rules)]

macro_rules! pack(
    ($n:expr) => (
        [u8, ..$n]
    );
)

macro_rules! unpack(
    ($n:ty) => (
        std::mem::size_of::<$n>()
    );
)

fn print_int<num>() {
    println!("{}", unpack!(num));
}

fn main() {
    print_int::<pack!(4)>();
}
```
Well, that almost worked. Macros in type signatures aren't currently allowed.


# Unresolved questions

## Disallow value type parameters in traits

Value type parameters in traits could be disallowed.
I don't know of a use case for this and it would basically require a pattern match on values in implementations. 

## Allow default values

We could allow default values for the type parameters. These value should be limited to the same operations as listed above.
```rust
struct HashTable<const inline_table_entries: uint = 16> { ... }
```

## Syntax

### Declaration
Adding a keyword would be useful for clarity. `v: int` could be confused with a bound allowing just integer types.
```rust
fn make_int<v: int>() -> int { v }

fn make_int<static v: int>() -> int { v }

fn make_int<const v: int>() -> int { v }

fn make_int<let v: int>() -> int { v }
```
### Invocation
* Simple literals
```rust
make_int::<..2>();
make_int::<const 2>();
make_int::<2>();
```

* Paths
```rust
make_int::<..module::static_val>();
make_int::<const module::static_val>();
make_int::<module::static_val>();
```

* Full value grammar
```rust
make_int::<(2 + 2)>();
make_int::<{2 + 2}>();
make_int::<..(2 + 2)>();
make_int::<..{2 + 2}>();
make_int::<const (2 + 2)>();
make_int::<const {2 + 2}>();
```
