- Feature Name: `clear_on_drop`
- Start Date: 2016-02-10
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This RFC proposes a pair of attributes, `#[clear_on_drop]` and
`#[clear_stack_on_return]`, to make it easy to write code which securely
clears sensitive data (for instance, encryption keys) after its use.

# Motivation
[motivation]: #motivation

Some kinds of data should not be kept in memory any longer than they are
needed. For instance, the TLS protocol has Ephemeral Diffie-Hellman
modes which give a connection the forward secrecy property — but only if
both the ephemeral key and the session keys are securely discarded after
the connection is over.

For long-lived processes, unless carefully overwritten, pieces of
sensitive data can linger in the heap, the stack, and even in the
processor registers. Clearing the heap and explicitly allocated
variables in the stack is somewhat easy, needing only careful attention
to not forget any exit path, and some way to defeat the compiler's dead
store optimizations. Clearing compiler-created temporary variables in
the stack and processor registers is harder.

This RFC proposes a pair of attributes to simplify this task, and reduce
the chance of mistakes. The first one, `#[clear_on_drop]`, asks the
compiler to, after any drop or move of a variable of the given type,
securely overwrite its contents. The second one,
`#[clear_stack_on_return]`, asks the compiler to, just before a given
function returns, securely overwrite all the stack space and processor
registers it used.

(In this text, "securely overwrite" means: overwrite with some value
unrelated to any of the values being securely overwritten, and in such a
way that no compiler or linker optimization pass will remove the
overwrite, without also removing the original write(s). The exact value
being used doesn't matter; for instance, a callee-saved register can be
securely overwritten by the normal restore of the caller's value from
the stack).

# Detailed design
[design]: #detailed-design

This RFC proposes two independent attributes, which complement each
other. They will be explained separately.

## `#[clear_on_drop]`

The `#[clear_on_drop]` attribute can be attached to a `struct` or to an
`enum`. Conceptually, the attribute modifies the type's `Drop` impl
(adding one if needed) so that, after it finishes:

* The memory used by the variable is securely overwritten;
* For non-`Copy` types, the memory used by it before each move is
  securely overwritten.

Clearly, it's impractical to track the memory used before every move, to
overwrite it all in the end; therefore, in practice `#[clear_on_drop]`
also asks the compiler to securely overwrite the old copy after every
move (for `Copy` types, the old copy will be overwritten by its `Drop`).

Derived impls for `#[clear_on_drop]` types implicitly have all their
functions marked with the `#[clear_stack_on_return]` attribute.

## `#[clear_stack_on_return]`

The `#[clear_stack_on_return]` attribute can be attached to a function.
It asks the compiler to, just before the function returns, and after the
`Drop` of its local variables, securely overwrite:

* All local variables;
* The stack space used for all temporaries;
* The stack space used to spill intermediate registers;
* All processor registers written by the function, except the return
  value registers.

Some of this can be omitted; for instance, if a register is written with
the same constant value in all paths leading out of the function, it
does not have to be overwritten (conceptually, it is being overwritten
with the same constant value it already has). In the same way, the stack
space used by a `#[clear_on_drop]` local variable does not need to be
overwritten, since its own `Drop` implementation already did the
overwrite.

When `#[clear_stack_on_return]` is combined with inlining, it has a few
special effects:

* When a function marked with `#[clear_stack_on_return]` is inlined
  within another function also marked with `#[clear_stack_on_return]`,
  the clearing of the inner (inlined) function can be delayed until the
  clearing of the outer function. This is important for speed.
* When any function is inlined within a `#[clear_stack_on_return]`
  function, the inner (inlined) function MUST be treated as if it also
  were marked with the `#[clear_stack_on_return]` attribute. This is
  important for safety: these are often small accessor functions, for
  instance a `Vec`'s `index()`.
* For the same reason, functions marked with `#[inline]` should always
  be inlined within a `#[clear_stack_on_return]` function, as if they
  were `#[inline(always)]`.

Any closures created within a `#[clear_stack_on_return]` function are
also treated as if they were marked with `#[clear_stack_on_return]`.

