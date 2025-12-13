- Feature Name: `proc_macro_include`
- Start Date: 2021-11-24
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Proc macros can now effectively `include!` other files and process their contents.
This both allows proc macros to communicate that they read external files,
and to maintain spans into the external file for more useful error messages.

# Motivation
[motivation]: #motivation

- `include!` and `include_str!` are no longer required to be compiler built-ins,
  and could be implemented as proc macros.
- Help incremental builds and build determinism, by proc macros telling rustc which files they read.
- Improve proc macro sandboxability and cacheability, by offering a way to implement this class of
  file-reading macros without using OS APIs directly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## For users of proc macros

Nothing changes! You'll just see nicer errors and fewer rebuilds
from procedural macros which read external files.

## For writers of proc macros

Three new functions are provided in the `proc_macro` interface crate:

```rust
/// Read the contents of a file as a `TokenStream` and add it to build dependency graph.
///
/// The build system executing the compiler will know that the file was accessed during compilation,
/// and will be able to rerun the build when the contents of the file changes.
///
/// May fail for a number of reasons, for example, if the string contains unbalanced delimiters
/// or characters not existing in the language.
///
/// If the file fails to be read, this is not automatically a fatal error. The proc macro may
/// gracefully handle the missing file, or emit a compile error noting the missing dependency.
///
/// Source spans are constructed for the read file. If you use the spans of this token stream,
/// any resulting errors will correctly point at the tokens in the read file.
///
/// NOTE: some errors may cause panics instead of returning `io::Error`.
/// We reserve the right to change these errors into `io::Error`s later.
fn include<P: AsRef<str>>(path: P) -> Result<TokenStream, std::io::Error>;

/// Read the contents of a file as a string literal and add it to build dependency graph.
///
/// The build system executing the compiler will know that the file was accessed during compilation,
/// and will be able to rerun the build when the contents of the file changes.
///
/// If the file fails to be read, this is not automatically a fatal error. The proc macro may
/// gracefully handle the missing file, or emit a compile error noting the missing dependency.
///
/// NOTE: some errors may cause panics instead of returning `io::Error`.
/// We reserve the right to change these errors into `io::Error`s later.
fn include_str<P: AsRef<str>>(path: P) -> Result<Literal, std::io::Error>;

/// Read the contents of a file as raw bytes and add it to build dependency graph.
///
/// The build system executing the compiler will know that the file was accessed during compilation,
/// and will be able to rerun the build when the contents of the file changes.
///
/// If the file fails to be read, this is not automatically a fatal error. The proc macro may
/// gracefully handle the missing file, or emit a compile error noting the missing dependency.
///
/// NOTE: some errors may cause panics instead of returning `io::Error`.
/// We reserve the right to change these errors into `io::Error`s later.
fn include_bytes<P: AsRef<str>>(path: P) -> Result<Vec<u8>, std::io::Error>;
```

As an example, consider a potential implementation of [`core::include`](https://doc.rust-lang.org/stable/core/macro.include.html):

```rust
#[proc_macro]
pub fn include(input: TokenStream) -> TokenStream {
    let mut iter = input.into_iter();

    let result = 'main: if let Some(tt) = iter.next() {
        let TokenTree::Literal(lit) = tt &&
        let LiteralValue::Str(path) = lit.value()
        else {
            Diagnostic::spanned(tt.span(), Level::Error, "argument must be a string literal").emit();
            break 'main TokenStream::new();
        }

        match proc_macro::include(&path) {
            Ok(token_stream) => token_stream,
            Err(err) => {
                Diagnostic::spanned(Span::call_site(), Level::Error, format_args!("couldn't read {path}: {err}")).emit();
                TokenStream::new()
            }
        }
    } else {
        Diagnostic::spanned(Span::call_site(), Level::Error, "include! takes 1 argument").emit();
        TokenStream::new()
    }

    if let Some(_) = iter.next() {
        Diagnostic::spanned(Span::call_site(), Level::Error, "include! takes 1 argument").emit();
    }

    result
}
```

(RFC note: this example uses unstable and even unimplemented features for clarity.
However, this RFC in no way requires these features to be useful on its own.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

If a file read is unsuccessful, an encoding of the responsible `io::Error` is passed over the RPC bridge.
If a file is successfully read but fails to lex, `ErrorKind::Other` is returned.

None of these three APIs should ever cause compilation to fail.
It is the responsibility of the proc macro to fail compilation if a failed file read is fatal.

# Drawbacks
[drawbacks]: #drawbacks

This is more API surface for the `proc_macro` crate, and the `proc_macro` bridge is already complicated.
Additionally, this is likely to lead to more proc macros which read external files.
Moving the handling of `include!`-like macros later in the compiler pipeline
likely is also significantly more complicated than the current `include!` implementation.

# Alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- [`proc_macro::tracked_path`](https://doc.rust-lang.org/stable/proc_macro/tracked_path/fn.path.html) (unstable)

This just tells the proc_macro driver that the proc macro has a dependency on the given path.
This is sufficient for tracking the file, as the proc macro can just also read the file itself,
but lacks the ability to require the proc macro go through this API, or to provide spans for errors.

Meaningfully, it'd be nice to be able to sandbox proc macros in wasm à la [watt](https://crates.io/crates/watt)
while still having proc macros capable of reading the filesystem (in a proc_macro driver controlled manner).

- Custom error type

A custom error wrapper would provide a point to attach more specific error information than just an
`io::Error`, such as the lexer error encountered by `include`. This RFC opts to use `io::Error`
directly to provide a more minimal API surface.

- Wrapped return types

Returning `Literal::string` from `include_str` and `Vec<u8>` from `include_bytes` implies that
the entire included file must be read into memory managed by the Rust global allocator.
Alternatively, a more abstract buffer type could be used which allows more efficiently working
with very large files that could be instead e.g. memmapped rather than read into a buffer.

This would likely look like `LiteralString` and `LiteralBytes` types in the `proc_macro` bridge,
but this RFC opts to use the existing `Literal` and `Vec<u8>` to provide a more minimal API surface.

- Status quo

Proc macros can continue to read files and use `include_str!` to indicate a build dependency.
This is error prone, easy to forget to do, and all around not a great experience.

# Prior art
[prior-art]: #prior-art

No known prior art.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- It would be nice for `include` to allow emitting a useful lexer error directly.
  This is not currently provided for by the proposed API.
- `include!` sets the "current module path" for the included code.
  It's unclear how this should behave for `proc_macro::include`,
  and whether this behavior should be replicated at all.
- Should `include_str` get source code normalization (i.e. `\r\n` to `\n`)?
  `include_str!` deliberately includes the string exactly as it appears on disk,
  and the purpose of these APIs is to provide post-processing steps,
  which could need the file to be reproduced exactly,
  so the answer is likely *no*,
  and the produced `Literal` should represent the exact contents of the file.
- What base directory should relative paths be resolved from?
  The two reasonable answers are

  - That which `include!` is relative to in the source file expanding the macro.
  - That which `fs` is relative to in the proc macro execution.

  Both have their merits and drawbacks.
- Unknown unknowns.

# Future possibilities
[future-possibilities]: #future-possibilities

Future expansion of the proc macro APIs are almost entirely orthogonal from this feature.
As such, here is a small list of potential uses for this API:

- Processing a Rust-lexer-compatible DSL
  - Multi-file parser specifications for toolchains like LALRPOP or pest
  - Larger scale Rust syntax experimentations
- Pre-processing `include!`ed assets
  - Embedding compiled-at-rustc-time shaders
  - Escaping text at compile time for embedding in a document format
