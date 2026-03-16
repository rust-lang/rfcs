# Stable ABI for Rust via Permanent Type Hashes and Separated Type Systems

- Feature Name: `stable_abi_type_hashes`
- Start Date: 2026-03-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary

This RFC proposes a stable ABI for Rust based on three complementary ideas:

1. **Permanent type hashes** — every primitive and user-defined type gets a stable, compiler-independent hash that never changes across compiler versions.
2. **Three separated type systems** — static types, runtime types (pointers), and compile-only constructs (lifetimes) are handled independently, each in the way that makes sense for their nature.
3. **File-level compilation granularity** — the compilation unit is reduced from the whole crate to individual files, with a project-wide cache to avoid redundant recompilation.

## Motivation

Rust currently has no stable ABI. This means:

- A `.so` compiled with `rustc 1.75` is incompatible with a binary compiled with `rustc 1.80`, even if the source code is identical.
- Dynamic linking of Rust libraries is unreliable in practice.
- The ecosystem depends on static linking by default, producing larger binaries.
- `libstd` cannot be shared dynamically across binaries in a reliable way.

The root cause is that symbol names in Rust binaries include a hash generated from compiler internals, which changes with every compiler version. This makes it impossible to guarantee that two separately compiled Rust artifacts can communicate.

This RFC proposes a solution that gives Rust a stable ABI without sacrificing the expressiveness of its type system, and with near-zero runtime overhead.

## Guide-level explanation

### Permanent type hashes

Every type in Rust is assigned a permanent, stable hash at the language level:

- Primitive types (`u8`, `u32`, `f64`, `String`, etc.) have fixed hashes defined in the language specification. These never change.
- Generic types like `Vec<u32>` derive their hash by combining the hash of `Vec`, the hash of the generic marker `<>`, and the hash of the inner type `u32`. The result is always the same, regardless of which compiler version produced it.
- User-defined structs derive their hash from their name and crate path, which are stable by definition.

This means that the symbol for `Vec<u32>::push` in a `.so` is always the same string, no matter which version of `rustc` compiled it.

### Three separated type systems

Not all types are equal in terms of ABI stability. This RFC separates them into three categories:

**Category 1 — Static types** (`u32`, `String`, `Vec<T>`, structs, enums): these have stable hashes as described above. They participate fully in the stable ABI.

**Category 2 — Runtime types** (raw pointers, `*const T`, `*mut T`): pointer values are memory addresses that change every time the program runs. They cannot have a stable compile-time hash. These types are handled by a separate runtime resolution system, equivalent to how dynamic linkers resolve relocations today. They do not block ABI stability for the rest of the type system.

**Category 3 — Compile-only constructs** (lifetimes): lifetimes exist only in the compiler's analysis phase. They are completely absent from the final binary. There is nothing to hash, nothing to resolve — they are simply invisible to the ABI.

This separation means that the complexity of pointers and lifetimes does not contaminate the stability of the rest of the type system.

### File-level compilation granularity

Currently, Rust compiles an entire crate as a single unit. Changing one line in a 500-file project triggers recompilation of the entire crate.

This RFC proposes treating compilation units the way a filing system works:

- **File** = smallest unit. Recompiled only if it changes.
- **Module/directory** = a folder of files. Recompiled only if any file inside changes.
- **Project** = the full crate. Only re-linked if any module changed.

A cache file at the project root stores the compiled artifacts for each file. On incremental builds, only the changed files are recompiled and the cache is updated. This is analogous tohow C handles `.o` object files.

## Reference-level explanation

### Hash derivation algorithm

For a type `T`, its ABI hash `H(T)` is defined as:

- If `T` is a primitive: `H(T)` is a fixed constant defined in the language spec.
- If `T` is a generic instantiation `G<A>`: `H(T) = combine(H(G), H(<>), H(A))` where `combine` is a stable, order-sensitive hash combination function.
- If `T` is a user-defined type: `H(T) = combine(H(crate_name), H(module_path), H(type_name))`.

The symbol name for a function `fn foo` operating on type `T` becomes `foo_{H(T)}` — a stable, human-readable-ish identifier that does not depend on compiler internals.

