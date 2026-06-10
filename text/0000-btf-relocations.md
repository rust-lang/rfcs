- Feature Name: `btf_relocations`
- Start Date: 2026-05-30
- RFC PR: [rust-lang/rfcs#3966](https://github.com/rust-lang/rfcs/pull/3966)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Add experimental Rust support for [Compile Once, Run Everywhere (CO-RE)][co-re]
relocations based on the [BPF Type Format (BTF)][btf]. The feature introduces a
`#[repr(Btf)]` representation for structs and unions whose field layout must be
queried through BTF-aware operations, and adds BTF-aware macros for accessing
the fields:

* `core::btf::field_byte_offset!`
* `core::btf::field_byte_size!`
* `core::btf::field_exists!`

The user-facing feature is gated by `#![feature(btf_relocations)]`.

## Motivation
[motivation]: #motivation

[BTF][btf] is the type metadata format used by the Linux kernel and eBPF
tooling. eBPF loaders such as [Aya][aya] and [libbpf][libbpf] use BTF for
relocations: the compiled program records which field or array element it
intended to access, and the loader rewrites the bytecode to match the layout of
the kernel it is about to run on.

Clang and GCC are capable of emitting such relocations.

Rust can already target eBPF, but it does not currently have a way to emit
these BTF access relocations. In practice, that means Rust eBPF programs often
have to pick one of three inconvenient options:

- Vendor the exact kernel type definitions and rebuild for each supported kernel
  layout.
- Avoid typed field access and manually encode offsets, sacrificing readability
  and maintainability.
- Write a module in C solely for accessing kernel types and use `build.rs` to
  link it to the Rust project.

`offset_of!` is the wrong primitive for this purpose: it intentionally folds to a
compile-time layout constant, so backend codegen no longer knows which source
field was being queried. Ordinary Rust field projection is also insufficient:
once it becomes a normal memory access, the BTF field identity needed for CO-RE
relocation has been lost.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `btf_relocations` feature is for Rust code that models external BTF
types, primarily Linux kernel types used by eBPF programs.

A type that should participate in BTF relocation is written with
`#[repr(Btf)]`:

```rust
#![feature(btf_relocations)]

#[repr(Btf)]
pub struct task_struct {
    pub pid: i32,
    pub tgid: i32,
}
```

`#[repr(Btf)]` is intentionally not just a layout hint. It marks a type as one
whose fields should not be accessed through ordinary Rust field projection for
accesses that are meant to be relocatable. For such types, direct field access
is rejected:

```rust
#![feature(btf_relocations)]

#[repr(Btf)]
pub struct task_struct {
    pub pid: i32,
}

fn pid(task: &task_struct) -> i32 {
    task.pid
    // error: cannot access fields of a `#[repr(Btf)]` type directly
}
```

The same restriction applies to `offset_of!`:

```rust
#![feature(btf_relocations)]

#[repr(Btf)]
pub struct task_struct {
    pub pid: i32,
}

const PID_OFFSET: usize = core::mem::offset_of!(task_struct, pid);
// error: cannot use `offset_of!` with a `#[repr(Btf)]` type
```

Instead, code that needs field metadata uses BTF-aware queries. These macros
take a root carrier type and a field path:

```rust
#![feature(btf_relocations)]

#[repr(Btf)]
pub struct task_struct {
    pub pid: i32,
    pub tgid: i32,
}

impl task_struct {
    #[inline]
    pub fn has_pid(&self) -> bool {
        core::btf::field_exists!(task_struct, pid)
    }

    #[inline]
    pub fn pid_offset(&self) -> Option<usize> {
        self.has_pid().then(|| {
            Some(core::btf::field_byte_offset!(task_struct, pid))
        })
    }

    #[inline]
    pub fn pid_size(&self) -> Option<usize> {
        self.has_pid().then(|| {
            Some(core::btf::field_byte_size!(task_struct, pid))
        })
    }

    #[inline]
    pub fn pid(&self) -> Option<&i32> {
        self.has_pid().then(|| {
            let offset = core::btf::field_byte_offset!(task_struct, pid);
            let ptr = self as *const task_struct as *const u8;

            // SAFETY: the BTF relocation says that `se.vruntime` exists in the
            // target layout, and the returned offset is relative to `task_struct`.
            Some(unsafe { &*(ptr.add(offset) as *const i32) })
        })
    }

    #[inline]
    pub fn has_tgid(&self) -> bool {
        core::btf::field_exists!(task_struct, tgid)
    }

    #[inline]
    pub fn tgid_offset(&self) -> Option<usize> {
        self.has_tgid().then(|| {
            Some(core::btf::field_byte_offset!(task_struct, tgid))
        })
    }

    #[inline]
    pub fn tgid_size(&self) -> Option<usize> {
        self.has_tgid().then(|| {
            Some(core::btf::field_byte_size!(task_struct, tgid))
        })
    }

