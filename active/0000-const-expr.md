- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

- Ensure that, if a constant expression is accepted in one position, then
  it's also accepted in every other position that expects constant
  expressions.
- Don't force the user to write `unsafe { }` in constant expressions.
- Allow arbitrary pointer arithmetic in constant expressions. This includes
  dereferencing the pointer as long as the value is not being read.

# Motivation

## Unification of constant expressions

Right now, if one of the following two lines compiles, the other one doesn't
necessarily compile:
```rust
static X: uint = <expr>; // <expr> is a fixed expression
static Y: [u8, ..<expr>] = [0, ..<expr>];
```
Even the following does not necessarily work:
```rust
static Z: [u8, ..X] = [0, ..X]; // X as above
```

For example:
```rust
#[repr(uint)]
enum A {
    B,
    C,
}

static X: uint = B as uint;
static Y: [uint, ..X] = [];

fn main() { }
```
This is extremely frustrating and confusing. All constant expressions should
go through the same checks and they should be valid in array-length position
if and only if they evaluate to a generic or `uint` integer.

## Removal of `unsafe { }`

Right now, the following code has to be marked unsafe (and ICEs):
```rust
static mut X: uint = 0;
static Y: uint = unsafe { X };
```
Normally `unsafe { }` means that the user acknowledges that he opts out of
compiler-based safety checks. That is, code in such blocks cannot be checked
by the compiler for total safety (in the sense of safe Rust code.) But in
constant expressions this is not the case. Constant expressions only contain
code which can be evaluated by the compiler itself. Given the lack of compile
time function evaluation, and if we assume that the compiler is bug free,
code that compiles as a constant expression has been proven safe by the
compiler.

## Full pointer arithmetic

Consider the following macro and code.
```rust
#![feature(macro_rules)]

macro_rules! size_of {
    ($T:ty) => {
        unsafe { 
            &(*(0u as *const [$T, ..2]))[1] as *const $T as uint
        }
    }
}

struct X {
    _i: i32,
    _j: i8,
}

fn main() {
    println!("{}", size_of!(X));
}
```
This will print the size of `X` (most likely 8) by only using pointer
arithmetic. Unfortunately this code does not work properly in statics:
```rust
static SIZE_OF_X: uint = size_of!(X);
```
This causes an ICE because (what follows is pure speculation) the compiler
tries to actually dereference here:
```rust
*(0u as *const [$T, ..2])
```
This is not actually necessary because we're referencing often enough to
cancel this dereference out, as the working program above shows.

If this code worked, then we could, in combination with the first part of
this RFC, use it to write the following struct:
```rust
#[repr(C)]
struct XContainer {
    data: [u8, ..size_of!(X)],
    _alignment: [X, ..0],
}
```

Note that C allows pointer arithmetic in constant expressions:
```
An address constant is a null pointer, a pointer to an lvalue designating an
object of static storage duration, or a pointer to a function designator; it
shall be created explicitly using the unary & operator or an integer constant
cast to pointer type, or implicitly by the use of an expression of array or
function type. The array-subscript [] and member-access .  and -> operators,
the address & and indirection * unary operators, and pointer casts may be
used in the creation of an address constant, but the value of an object shall
not be accessed by use of these operators.
```
This means that `*(0 as *const uint)` is disallowed but
`&*(0 as *const uint)` returns a `uint` null pointer.

Also note that, together with the second point, this pointer arithmetic
doesn't need an unsafe block. This makes sense because the compiler doesn't
actually use any pointers but only integers during the calculation.

# Detailed design

The expected behavior has already been described in the previous section and
the rest are implementation details.

However, here are some details regarding the current implementation that
cause the problems described above:

- There is not one place that checks if a constant expression is valid.
  Instead there is one file that checks constant expressions in statics and
  another one that checks constant expressions in arrays. This code
  duplication most likely causes the first bug above.
- The evaluation of constant expressions is split into "easy" constant
  expressions and "hard" constant expressions. A lack of synchronization
  between these two modules causes interesting bugs like the following: The
  expression `1/0` "successfully" evaluates to `0` but `X[1/0]` fails because
  `1/0` tries to divide by zero.

These modules should be unified.

# Drawbacks

Allowing "unsafe" code without an unsafe block means that code from constant
expressions cannot always be used in other code. On the other hand, this
might already be the case today.

# Alternatives

Some of these things can be considered bugs and have to be fixed.

- Don't implement all of it.

# Unresolved questions

None right now.
