
- Feature Name: `unsafe_extern`
- Start Date: 2023-05-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

In Edition 2024 it is `unsafe` to declare an `extern` function or static, but external functions and statics *can* be safe to use after the initial declaration.

# Motivation
[motivation]: #motivation

Simply declaring extern items, even without ever using them, can cause Undefined Behavior.
When performing cross-language compilation, attributes on one function declaration can flow to the foreign declaration elsewhere within LLVM and cause a miscompilation.
In Rust we consider all sources of Undefined Behavior to be `unsafe`, and so we must make declaring extern blocks be `unsafe`.
The up-side to this change is that in the new style it will be possible to declare an extern fn that's safe to call after the initial unsafe declaration.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust can utilize functions and statics from foreign code that are provided during linking, though it is `unsafe` to do so.

An `extern` block can be placed anywhere a function declaration could appear (generally at the top level of a module), and must always be prefixed with the keyword `unsafe`.

Within the block you can declare the exernal functions and statics that you want to make visible within the current scope.
Each function declaration gives only the function's signature, similar to how methods for traits are declared.
If calling a foreign function is `unsafe` then you must declare the function as `unsafe fn`, otherwise you can declare it as a normal `fn`.
Each static declaration gives the name and type, but no initial value.

* If the `unsafe_code` lint is denied or forbidden at a particular scope it will cause the `unsafe extern` block to be a compilation error within that scope.
* Declaring an incorrect external item signature can cause Undefined Behavior during compilation, even if Rust never accesses the item.

```rust
unsafe extern {
    // sqrt (from libm) can be called with any `f64`
    pub fn sqrt(x: f64) -> f64;
    
    // strlen (from libc) requires a valid pointer,
    // so we mark it as being an unsafe fn
    pub unsafe fn strlen(p: *const c_char) -> usize;
    
    pub static IMPORTANT_BYTES: [u8; 256];
    
    pub static LINES: UnsafeCell<i32>;
}
```

Note: other rules for extern blocks, such as optionally including an ABI, are unchanged from previous editions, so those parts of the guide would remain.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This adjusts the grammar of the language to *require* the `unsafe` keyword before an `extern` block declaration (currently it's optional and syntatically allowed but semantically rejected).

Replace the *Functions* and *Statics* sections with the following:

### Functions
Functions within external blocks are declared in the same way as other Rust functions, with the exception that they must not have a body and are instead terminated by a semicolon. Patterns are not allowed in parameters, only IDENTIFIER or _ may be used. The function qualifiers `const`, `async`, and `extern` are not allowed. If the function is unsafe to call, then the function must use the `unsafe` qualifier.

If the function signature declared in Rust is incompatible with the function signature as declared in the foreign code it is Undefined Behavior.

Functions within external blocks may be called by Rust code, just like functions defined in Rust. The Rust compiler will automatically use the correct foreign ABI when making the call.

When coerced to a function pointer, a function declared in an extern block has type
```rust
extern "abi" for<'l1, ..., 'lm> fn(A1, ..., An) -> R
```
where `'l1`, ... `'lm` are its lifetime parameters, `A1`, ..., `An` are the declared types of its parameters and `R` is the declared return type.

### Statics
Statics within external blocks are declared in the same way as statics outside of external blocks, except that they do not have an expression initializing their value. It is unsafe to declare a static item in an extern block, whether or not it's mutable, because there is nothing guaranteeing that the bit pattern at the static's memory is valid for the type it is declared with.

Extern statics can be either immutable or mutable just like statics outside of external blocks. An immutable static must be initialized before any Rust code is executed. It is not enough for the static to be initialized before Rust code reads from it. A mutable extern static is unsafe to access, the same as a Rust mutable static.

# Drawbacks
[drawbacks]: #drawbacks

* It is very unfortunate to have to essentially reverse the status quo.
  * Hopefully, allowing people to safely call some foreign functions will make up for the churn caused by this change.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Incorrect extern declarations can cause UB in current Rust, but we have no way to automatically check that all declarations are correct, nor is such a thing likely to be developed. Making the declarations `unsafe` so that programmers are aware of the dangers and can give extern blocks the attention they deserve is the minimum step.

# Prior art
[prior-art]: #prior-art

None we are aware of.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Extern declarations are actually *always* unsafe and able to cause UB regardless of edition. This RFC doesn't have a specific answer on how to improve pre-2024 code.

# Future possibilities
[future-possibilities]: #future-possibilities

None are apparent at this time.