    #[inline]
    pub fn tgid(&self) -> Option<&i32> {
        self.has_tgid().then(|| {
            let offset = core::btf::field_byte_offset!(task_struct, tgid);
            let ptr = self as *const task_struct as *const u8;

            // SAFETY: the BTF relocation says that `se.vruntime` exists in the
            // target layout, and the returned offset is relative to `task_struct`.
            Some(unsafe { &*(ptr.add(offset) as *const i32) })
        })
    }
}
```

Nested field paths are supported. For example, access to the fields of
`sched_entity` that is nested in `task_struct` can be done with one macro call:

```rust
#![feature(btf_relocations)]

#[repr(Btf)]
pub struct load_weight {
    pub weight: usize,
}

#[repr(Btf)]
pub struct sched_entity {
    pub load: load_weight,
    pub vruntime: u64,
}

#[repr(Btf)]
pub struct task_struct {
    pub se: sched_entity,
}

impl task_struct {
    #[inline]
    pub fn sched_vruntime(&self) -> Option<&u64> {
        core::btf::field_exists!(task_struct, se.vruntime).then(|| {
            let offset = core::btf::field_byte_offset!(task_struct, se.vruntime);
            let ptr = self as *const task_struct as *const u8;

            // SAFETY: the BTF relocation says that `se.vruntime` exists in the
            // target layout, and the returned offset is relative to `task_struct`.
            Some(unsafe { &*(ptr.add(offset) as *const u64) })
        })
    }

    #[inline]
    pub fn sched_load_weight(&self) -> Option<&usize> {
        core::btf::field_exists!(task_struct, se.load.weight).then(|| {
            let offset = core::btf::field_byte_offset!(task_struct, se.load.weight);
            let ptr = self as *const task_struct as *const u8;

            // SAFETY: the BTF relocation says that `se.load.weight` exists in the
            // target layout, and the returned offset is relative to `task_struct`.
            Some(unsafe { &*(ptr.add(offset) as *const usize) })
        })
    }
}
```

The offset returned for a nested path is relative to the root carrier type,
`task_struct`.

On BPF targets with BTF-capable backend support and debug info enabled, these
queries lower to CO-RE relocations. On targets or backends without BTF
relocation support, they fall back to the current compilation unit's ordinary
layout information.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Feature gate

The language feature is named `btf_relocations`.

The feature gate controls the user-facing BTF relocation surface, including
`#[repr(Btf)]` and the `core::btf` field-info macros.

### `#[repr(Btf)]`

The `repr` attribute accepts a new representation hint:

```rust
#[repr(Btf)]
struct S {
    field: u32,
}
```

`#[repr(Btf)]` is accepted on structs and unions. It is rejected on other item
kinds.

A `#[repr(Btf)]` type uses C-compatible field layout. In compiler terms,
`repr(Btf)` implies the layout constraints of `repr(C)` and also marks the type
as BTF-relocatable. This gives the backend stable field ordering and offsets for
the compile-time fallback while preserving a distinct marker for type checking
and codegen.

Direct field projection from a `#[repr(Btf)]` ADT is rejected. This includes
projections reached through autoderef:

```rust
task.pid
```

The `offset_of!` macro is also rejected when any container in the queried path is
a `#[repr(Btf)]` ADT.

These restrictions avoid silently producing non-relocatable code for operations
that appear to query a relocatable type. Code that genuinely wants a normal
non-relocatable Rust type should not use `#[repr(Btf)]`.

### Field-info macros

The following macros are added under `core::btf`:

```rust
core::btf::field_byte_offset!(Carrier, field.path) -> usize
core::btf::field_byte_size!(Carrier, field.path) -> usize
core::btf::field_exists!(Carrier, field.path) -> bool
```

`Carrier` is the root local Rust type whose BTF graph describes the access.
The second argument is a dot-separated Rust field path starting from `Carrier`.
The compiler type checks this path using the same field lookup rules as
`offset_of!`, except that `#[repr(Btf)]` containers are accepted for these
BTF-aware queries.

The macros have the following meanings:

* `field_byte_offset!(Carrier, field.path)` returns the byte offset of the
  complete field path from the root carrier.
* `field_byte_size!(Carrier, field.path)` returns the byte size of the terminal
  field.
* `field_exists!(Carrier, field.path)` returns whether the complete field path
  exists in the target BTF type.

These macros do not perform memory access. They are metadata queries and do not
require the caller to uphold memory-safety invariants.

Nested paths are supported:

```rust
core::btf::field_byte_offset!(task_struct, se.load.weight)
```

These macros are the only user-facing API for BTF field-info queries.
Implementations may lower them to a dedicated internal compiler operation that
carries the root type, resolved field path, and query kind; that operation is
not exposed as a callable API.

### Backend lowering

For LLVM BPF codegen with debug info enabled, each resolved field-info query
lowers to `llvm.bpf.preserve.field.info` with the corresponding BPF field-info
kind:

