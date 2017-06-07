- Feature Name: metadata_in_import_libs
- Start Date: 2017-02-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

When targetting the MSVC toolchain and building a rust `dylib`, embed rust-specific metadata into the static import library
instead of the DLL.

This RFC is entirely specific to the MSVC toolchain.

# Motivation
[motivation]: #motivation

Rust metadata can be very large. Let's take as an example the `std-xxx.dll` on which `rustc.exe` depends:

```
section   size_on_disk 
               (bytes)
----------------------
.text           358400
.rdata          408064
.data             1024
.pdata           23552
>> .rustc      2136576
.tls               512
.reloc            3072
----------------------
total          2931200
```

Of the 2.8 MiB DLL, 2.0 MiB is taken up by the metadata alone, resulting in a 73% space saving if it were removed. There are currently 37 other
DLLs which would all benefit from having their metadata stripped.

One of the primary uses for `dylib`s is to save space by sharing code when building multiple related binaries (a good example
of this is the rust compiler itself, and associated tools). In practice, the metadata is so large that statically linking all
of the rust binaries would actually reduce the size of the rust installation.

When building a `dylib`, a static import library is already produced by the rust compiler. This import library is required
when later linking against this `dylib`, but does not need to be distributed after linking. The same is true for the
rust metadata, so combining them does not break any existing uses of `dylib`s.

# Detailed design
[design]: #detailed-design

Import libraries are themselves "normal" static libraries. Whenever the rust compiler creates an import library for a `dylib`,
it will add an additional object file to the import library. This object file will contain the rust-specific metadata for the `dylib`,
and the format will be identical to the way metadata is embedded when producing an `rlib`.

Since the import library will itself be a valid `rlib`, metadata extraction from the import library is already implemented.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

For most users, this change should not be visible, so documentation does not need to be particularly prominent. However, currently it is
not documented anywhere that rust produces an import library when building a `dylib`. We should at least extend the "linkage" section
of the rust reference to describe these platform differences.

This change brings rust more in line with the behaviour of MSVC, where linking to a dynamic library is achieved by linking to its
import library as though it were a normal static library. For users familiar with MSVC, this should be less surprising than the
previous behaviour.

# Drawbacks
[drawbacks]: #drawbacks

The only drawback is in the behaviour difference across platforms.

There are no existing use-cases that would be broken by this change, because it is already impossible to link to rust `dylib`s without
the corresponding import library.

# Alternatives
[alternatives]: #alternatives

- Implement a tool to strip the metadata from a `dylib` before distributing.
  
- Do nothing.

# Unresolved questions
[unresolved]: #unresolved-questions

Is a similar saving possible for other targets?