The effects of a `panic!()` within a `#[clear_stack_on_return]` function
are unspecified. It might or might not clear the stack and registers,
and might even partially clear the state.

## Example

```rust
#[clear_on_drop]
#[derive(Clone, Debug)]
pub struct MyKeyedHash {
    key: [u8; 8],
    state: [u8; 8],
}

// Generated by #[clear_on_drop]
//impl Drop for MyKeyedHash {
//    #[clear_stack_on_return]
//    fn drop(&mut self) {
//        self.key = [0; 8];
//        self.state = [0; 8];
//        // Prevent dead store optimizations from removing the above
//        compiler_magic(&self);
//    }
//}

// Generated by #[derive(Clone)] (Debug is similar)
//impl Clone for MyKeyedHash {
//    #[clear_stack_on_return]
//    fn clone(&self) -> Self { ... }
//    #[clear_stack_on_return]
//    fn clone_from(&mut self, source: &Self) { ... }
//}

impl MyKeyedHash {
    #[clear_stack_on_return]
    fn new(key: &[u8]) -> MyKeyedHash { ... }

    #[clear_stack_on_return]
    fn process(&mut self, data: &[u8]) { ... }

    #[clear_stack_on_return]
    fn finish(self) -> [u8; 8] { ... }
}
```

## Implementation notes

The `#[clear_on_drop]` attribute requires only minimal help from LLVM;
it needs only a way to suppress dead store optimizations. The equivalent
of `asm!("" : : "r" (&self))` after the overwrite step might be enough.

The `#[clear_stack_on_return]` attribute, on the other hand, will
probably need to be implemented deep in the LLVM layer, and passed from
Rust as a LLVM function attribute.

# Drawbacks
[drawbacks]: #drawbacks

This design protects only specially-marked types and functions. There
are many types and functions which obviously should be marked (cypher
and hash state objects, their internal functions, and bignum libraries
designed for public key operations), but for some it's not obvious. For
instance, should a plaintext buffer be protected by these attributes?

This design only does a "shallow" clear. For instance, if a
`#[clear_on_drop]` object (or a `#[clear_stack_on_return]` function)
contains a `Vec`, its contents will not be cleared (this might be
mitigated by being careful to not reallocate, and explicitly clearing
the `Vec` in the `Drop`).

The contents of the stack and/or registers might not be completely
cleared when unwinding after a `panic!()`. Requiring the compiler to do
so might complicate things too much.

While `#[clear_on_drop]` might be implemented only on rustc itself,
implementing `#[clear_stack_on_return]` requires changing LLVM.

# Alternatives
[alternatives]: #alternatives

The `#[clear_on_drop]` attribute can be emulated by a hand-written
`Drop` impl, combined with the unstable inline assembly feature, as long
as the type is never moved. This, however, prevents designs where a few
specific methods "consume" the object (see the `finish` method in the
example above).

Some cryptographic libraries attempt to do something like the proposed
`#[clear_stack_on_return]` by calling functions written in assembly to
allocate and overwrite a large amount of stack, and zero all registers.
In my opinion, this is both fragile and wasteful; compiler changes might
use more stack than expected (or, to prevent that, large amounts of
stack are unnecessarily cleared), and a new compiler might use more
registers, including registers which did not exist when the
register-clearing code was written (for instance, new or larger vector
registers).

Even a naive implementation within the compiler, on the other hand,
could simply overwrite all the stack the function used, since it knows
by how much it had to adjust the stack pointer, and overwrite all the
caller-saved and scratch registers which it knows about.

An interesting alternative to marking some types and functions might be
to always securely overwrite. This could lose a lot of speed, but might
be interesting for high-security programs; perhaps this mode could be
enabled by a compiler command line option.

Finally, there's the alternative of doing nothing. Unless the program
plays with unsafe code and uninitialized memory, there's little risk of
a Rust program leaking old secrets. This, however, protects only against
bugs in the program; its memory can still be externally dumped and
analyzed, either through software (`ptrace()` and others) or hardware
(coldboot attacks).

# Unresolved questions
[unresolved]: #unresolved-questions

What should be done when a `#[clear_stack_on_return]` function panics?