* byte offset: `0`
* byte size: `1`
* field exists: `2`

The frontend does not expose LLVM's
`llvm.preserve.{struct,array,union}.access.index` operations. The codegen
backend constructs the required access-index chain internally from the Rust
carrier type and compiler-generated field path.

The result of the LLVM intrinsic is an integer value. Offset and size queries
are zero-extended to `usize`. Existence queries are compared against zero and
return `bool`.

If the target, backend, or codegen mode cannot emit BTF field relocations, the
field-info queries fall back to ordinary layout-computed values:

* `field_byte_offset!` returns the complete field-path offset from the
  current compilation layout.
* `field_byte_size!` returns the field size from the current compilation
  layout.
* `field_exists!` returns `true` for a field path present in the current
  compilation layout.

BTF CO-RE relocation emission is only meaningful for BPF targets, and it
requires the debug metadata used to describe the relevant types.

## Drawbacks
[drawbacks]: #drawbacks

This is a niche feature aimed at one target family and one ecosystem workflow.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Emit BTF relocations in bpf-linker

The main alternative is to emit BTF relocations in [bpf-linker][bpf-linker],
which is a bitcode linker used exclusively for BPF targets. However, it
prevents us from supporting ld type of linkers (e.g. binutils, lld) for BPF
targets in future.

### Use `offset_of!`

`offset_of!` is intentionally a constant layout query. It does not preserve the
field identity needed to emit a BTF relocation. Reusing it would either silently
produce non-relocatable code or require changing the meaning of an existing
stable API in target-specific ways.

### Make ordinary field access relocatable
[relocatable-field-access]: #relocatable-field-access

The compiler could try to make `task.pid` on selected types emit relocatable
accesses automatically. This is what Clang currently does for CO-RE field
accesses: ordinary C field projection can be preserved and lowered to BTF
relocations. This is attractive ergonomically, but it requires a more intrusive
change to MIR and the design of a proper abstract machine with operational
semantics. It also has some similarities to the [`Sized` hierarchy
RFC][sized-hierarchy], which has not yet been accepted.

This overlaps with the accepted [Field Projections project goal][field-projections],
which is exploring virtual places as a general mechanism for custom field
projection. This RFC deliberately does not depend on that work: it provides the
low-level BTF field metadata queries needed for CO-RE today, while leaving a
future field-projection-based ergonomic surface open.

Providing the field-info queries proposed in this RFC does not rule out
exploring this alternative in the future. On the contrary, Clang provides both
explicit field-info builtins and field projection. It makes sense to treat these
as separate RFCs.

### Use a separate `#[relocatable]` attribute

A standalone `#[relocatable]` attribute was considered. `#[repr(Btf)]` is more
specific about the external format and makes clear that the marker affects
representation-related compiler behavior. It also groups the feature with other
layout and representation attributes.

### Expose LLVM intrinsics directly

LLVM already provides BPF intrinsics for preserving access indices and querying
field information. Exposing those directly would leak backend-specific details
into Rust code and make the API unusable for non-LLVM codegen backends.

The proposed Rust field-info queries are backend-neutral. LLVM-specific lowering
remains an implementation detail. Lowering for GCC might be implemented in the
future.

### Implement this entirely in libraries

Libraries can provide ergonomic wrappers, but they cannot make `offset_of!`
preserve field identity after type checking and MIR lowering, nor can they
reliably construct backend metadata for CO-RE relocation emission. The compiler
must participate.

## Prior art
[prior-art]: #prior-art

Clang and LLVM support BPF CO-RE through builtins and LLVM intrinsics such as
`__builtin_preserve_access_index`, `__builtin_preserve_field_info`, and
`llvm.bpf.preserve.field.info`. C BPF programs commonly use libbpf macros such
as `BPF_CORE_READ` and `bpf_core_field_exists` to generate these relocations.

Rust BPF projects such as [Aya][aya] need access to the same relocation
model. Today, they generally rely on generated bindings, helper macros, or
backend behavior outside Rust's stable language surface.

This RFC follows the same underlying CO-RE model as C/Clang while avoiding a
direct dependency on C syntax or LLVM-specific frontend intrinsics.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

* How should non-LLVM codegen backends expose equivalent relocation support?

## Future possibilities
[future-possibilities]: #future-possibilities

Once the [`Sized` hierarchy RFC][sized-hierarchy] is accepted,
[ordinary field access could be made relocatable][relocatable-field-access].

[btf]: https://docs.kernel.org/bpf/btf.html
[co-re]: https://nakryiko.com/posts/bpf-portability-and-co-re/
[aya]: https://github.com/aya-rs/aya
[libbpf]: https://github.com/libbpf/libbpf
[bpf-linker]: https://github.com/aya-rs/bpf-linker
[field-projections]: https://github.com/rust-lang/rust-project-goals/issues/390
[sized-hierarchy]: https://github.com/rust-lang/rfcs/pull/3729
