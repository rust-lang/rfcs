- Feature Name: `project-inline-asm`
- Start Date: 2019-12-07
- RFC PR: [rust-lang/rfcs#2836](https://github.com/rust-lang/rfcs/pull/2836)
- Rust Issue: [rust-lang/rust#29722](https://github.com/rust-lang/rust/issues/29722)

# Summary
[summary]: #summary

To create a [project group] with the purpose of designing subsequent RFCs to extend the language to support inline assembly in Rust code.

# Motivation
[motivation]: #motivation

In systems programming some tasks require dropping down to the assembly level. The primary reasons are for performance, precise timing, and low level hardware access. Using inline assembly for this is sometimes convenient, and sometimes necessary to avoid function call overhead.

The inline assembler syntax currently available in nightly Rust is very ad-hoc. It provides a thin wrapper over the inline assembly syntax available in LLVM IR. For stabilization a more user-friendly syntax that lends itself to implementation across various backends is preferable.

# Project group details

[Repository][asm project]

[Zulip stream][zulip]

Initial shepherds:

* [Amanieu (Amanieu d'Antras)](https://github.com/Amanieu)

Lang team liaisons:

* [joshtriplett (Josh Triplett)](https://github.com/joshtriplett)

# Charter
[charter]: #charter

The main goal of the asm project group is to design and implement an `asm!` macro using a syntax that we feel we can maintain, easily write, and stabilize.

The project group has the following additional goals:
* to provide a transition path for existing users of the unstable `asm!` macro.
* to ensure that the chosen `asm!` syntax is portable to different compiler backends such as LLVM, GCC, etc.
* to provide a fallback implementation on compiler backends that do not support inline assembly natively (e.g. [Cranelift][cranelift]).
* to initially support most major architectures (x86, ARM, RISC-V) with the intention of extending to other architectures in the future.

With a lower priority, the project group also intends to tackle the following secondary, future goals:
* support for module-level assembly (`global_asm!`).
* support for naked functions (`#[naked]`).

[asm project]: https://github.com/rust-lang/project-inline-asm
[zulip]: https://rust-lang.zulipchat.com/#narrow/stream/216763-project-inline-asm
[cranelift]: https://github.com/CraneStation/cranelift/issues/444
[project group]: https://github.com/rust-lang/wg-governance/blob/master/draft-rfcs/working-group-terminology.md