### Runtime pointer resolution

Types containing raw pointers are excluded from the static hash system. Their symbols are resolved at load time by the dynamic linker, using the existing platform relocation mechanism (`.rela.dyn` on ELF, etc.). This is transparent to the programmer.

### Lifetime erasure

Lifetimes are erased before code generation. They produce no symbols, no hashes, and no ABI surface. No changes are needed to handle them — they are already invisible to the linker.

### Compilation cache

Each file produces a `.robj` (Rust object) cache artifact stored in a project-level cache directory. The build system checks file modification times and content hashes before deciding whether to recompile. Only files whose content changed since the last build are recompiled.

## Drawbacks

- Defining permanent hashes for all primitive types requires a one-time standardization effort.
- User-defined types that change their crate path (due to refactoring) will change their hash and break binary compatibility. This is expected and correct behavior, but may surprise users.
- The file-level compilation cache adds build system complexity.
- Raw pointer types require explicit opt-out from the static hash system, which may require new syntax or attributes.

## Rationale and alternatives

### Why permanent hashes instead of the current approach?

The current compiler-internal hash is an implementation detail that leaks into the public ABI. Permanent type hashes make the ABI a first-class language feature, not a side effect of compiler internals.

### Why three separate systems?

Because the three categories have fundamentally different properties. Forcing pointers and lifetimes into the same system as `u32` either makes the system too complex to be stable, or forces artificial restrictions on pointer and lifetime semantics. Separation keeps each system simple and correct within its domain.

### Alternative: `#[repr(C)]` everywhere

The existing `#[repr(C)]` mechanism is opt-in and only fixes memory layout, not symbol naming. It does not solve the mangling problem for generics and does not provide a complete ABI solution.

### Alternative: dynamic dispatch (`dyn Trait`)

Dynamic dispatch avoids monomorphization but trades compile-time type information for runtime indirection. It does not solve the ABI problem for non-trait types and imposes a performance cost on all users, not just those who need dynamic linking.

### What if we do nothing?

Rust remains unable to support reliable dynamic linking of Rust-to-Rust code. The ecosystem stays tied to static linking, `libstd` cannot be shared, and interoperability with plugin systems and long-lived shared libraries remains painful.

## Prior art

- **Swift ABI stability (2019)**: Apple froze the Swift ABI by committing to stable type layouts and symbol names. This required significant engineering effort but enabled `libSwiftCore.dylib` to ship with the OS. The key insight — that ABI stability requires treating the ABI as a first-class language commitment — is shared by this RFC.
- **COM (Windows, 1993)**: Uses manually assigned GUIDs per interface to achieve permanent stable identifiers. Proven to work for 30+ years. This RFC automates what COM requires manually.
- **C ABI**: Stable because the type system is simple enough that no compiler-internal information needs to leak into symbol names. This RFC brings the same property to Rust's richer type system.
- **Go**: Achieves ABI stability by avoiding monomorphization (using dynamic dispatch by default) and by controlling the entire toolchain. A different tradeoff with different costs.

## Unresolved questions

- Exact specification of the `combine` hash function (must be stable, collision-resistant, and order-sensitive).
- How to handle generic types with multiple type parameters: `HashMap<K, V>` — proposed: `combine(H(HashMap), H(<>), H(K), H(<>), H(V))`.
- How user-defined types in external crates interact with semver — if a crate renames a type, the hash changes. Should cargo warn about this?
- Whether the file-level cache should be part of `cargo` or the compiler itself.
- How to handle `#[repr(C)]` types — they already have stable layout; their hash should reflect that they are layout-compatible with C.

## Future possibilities

- Once stable hashes exist, Rust plugins (`.so` files loaded at runtime) become practical without `unsafe` FFI wrappers.
- `libstd` could ship as a shared library with the OS, reducing binary sizes significantly.
- The permanent hash system could be extended to function signatures, enabling stable vtable layouts for `dyn Trait` across compiler versions.
- The file-level compilation cache could be shared across machines in distributed build systems (similar to `ccache` or `sccache` for C).
