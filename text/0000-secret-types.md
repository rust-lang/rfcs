- Feature Name: Secret Types 
- Start Date: 2020-01-23
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

The goal is to provide primitive data types that can be used to traffic in transient secrets in code that are important to not accidentally leak. These new primitives would be used in conjunction with an in-progress LLVM RFC to ensure important security invariants like secret-independent runtime are maintained throughout the compilation process. Note that we explicitly do not want secret_isize and secret_usize, because we do not want to index based on secrets.

- secret_i8
- secret_i16 
- secret_i32
- secret_i64
- secret_i128
- secret_u8
- secret_u16 
- secret_u32
- secret_u64
- secret_u128
- secret_bool


# Motivation
[motivation]: #motivation

Applications deal with sensitive data all the time to varying degrees of success. Sensitive data is not limited to information like cryptographic codes and keys, but could also be data like passwords or PII such as social security numbers. Accidental secret leakage is a challenge for both programmers, who might mix secret and public data inadvertently, and compilers, which might use optimizations that reveal secrets via side-channels.

Writing cryptographic and other security critical code in high-level languages like Rust is attractive for numerous reasons. High level languages are generally more readable and accessible to developers and reviewers, leading to higher quality, more secure code. It also allows the integration of cryptographic code with the rest of an application without requiring the use of any FFI. We are also typically motivated to have a reference implementation for algorithms that is portable to architectures that may not be supported by highly optimized assembly implementations. However, writing data invariant code in high level languages is difficult due to compiler optimizations. For this reason, having compiler support for a data type that is resistant to timing side channel attacks is desirable.

Timing side channel attacks are a particular threat in a post spectre world [1]. Side channels are primarily used to attack secrets that are
long lived
 - Extremely valuable if compromised
 - Each bit compromised provides incremental value
 - Confidentiality of compromise is desirable
Therefore, it’s important to use data-invariant programming for secrets. For this reason, secret types would only allow data invariant operations at all levels of compilation.

Additionally, secret types serve as an indicator to programmers that this information should be treated with care, and not mixed with non-secret data, an invariant that would be enforced by the type system.

[1] *Cryptographic Software in a Post-Spectre World.* Chandler Carruth. RWC 2020. Recording forthcoming.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Secret integer types are a type of restricted integer primitive. In particular, non-constant time operations like division are prohibited. Printing of secret integers directly is also prohibited--they must first be declassified into non-secret integers.

These integers are intended to form a basis for the creation of other secret types, which can be built on top of them, such as secret strings for storing passwords in them.

Comparison of secret integer types is allowed, but the algorithm is independent of the secrets. Comparison of secret types must return a secret_bool, which cannot be branched on.

Example: comparison
```
let x : secret_i8 = 6;
Let y : secret_i8 = 10;
if (x < y) { // compiler error: cannot branch on secret bool
  ...
}
```
Example: declassification
```
let x : secret_i8 = 6;
let y : secret_i8 = 10;
println!((x ^ y).declassify())
```

Since indexing vectors and arrays is only possible using `usize,` you will be prevented from indexing into a vector or array with a secret integer. Error messages will derive naturally from the type system.


Secret types will likely be a more advanced topic when teaching Rust.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
For each fixed-size integer type:
Implement the following methods:
- From_be
- From_le
- From_be_bytes
- From_le_bytes
- From_ne_bytes
- Is_positive
- Is_negative
- Leading_zeros
- Min_value
- Max_value
- Overflowing_add
- Overflowing_sub
- Overflowing_mul
- Overflowing_neg
- Overflowing_shl
- Overflowiing_shr
- Overflowing_pow
- Pow
- Reverse_bits
- Rotate_left
- Rotate_right
- Saturating_add
- Saturating_sub
- Saturating_neg
- Saturating_mul
- Saturating_pow
- Signum
- Swap_bytes
- To_be
- To_le
- To_be_bytes
- To_le_bytes
- To_ne_bytes
- Trailing_zeros
- Wrapping_add
- Wrapping_sub
- Wrapping_mul
- Wrapping_neg
- Wrapping_shl
- Wrapping_shr
- Wrapping_pow


Implement the following traits
Secret integers may only  be combined with other secret integers and the result will be a secret type
- Add
- AddAssign
- BitAnd
- BitAndAssign
- BitOr
- BitOrAssign
- BitXor
- BitXorAssign
- Clone
- Copy
- Default
- Drop
- Mul
- MulAssign
- Neg
- Not
- Shl
- ShlAssign
- Shr
- ShrAssign
- Sub
- SubAssign
- Sub


For secret_bool, implement the following methods:
- n/a
And the following traits
- BitAnd
- BitAndAssign
- BitOr
- BitOrAssign
- BitXor
- BitXorAssign
- Clone
- Copy
- Default
- Not

We will also need to define a trait Classify<T> for T and method declassify. Classify will take a non secret integer or boolean and return a secret integer or boolean. `declassify` will consume a secret integer returning a non-secret integer.

We will need to define new comparison traits that run in constant time and return a secret_bool:
- SecretEq
- SecretOrd
For reference, see the subtle crate [2] .

[2] https://docs.rs/subtle/2.2.2/subtle/


# Drawbacks
[drawbacks]: #drawbacks

Because secret integers prohibit the use of certain operations and compiler optimizations, there will be a performance impact on code that uses them. However, this is intentional.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The main alternative to this design is to handle this via crates such as secret_integers [3]. However, without compiler integration, the compiler could optimize out the guarantees that have been claimed at the source level. For example, the compiler could use non constant time instructions such as divide. Using these primitives correctly from a high-level language will typically require careful management of memory allocation, flagging relevant memory as containing secret for the operating system, hardware, or other system level components.
Other alternatives would require developers to use handwritten assembly to ensure that only constant time operations are used.

[3] https://docs.rs/secret_integers/0.1.5/secret_integers/
 

# Prior art
[prior-art]: #prior-art
- https://docs.rs/secret_integers/0.1.5/secret_integers/
- https://docs.rs/subtle/2.2.2/subtle/


# Unresolved questions
[unresolved]: #unresolved-questions
Out of scope: Memory zeroing (beyond implementing the drop trait) and register spilling

There is an in progress RFC for LLVM that will be required for the secret type guarantees to be fully guaranteed throughout the compilation process.

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC describes the basic primitive types required for secret types in Rust. The intention is for crates to build additional secret types---for example a SecretString type---on top of these primitives.

Register spilling is a problem, and may require additional work to store secret values separately from non-secret values. One of the easy things to do with spectre is to create a timing side channel for stale data on the stack, meaning that it’s important to zero sensitive data from the stack. Reliably zeroing sensitive data is a difficult problem. We leave memory zeroing as future work.

Future possibilities also include creating compiler errors that specify why public and secret primitives cannot be mixed instead of returning a simple type error.

