- Start Date: (fill me in with today's date, YYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Improve SIMD support in the Rust compiler and libraries.

# Motivation

[SIMD] is a useful way of exploiting instruction-level parallelism in modern
hardware. While some code can be optimized to use these instructions without
programmer instruction, they cannot be relied upon for real-world code.

Rust already has minimal support for SIMD vectors via an `#[simd]` attribute
on struct types that mean certain requirements. Extending this support into
something that allows access to more of the underlying features would greatly
help programmers to exploit modern hardware safely.

# Detailed design

## Syntax

SIMD vectors are unlikely to be common, therefore implementing an entire new
syntax is probably not worth it. Syntax extensions are the logical choice
here, as we only need a few elements to get this to work.

Given the similarity to the existing fixed-length vectors, I propose the
following syntax:

```rust
simd![T,..n] // SIMD type syntax
simd![Expr, Expr, Expr] // SIMD expression syntax
simd![Expr,..Expr] // SIMD repeat expression
```

The second expression in the repeat syntax must evaluate to a constant integral
greater than or equal to 1.

As such, these would be used like so:

```rust
fn make_vec3() -> simd![f32,..3] {
    simd![0, ..3]
}
```

## Type System Integration

SIMD vectors would require the addition of a new base type similar to
fixed-length vectors. This type would, however, limit it's subtype to machine
types, i.e. integers, floats and bools.

Casting between different SIMD types of the same length should be supported,
e.g. `simd![f32, ..4] => simd![u32, ..4]`. Vectors of length 1 should be
coercible to their subtype and vice versa.

Taking the address of a vector element will not be allowed.

## Back-end Implementation

LLVM supports arbitrary vector types of integer or floating point types of any
length greater than zero. Rust SIMD vectors would map directly to these.

## API and Operations Support

Operations on SIMD vector types should follow that of the [OpenCL vectors].

### Binary Operations

For standard binary operations, the operation is performed component-wise,
for example:

```rust
let v : simd![f32,..4];
let u : simd![f32,..4];
let x : simd![f32,..4];

...

let x = v + u
```

will be quivalent to:

```rust
x.x = v.x + u.x;
x.y = v.y + u.y;
x.z = v.z + u.z;
x.w = v.w + u.w;
```

The operations would be limited based on the supported operations of the
vector's subtype.

### Comparisons

Programmers using SIMD vectors in comparisons are likely to want to get a
vector of boolean values as a result. However, the traits for these operations
require returning a single boolean value. With this in mind, I propose only
implementing the equality operator that returns whether every pair in the two
vectors are equal or not.

Component-wise comparison, returning a vector of boolean values, shall be done
by a set of appropriate intrinsics.

### Shuffle Access

Open CL vectors support an accessor-style syntax for shuffle and related
operations. I propose implementing a similar syntax:

```rust
let v : simd![f32,...4] = simd![1.0, 2.0, 3.0, 4.0];

let x = v.x; // 1.0f32
let v3 : simd![f32,..3] = v.xyz; // simd![1.0, 2.0, 3.0]
let swiz : simd![f32,..4] = v.wzyx; // simd![4.0, 3.0, 2.0, 1.0]
let dup : simd![f32,..4] = v.xxyy; // simd![1.0, 1.0, 2.0, 2.0]

// simd![1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0]
let num : simd![f32,..8] = v.s00112233;
```

The `x`, `y`, `z` and `w` components will be usable to access the first,
second, third and fourth components of the type, for vectors of size 2, 3
and 4.

Vectors of size sixteen or less will support component access via numeric
indices in hexadecimal as part of a field with an `s` prefix.

`shuffle` and `shuffle2` intrinsics should be available to implement the
above behaviour for variable permutations and vectors of a size greater than
16.

The same accessor syntax may be used to set arbitrary components of a vector
all at once.

### Load/Store Functions

There should be a set of unsafe load and store functions for reading and
writing vectors to a raw pointer. These are required due to alignment concerns
on some operating systems.

### Miscellaneous Functions

There are numerous useful functions and methods that could be implemented for
SIMD vectors. I consider it to be beyond the scope of this RFC to explore that
part of SIMD support.

# Alternatives

* Use fixed-length vectors as they are now and do vector operations where
  appropriate. I do not think that this is a good idea as SIMD vectors are
  more strict than normal vectors, fixed-length or otherwise. They would also
  not interact well under DST and the current coercion rules.
* Don't have SIMD support at all. If Rust does not wish to support SIMD
  vectors in the language going forward, the current support for them should
  be removed. If we do not support SIMD vectors properly, then we should not
  support them at all.

# Unresolved questions

1. Syntax - should it stay as proposed or is there a better alternative?
2. Shuffle support via field access. It's a nice feature that I would like to
   see, however the complexity of implementation may not be worth it.
3. Upper limit on vector size. To my knowledge, LLVM does not have an upper
   limit on the size of the vector. Should we enforce one anyway?
4. LLVM supports vectors of pointers to the supported types, should we also
   support this, given that they would need to be unsafe?

[SIMD]: http://en.wikipedia.org/wiki/SIMD
[OpenCL vectors]: http://www.khronos.org/registry/cl/specs/opencl-1.2.pdf#page=234
