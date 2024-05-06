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

An `extern` block can be placed anywhere a function declaration could appear (generally at the top level of a module).

* On editions >= 2024, you *must* write all `extern` blocks as `unsafe extern`.
* On editions < 2024, you *may* write `unsafe extern`, or you can write an `extern` block without the `unsafe` keyword. Writing an `extern` block without the `unsafe` keyword is provided for compatibility only, and will generate a warning.
* `unsafe extern` interacts with the `unsafe_code` lint, and a `deny` or `forbid` with that lint will deny or forbid the unsafe external block.

Within an `extern` block is zero or more declarations of external functions and/or external static values.
An extern function is declared with a `;` instead of a function body (similar to a method of a trait).
An extern static value is also declared with a `;` instead of an expression (similar to an associated const of a trait).
In both cases, the actual function body or value is provided by whatever external source (which is probably not even written in Rust).

When an `unsafe extern` block is used, all declarations within that `extern` block *must* have the `unsafe` or `safe` keywords as part of their signature.
The `safe` keyword is a contextual keyword; it is currently allowed only within `extern` blocks.

If an `extern` block is used in an older edition without the `unsafe` keyword, declarations *cannot* specify `safe` or `unsafe`.
Code must update to `unsafe extern` style blocks if it wants to make `safe` declarations.

```rust
unsafe extern {
    // sqrt (from libm) can be called with any `f64`
    pub safe fn sqrt(x: f64) -> f64;

    // strlen (from libc) requires a valid pointer,
    // so we mark it as being an unsafe fn
    pub unsafe fn strlen(p: *const c_char) -> usize;

    // this function doesn't say safe or unsafe, so it defaults to unsafe
    pub fn free(p: *mut core::ffi::c_void);

    pub safe static IMPORTANT_BYTES: [u8; 256];

    pub safe static LINES: SyncUnsafeCell<i32>;
}
```

`extern` blocks are `unsafe` because if the declaration doesn't match the actual external function, or the actual external data, then it causes compile time Undefined Behavior (UB).

Once they are unsafely declared, a `safe` item can be used outside the `extern` block as if it were any other safe function or static value declared within rust.
The unsafe obligation of ensuring that the correct items are being linked to is performed by the crate making the declaration, not the crate using that declaration.

Items declared as `unsafe` *must* still have a correctly matching signature at compile time, but they *also* have some sort of additional obligation for correct usage at runtime.
They can only be used within an `unsafe` block.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The grammar of the langauge is updated so that:

* Editions >= 2024 *must* prefix all `extern` blocks with `unsafe`.
* Editions < 2024 *should* prefix `extern` blocks with `unsafe`, this is a warn-by-default compatibility lint when `unsafe` is missing.

Replace the *Functions* and *Statics* sections with the following:

### Functions
Functions within external blocks are declared in the same way as other Rust functions, with the exception that they must not have a body and are instead terminated by a semicolon. Patterns are not allowed in parameters, only IDENTIFIER or _ may be used. The function qualifiers `const`, `async`, and `extern` are not allowed. If the function is unsafe to call, then the function should use the `unsafe` qualifier. If the function is safe to call, then the function should use the `safe` qualifier (a contextual keyword). Functions that are not qualified as `unsafe` or `safe` are assumed to be `unsafe`.

If the function signature declared in Rust is incompatible with the function signature as declared in the foreign code it is Undefined Behavior to compile and link the code.

Functions within external blocks may be called by Rust code, just like functions defined in Rust. The Rust compiler will automatically use the correct foreign ABI when making the call.

When coerced to a function pointer, a function declared in an extern block has type
```rust
extern "abi" for<'l1, ..., 'lm> fn(A1, ..., An) -> R
```
where `'l1`, ... `'lm` are its lifetime parameters, `A1`, ..., `An` are the declared types of its parameters and `R` is the declared return type.

### Statics
Statics within external blocks are declared in the same way as statics outside of external blocks, except that they do not have an expression initializing their value. If the static is unsafe to access, then the static should use the `unsafe` qualifier. If the static is safe to access (and immutable), then the static should use the `safe` qualifier (a contextual keyword). Statics that are not qualified as `unsafe` or `safe` are assumed to be `unsafe`.

Extern statics can be either immutable or mutable just like statics outside of external blocks. An immutable static must be initialized before any Rust code is executed. It is not enough for the static to be initialized before Rust code reads from it. A mutable extern static is always `unsafe` to access, the same as a Rust mutable static, and as such can not be marked with a `safe` qualifier.

# Drawbacks
[drawbacks]: #drawbacks

This change will induce some churn. Hopefully, allowing people to safely call some foreign functions will make up for that.

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
