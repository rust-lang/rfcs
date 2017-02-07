- Feature Name: (fill me in with a unique ident, my_awesome_feature)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC adds explicit proper tail calls to Rust via the `become` keyword.
It also specifies the semantics of proper tail calls and their interaction
with destructors.

# Motivation
[motivation]: #motivation

Proper tail calls are commonly found in functional languages, such as OCaml,
SML, Haskell, Scheme, and F#.  They also can be used to easily encode state
machines and jump tables.  Furthermore, they allow for code to be written in
continuation-passing style, which may be useful for some functional programming
patterns.

# Detailed design
[design]: #detailed-design

## Syntax
[syntax]: #syntax

A proper tail call is written using the `become` keyword.  This is already
reserved, so there is no backwards-compatibility break.

The `become` keyword must be followed by a function or method calls.
Invocations of overloaded operators with at least one non-primitive argument
were considered as valid targets, but were rejected on grounds of being too
error-prone.  In any case, these can still be called as methods.

A future RFC may allow `become` in more places.

## Type checking
[typechecking]: #typechecking
A `become` statement is type-checked exactly like a `return` statement.  In the
current implementation, the syntactic restrictions on `become` noted above are
enforced during this phase.  Additionally, the callee **must** use either the
`rust` calling convention or the `rust-call` calling convention.

## Borrowchecking and Runtime Semantics
[semantics]: #semantics

A `become` expression acts as if the following events occurred in-order:

1. All variables that are being passed by-value are moved to temporary storage.
2. All local variables in the caller are destroyed according to usual Rust
   semantics.  Destructors are called where necessary.  Note that values
   moved from in step 1 are _not_ dropped.
3. The caller's stack frame is removed from the stack.
4. Control is transferred to the callee's entry point.

This implies that it is invalid for any references into the caller's stack frame
to outlive the call.

The borrow checker ensures that none of the above steps will result in the use
of a value that has gone out of scope.

An implementation is required to support an unlimited number of proper tail
calls without exhausting any resources.

## Implementation
[implementation]: #implementation

The parser parses `become` exactly how it parses the `return` keyword.  The
difference in semantics is handled later.

During type checking, the following are checked:

1. The target of the tail call is, in fact, a call.
2. The target of the tail call has the proper ABI.

Later phases in the compiler assert that these requirements are met.

New nodes are added in HIR and HAIR to correspond to `become`.  In MIR, however,
a new flag is added to the `TerminatorKind::Call` varient.  This flag is only
allowed to be set if all of the following are true:

1. The destination is `RETURN_POINTER`.
2. There are no cleanups.
3. The basic block being branched into has length zero.
4. The basic block being branched into terminates with a return.

Trans will assert that the above are in fact true.

Finally, the use of proper tail calls must be propogated to LLVM.  This is done
in two ways:

1. Turn on tail call optimization.  This is done by setting
   `Options.GuaranteedTailCallOpt` in
   [PassWrapper.cpp](src/rustllvm/PassWrapper.cpp).
2. Make the actual call a tail call.  This is done by means of the following
   function, added to [RustWrapper.cpp](src/rustllvm/RustWrapper.cpp):

   ```c++
   extern "C" void LLVMRustSetTailCall(LLVMValueRef Instr) {
     CallInst *Call = cast<CallInst>(unwrap<Instruction>(Instr));
     Call->setTailCall();
   }
   ```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Tail calls are essentially disciplined cross-function `goto` – a unidirectional
transfer of control.  They are a wholly new pattern for Rust, and make others
possible, such as continuation-passing style.

Nevertheless, tail calls are an advanced feature that can produce code which is
difficult to debug.  As such, they should only be used when other techniques are
not suitable.

I believe that the first three paragraphs under #detailed-design could be added
to the Rust Reference with only minor changes.  New material containing some
use cases would need to be added to _The Rust Programming Language_ and
_Rust by Example_.

# Drawbacks
[drawbacks]: #drawbacks

## Runtime overhead
[runtime overhead]: #runtime-overhead

One major drawback of proper tail calls is that their current implementation in
LLVM is not zero-cost: it forces a callee-pops calling convention, and thus
causes a stack adjustment after each non-tail call.  This could be a performance
penalty.

## Portability
[portability]: #portability

An even greater drawback of proper tail calls is lack of cross-platform support:
LLVM does not support proper tail calls when targeting MIPS or WebAssembly, and
a compiler that generated C code would be hard-pressed to support them.  While
relying on sibling call optimization in the C compiler might be possible with
whole-program compilation, it would still be tricky.  WebAssembly does not
support tail calls at all yet, so stablization of this feature will need to wait
until this changes, which could take years.

In fact, this is such a drawback that I (Demi Marie Obenour) considered not
making this RFC at all.  Rust language features (as opposed to library features)
should work everywhere.  That this does not is unfortunate.

## Debugability
[debugability]: #debugability

Proper tail calls can make debugging difficult, by overwriting stack frames.

# Alternatives
[alternatives]: #alternatives

Proper tail calls are not necessary.  Rust has done fine without them, and will
continue to do so if this RFC is not accepted.

# Unresolved questions
[unresolved]: #unresolved-questions

Is there a way to emulate proper tail calls when compiling to targets that don't
support them?  Is it possible to eliminate the overhead imposed on every
non-tail call?
