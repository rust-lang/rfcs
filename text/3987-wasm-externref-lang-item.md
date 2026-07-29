- Feature Name: `wasm_externref`
- Start Date: 2026-07-28
- RFC PR: (https://github.com/rust-lang/rfcs/pull/3987)
- Tracking Issue: (TBD)

## Summary
[summary]: #summary

Support a new `externref` lang item as an opaque, unforgeable reference to a WebAssembly host value. It lowers to the WebAssembly [externref](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/Value_types/externref) reference type in function signatures, following the same semantics of Clang's well-established (since Clang 15) `__externref_t`. It is a bare-position-only type, legal solely as the top-level type of function parameters, return values, and local bindings (including function pointer signature slots). It cannot appear inside any other type - no references, aggregates, statics, or generic arguments, with this enforced at type-check time.

## Motivation
[motivation]: #motivation

WebAssembly applications embedded in JS environments need to interoperate with foreign JavaScript references.

In the Rust Wasm ecosystem today, toolchains face a number of hard limitations with regard to externref interoperability:

- **Third-party Wrappers are Required**: Without native support in Rust, a third-party wrapper technique must be used instead. Typically, this involves creating a WebAssembly table, inserting the `externref` into that table, then using a numeric index to represent all foreign objects (exposed as `JsValue` in wasm-bindgen). Projects like wasm-bindgen are forced to fill this for functionality that could be in core.
- **Interoperability is Harmed**: A consequence of third party wrappers being required is harming interoperability. Sharing any JS value between separate Rust crates requires a shared representation of the table. wasm-bindgen's `JsValue` does not interoperate with other custom table wrapping systems. This in turn makes specific toolchain foreign wrappers "viral" and results in toolchain silos for all `wasm-*` targets, harming the entire Rust Wasm ecosystem.
- **Performance**: All operations on foreign references require multiple table accesses or updates, resulting in a performance cost that adds up over hot paths. Explicit drops are needed for all values from the table instead of being able to rely on implicit handles.
- **Optimization**: Direct use of `externref` allows more advanced static analysis of Wasm code enabling more optimization opportunities both for optimizers and V8. Without this direct codegen all usages as indirect table references hinder these opportunities for optimization.

The RFC's implementation directly exposes LLVM's existing support with no further low-level changes.

The majority of the design is in making sense of the special properties of `externref` in dealing with its opaque behaviours. We do this by making most usages of it a compile error (including `size_of::<externref>()`).

**The primary use case is a very minimal one:** supporting the marshalling of `externref` from one foreign function call to another foreign function call, and that is it.

This simple design has the benefit that size hierarchy handling is fully elided since **all generic use is banned** - `fn pass<T: Sized>(foo: T) -> T { foo }` cannot monomorphize on `externref` since it fails as a generic argument during type checking according to this RFC.

Wrapper types like `JsValue` in wasm-bindgen and other libraries can be refactored on top of `externref`, enabling better interoperability and performance for the Rust Wasm ecosystem.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`externref` is an opaque handle to a foreign value owned by the host (for example, a JavaScript object, string, or function). You cannot inspect it, dereference it, or store it in memory - you can only receive it and immediately pass it along to hand it back to the host:

```rust
#![feature(wasm_externref)]
use core::arch::wasm32::externref;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn create_ref() -> externref;
    fn use_ref(v: externref);
}

pub fn run() {
    unsafe {
        let r = create_ref(); // a live host reference in a local
        let copy = r; // Copy: refs may be duplicated freely
        use_ref(copy);
        use_ref(r);
    }
}
```

Exported functions may also use it, and the host calls them with values directly:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn roundtrip(v: externref) -> externref {
    v // identity-preserved: on JS hosts, roundtrip(obj) === obj
}
```

`externref` is severely restricted, because WebAssembly reference types cannot exist in linear memory. It may only appear as:

- a function parameter or return type (any ABI, including function pointers),
- the type of a local binding.

Everything else is a compile error:

```rust
struct Holder(externref); // ERROR: no aggregates
static GLOBAL: externref = ...; // ERROR: no statics
let r: &externref = &v; // ERROR: no references
let o: Option<externref> = ...; // ERROR: no generic arguments
let a: [externref; 2] = ...; // ERROR: no arrays
let f = |x: externref| x; // ERROR: no closures
```

These restrictions are what make the type zero-cost: values live only in Wasm locals and on the Wasm operand stack, where the host GC can trace them. There is no `Drop`.

To store references in structs, collections, or across calls, libraries build owned wrapper types over Wasm tables (the one place references may rest), allocating a slot per stored value. This RFC deliberately leaves such storage to libraries.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### The type

`externref` is a lang item defined in `core::arch::wasm32`, available on `wasm32` targets:

```rust
#[lang = "externref"]
#[non_exhaustive]
#[derive(Copy, Clone)]
pub struct externref;

impl !Send for externref {}
impl !Sync for externref {}
```

It is `Copy` (duplicating a reference is a wasm-level no-op), `!Send`/`!Sync` (host references are not portable across threads/instances), and implements `Debug` (`"externref"` output). No other trait implementations are provided.

### Position rules

`externref` is well-formed only as:

1. the top-level type of a function parameter or return type, in any ABI, including `fn`-pointer types' parameter/return slots;
2. the top-level type of a local binding (`let`).

It is rejected in all other positions, including: behind references or raw pointers, as a field of any ADT or tuple, as an array/slice element, in statics and consts, as a generic argument (hence in `Option<T>`, `Vec<T>`, trait type parameters, etc.), in closure parameters or captures, and in `async` contexts where it would be held across an await point (coroutine captures).

Enforcement is at type-check time (WF checks plus typeck writeback for inference-produced types), with a monomorphization-time check as a backstop for codegen-only channels (e.g. coroutine captures). This mirrors Clang's Sema enforcement for `__externref_t`.

### Codegen

`externref` lowers to the LLVM target extension type `target("wasm.externref")` - LLVM's representation of WebAssembly reference types. Target extension types are opaque to LLVM's memory model and cannot be stored to linear memory. The position rules guarantee no value ever requires such a home. Codegen must keep values in SSA form, including under full debuginfo (`dbg.value` only - debuginfo must not force a stack slot, which cannot be lowered for reference types). LLVM currently selects these functions via SelectionDAG only (GlobalISel is disabled for functions using reference types).

Requires the reference-types target feature, which is part of LLVM's default Wasm feature set (since LLVM 19) on all wasm32 targets.

### FFI and ABI

In `extern "C"` (and other ABIs), `externref` passes as a single Wasm externref value. It is FFI-safe by construction. improper_ctypes treats it as a known-good type. Variadic positions are rejected (no memory representation).

## Drawbacks

- A type that opts out of the type system's assumptions. Rust pervasively assumes values are addressable and can appear in generic contexts. externref is the first type excluded from `&T`, aggregates, and generics entirely. These checks are a whole new surface area to guard, and the mono-time backstop means some diagnostics can only be post-mono (with correspondingly worse spans).
- Restricted usefulness without storage. The bare type alone supports only transient marshalling, while real applications always need table-backed wrappers. This requires `global_asm!` (asm_experimental_arch) or future table support. The counterargument, validated by the prototype is that it is the optimal primitive to build table semantics from.
- Ecosystem generics friction. `fn foo<T>(t: T)` cannot accept externref. This is inherent (it is the point of the restriction), but users will encounter it.

## Rationale and alternatives

- Do nothing / post-link rewriting (status quo): wasm-bindgen's `externref` pass demonstrates both the feasibility and cost of this path. It requires a whole-binary rewriting stage, unchecked by the compiler and unavailable to other toolchains (e.g. emscripten-linked Rust). A language type is checked, composable, and toolchain-neutral.
- Asm-only trampolines (no lang type): with `global_asm!`, externref-typed imports/exports and table shims can be authored entirely in assembly, with Rust seeing only slot indices. In prototype work along these lines we can verify this works, but the drawback is that every single binding function requires a custom trampoline, so we are back to the problem of needing a third-party bindgen system. This experimentation itself has been the strongest motivation for the lang item: the minimal shared primitive for further low-level work.
- A general "unaddressable types" mechanism first: designing a reusable `!Addressable` framework is a much larger language effort. This RFC's rules are self-contained and match a shipped precedent (Clang), while still allowing a general mechanism to subsume them later.

## Prior art
[prior-art]: #prior-art

- Clang / LLVM `__externref_t` (since LLVM 15): identical position semantics, with this RFC as a port of its rules to Rust's type system. Emscripten supports it in `EM_JS` and exports.
- wasm-bindgen externref mode: post-link signature rewriting via walrus, as motivating precedent for doing it in the language.
- Wasm-GC languages (MoonBit, Kotlin/Wasm): expose host references as reference-typed values in FFI signatures directly, including storing them in language-level aggregates.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- `core::arch::wasm32::externref` or another location for the type?
- Exact trait story for the bare type - is a `Debug` impl (whose fmt takes `&self`) acceptable as a Self-impl carve-out to the no-references rule, or should the type implement nothing?
- Supporting async functions with liveness checks: `externref` locals that are dead before every suspension point never enter the coroutine state and are sound to permit. This could be supported as an extension of existing liveness checks, while surfacing a spanned type-check-time diagnostic rather than the current monomorphization-time rejection.
- Should `extern "C"` variadics, unions, and #[repr(transparent)] wrappers be diagnosed with dedicated messages (all currently rejected by the general rules)?

## Future possibilities
[future-possibilities]: #future-possibilities

- If a future `Addressable` trait were introduced as a new default-bound axis alongside `Sized` and relaxable via `?Addressable` similarly to `?Sized`, then `externref` could layer with that as the first `Sized + !Addressable` language type, enabling pass-through generics correctly without aggregate containment (`Option<T>`, collections, etc), which requires memory layout.
- Alternatively, if addressability were even to be supported in future say as a first-class externref table in Clang (like a function table), then we could add `&externref` support where `&externref: Sized` holds and supports storage cases.

While the above future directions remain unclear, the minimal proposal here layers with any of the above outcomes.
