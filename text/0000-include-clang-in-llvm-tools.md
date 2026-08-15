- Feature Name: Include Clang in llvm-tools
- Start Date: 2025-08-04
- RFC PR: [rust-lang/rfcs#3847]()
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Include a version of `clang` and `clang++` compiled against Rust LLVM in the `llvm-tools` component in nightly.

# Motivation
[motivation]: #motivation

Allowing user-access to the LLVM pipeline allows for many user-built features, such as cross-language inlining. However, LLVM version mismatching between tools can lead to frustrating problems. Including `clang` and `clang++` in `llvm-tools` allows users to use only the tools that Rust ships with, ensuring consistent versioning.

In future versions of Rust, including a compiler with Rustup could also improve ergonomics for FFI crates, as it could avoid depending on system compilers. See how [Zig's implementation](https://actually.fyi/posts/zig-makes-rust-cross-compilation-just-work/) led to easy cross-compiles in Rust to Macos.

## Background

`clang` and `clang++` are LLVM-based C and C++ compilers mentioned in [official documentation](https://doc.rust-lang.org/rustc/linker-plugin-lto.html):
```bash
# Compile the Rust staticlib
RUSTFLAGS="-Clinker-plugin-lto" cargo build --release
# Compile the C code with `-flto=thin`
clang -c -O2 -flto=thin -o cmain.o ./cmain.c
# Link everything, making sure that we use an appropriate linker
clang -flto=thin -fuse-ld=lld -L . -l"name-of-your-rust-lib" -o main -O2 ./cmain.o
```
Unfortunately, this example does not always work, because it calls system `clang`, which may use a different version of LLVM than Rust. Additionally, even at the same version, there is a potential for problems from mixing base LLVM tools with the Rust fork of LLVM.

Rustup has the ability to install a component called `llvm-tools`, which exposes the llvm tools used by Rust, including `llvm-link` and `llc` - notably, it does not contain a build of `clang` or `clang++`.

## Stability Guarantee

The only stability guarantee is that the versions of `clang` and `clang++` will have the same LLVM version as Rust and the other LLVM tools. It is not guaranteed that `clang` and `clang++` will never break their own interfaces.

## Conclusion

Builds of `clang` and `clang++` should be added to the `llvm-tools` component to enable version matching when working with base LLVM tools.

# Drawbacks
[drawbacks]: #drawbacks

This will increase compile times and require more storage on devices with the `llvm-tools` component installed.

It may also drive more people to use manual compilation processes, which may cause fragmentation or be at odds with the Rust vision.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Users can opt for system `clang` and `clang++` when building projects with LLVM, however there is no guarantee that users will have an appropriate version of the system tools, or that the Rust fork of LLVM won't contain any breaking changes.

# Prior art
[prior-art]: #prior-art

This may help in the goal [Expose experimental LLVM features for GPU offloading](https://rust-lang.github.io/rust-project-goals/2025h1/GPU-Offload.html), as raw LLVM access is particularly useful for GPU compilation libraries.

This was mentioned in [Shipping clang as a Rustup component](https://github.com/rust-lang/rust/issues/56371)

See also the issues for [`llvm-dis`, `llc` and `opt`](https://github.com/rust-lang/rust/issues/55890)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should `clang` and `clang++` be part of the `llvm-tools` component or added as their own component?
