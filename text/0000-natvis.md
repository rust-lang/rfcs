- Feature Name: `natvis`
- Start Date: 2021-11-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC aims to improve the debugging experience for Rust developers, by
enabling Rust developers to package debugger visualization scripts with their
crates.

# Motivation
[motivation]: #motivation

Most, if not all, Rust developers will at some point have to debug an issue
in their crate. Trying to view types as they are laid out in memory is not
always the most telling. Furthermore when viewing types from external crates,
the information is even harder to interpret. 

Many languages and debuggers enable developers to control how a type is
displayed in a debugger. These are called "debugger visualizations" or "debugger
views". Debugger views are merely a convenience for some types, such as
`Vec<T>`, but are essential for types such as `HashMap<T>`, where non-trivial
logic is needed in order to correctly display the contents of a type.

Currently, Rust provides visualizations for a handful of types defined in its
standard library via `.natvis` files or python scripts. However, this support
is inflexible; updating it requires modifying the Rust toolchain itself,
and either using a local build of the toolchain or waiting for a new upstream
build of the toolchain. It is not feasible for developers of ordinary crates
to update the Rust toolchain, solely to add visualizations for their crates.

The expected outcome of this RFC is to design a way for developers to
seamlessly integrate debugger visualizations with their crates. This would mean:

* Any developer can add debugger visualizations to their crate.
* If a Rust developer uses a crate that has debugger visualizations in it,
  then the visualizations of those external crates will "just work" when viewed
  under a debugger without the need of any manual configuration.
* Supports existing debugging visualization systems. We do not propose to
  define a new debugger visualization system; that would be a tremendous
  undertaking, and would ignore the value of existing systems.
* No impact on code quality or size.
* No impact on crates that do not use debugger visualizations.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC explores making Natvis debugger visualizations extensible, in Rust.
The scenario that we want to enable is:

* Alice publishes a crate, say, `cool_stuff`. Alice wrote debugger
  visualizations for `cool_stuff`, and included them in the crate.
* Bob is writing a new Rust application. Deep in the crate dependency graph of
  Bob's application, some crate uses `cool_stuff`.
  (Bob is not even aware of the existence of debugger visualizations.)
* While Bob is debugging the application, and examining data structures,
  he comes across an instance of `cool_stuff::CoolType` in the debugger.
  Because Rust and the debugger know about the visualizations that Alice wrote,
  the `CoolType` value is displayed using its defined debugger view in the debugger.
  Bob did not need any knowledge, a priori, of how debugger visualizations
  worked or that Alice had written any debugger visualizations. From Bob's
  point of view, debugging `CoolType` "just worked".

## An example: The `regex` crate

To make this less hypothetical, let's consider an important community crate,
one which would benefit from debugger visualizations, such as the `regex`
crate. Carol is writing an app that uses `regex` to scan over large input files.
The app code looks something like:

```rust
// search for "#define FOO nnn"
fn find_c_defines(input: &str) {
    let rx = Regex::new(r#"^#define\s+(\w+)\s+([0-9]+)\s*(//(.*))?"#).unwrap();
    for captures in rx.captures_iter(input) {
        let my_match: Match = captures.get(1).unwrap();
        do_some_work(my_match.as_str());
    }
}
```

Let's say that Carol is debugging the app, there's a problem in
`do_some_work()`. (Perhaps some code path has triggered a panic.) Carol wants
to look at the state of the app, inside the `find_c_defines` function,
and she specifically wants to see what the state of `captures` is. So she
selects the `find_c_defines` call frame and looks at the local variables
window.

Unfortunately, the debugger's view of the `captures` variable does not give
her any useful information at all. It shows only something like:

```text
> Variables
  > captures: {...}
    > text: "...the entire input text..."
    > locs: {...}
      > __0: (4) vec![None, None, None, None]
      > named_groups: (refs:2) size=0, capacity=1
        > [raw]: alloc::sync::Arc<std::collections::hash::map::HashMap<...>>
          > ptr: {pointer:0xNNNNNNNN}
            > pointer: {...}
              > strong: {...}
              > weak: {...}
              > data: size=0, capacity=1
                > base: {...}
                  > hash_builder: {...}
        ...
```

The debugger shows the structure of the data, not its meaning. It is useless.
Even the implementor of `regex` would have a hard time knowing how to decode
this. In reality, when trying to understand the state of the captures
variable there are several methods defined for a `Captures` type that paint
the actual picture in terms of the information a Rust developer would like to
extract from this variable. In order to meaningfully understand what the `Captures`
type is truly trying to tell us, it would be very helpful to visualize this data
differently in the debugger.

What we _want_ is something like this:

```text
> Variables:
  > captures: {...}
    > $1: "SOME_CONSTANT"
    > $2: "42"
    > $3: "// some developer comment"
```

This RFC will describe how to support adding Natvis visualizations which is supported by:

* The Windows Debugger (WinDbg)
* Visual Studio Debugger.

It should be easy for crate developers to add debugger visualizations to their
crates.

## Supporting Natvis

This section describes how Microsoft's Natvis is supported in Rust.

To use Natvis, developers write XML documents that describe how debugger types
should be displayed. (See: https://docs.microsoft.com/en-us/visualstudio/debugger/create-custom-views-of-native-objects?view=vs-2019)
The Natvis XML files provide patterns, which match type names, and for matching
types, a description of how to display those types. This allows for some
limited support for generic types.

When writing Natvis files for C++, developers write a standalone XML document
and add it to their project. The build system (such as Visual Studio) knows
how to package the Natvis file into the debug data (the PDB file) for the
project, and the debugger knows how to find the Natvis XML in each PDB file.

Developers can add one or more standalone Natvis XML files to their crate.
The `Cargo.toml` file specifies the name of these Natvis files or the Natvis
files can be specified via a command line option. This is the easiest way to
add Natvis support to a project.

The advantage of a standalone XML document is that this process is already
well-understood by many developers. This will help C++ developers move from
C++ to Rust. It also avoids any need to modify Rust source code. If a code
base uses code generation (such as `bindgen` or proc-macros), then standalone
Natvis XML files would be the only way to provide visualizations for those
types.

### Standalone Natvis XML files

To provide standalone Natvis XML files, developers create a file with the
`.natvis` file extension. These Natvis files are then specified in the 
`Cargo.toml` file via a new key or via the command line using the `-Z natvis`
option.

As an example, consider a crate with this directory structure:

```text
/Cargo.toml
  +-- src
      +-- main.rs
      +-- main.natvis
```

Where `main.rs` contains:

```rust
/// A rectangle in first quadrant
struct FancyRect {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}
```

and `main.natvis` contains:

```xml
<?xml version="1.0" encoding="utf-8"?>
<AutoVisualizer xmlns="http://schemas.microsoft.com/vstudio/debugger/natvis/2010">
    <Type Name="my_crate::FancyRect">
      <DisplayString>({x},{y}) + ({dx}, {dy})</DisplayString>
      <Expand>
        <Synthetic Name="LowerLeft">
          <DisplayString>({x}, {y})</DisplayString>
        </Synthetic>
        <Synthetic Name="UpperLeft">
          <DisplayString>({x}, {y + dy})</DisplayString>
        </Synthetic>
        <Synthetic Name="UpperRight">
          <DisplayString>({x + dx}, {y + dy})</DisplayString>
        </Synthetic>
        <Synthetic Name="LowerRight">
          <DisplayString>({x + dx}, {y})</DisplayString>
        </Synthetic>
      </Expand>
    </Type>
</AutoVisualizer>
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Cargo would add a `-Z natvis={comma-separated list of .natvis files}` flag,
and forward this flag to rustc.

`Cargo.toml` would add a new syntax for specifying the list of Natvis files to
be added to the crate. The new manifest key, `natvis` would be added to the
`[package]` section. This would be in control of setting the `-Z natvis` flag
that would be passed on to rustc. 

We would also add a `-Z natvis={comma-separated list of .natvis files}` flag
to rustc, which instructs the compiler to take the set of .natvis files for
a given crate and store them in the metadata. When running the linker, using
the `MSVC` toolchain, the `/NATVIS` linker option would be set and passed the
total set of .natvis files from all crate dependencies, if any exist, as well
as the current crate and embed them into the pdb. Since the `MSVC` linker is
the only one that supports embedding natvis files into a pdb, this feature
would be specific to the `MSVC` toolchain only.

# Drawbacks
[drawbacks]: #drawbacks

One drawback here is that a lot of types implement the Debug trait which
already specifies how a type should be viewed when debugging. Implementing
this RFC would mean a Rust developer would have to manually specify the
Natvis for a type that may already have implemented the Debug trait which
would be redundant. Currently, running the Debug trait in the debugger directly
is not possible and so a manual definition would be required to have a debugger view.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design provides a simple mechanism for cargo to collect the list of
.natvis files specified for a given crate and embed them in the resulting
pdb. It does not need any manual intervention by a Rust developer who is
consuming such a crate to get the debugging experience to work when it is
viewed under a debugger that supports the Natvis Framework.

This design does not break any existing usage of cargo or rustc. This new feature
would be strictly opt-in. The Natvis syntax may not be familiar to many Rust
developers which may lead to a period of learning the syntax. Since this feature
would be optional, a consumer of a crate that has natvis definitions for types
would not need to go through this learning curve. 

Not doing this would keep the existing debugging experience for external Rust crates.
Most Rust types, outside of the standard library, do not have any debugger views
defined for them by default which makes them difficult to interpret when viewed
under a debugger.

# Prior art
[prior-art]: #prior-art

Many debuggers and languages already address this problem. Some do so in a way
that is more flexible than others.

Briefly, we cover some of the known systems for debugger views:

* Microsoft Natvis (Native Visualizers)
* Microsoft `[DebuggerDisplay]` in .NET

## Microsoft Natvis

Natvis is a framework that customizes how native types appear when viewed
under a debugger. The Visual Studio Natvis framework is supported out of the
box on the Windows Debugger(WinDBG) and the VS debugger. Natvis files are
essentially XML files that use the Natvis syntax to describe how to visualize
types to the debugger. This allows users to more easily interpret the data that
any given type holds.

Taking a look at the previous Natvis example for the `FancyRect` type, the
resulting debugger view of this would be:

```text
> Variables:
  > fancy_rect: (10, 10) + (5, 5)
    > LowerLeft: (10, 10)
    > UpperLeft: (10, 15)
    > UpperRight: (15, 15)
    > LowerRight: (15, 10)
```

The MSVC linker supports embedding debugger visualizations defined in a Natvis file
(`.natvis`) into a PDB generated by LINK through the use of the `/NATVIS` linker flag.

## Microsoft `[DebuggerDisplay]` and `ToString()` in .NET

The .NET `[DebuggerDisplay]` attribute controls how objects, properties or fields
are to be displayed in the debugger. The `[DebuggerDisplay]` attribute
takes a single argument, the string to be displayed in the debugger.
Text within a pair of braces (`{``}`) is evaluated as a field, property, or method.

If a class has an overridden `ToString()` method, then the debugger displays the
results of the `ToString()` method and a `[DebuggerDisplay]` attribute is not required.
One setback here is that the debugger is unable to display the result of the
`ToString()` when viewing a crash dump.

```csharp
/// A rectangle in first quadrant
[DebuggerDisplay("({x},{y}) + ({dx}, {dy})")]
public class FancyRect {
    double x;
    double y;
    double dx;
    double dy;
}
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Is the `[package]` section of the `Cargo.toml` manifest the best place to add this new syntax?

# Future possibilities
[future-possibilities]: #future-possibilities

## Inline Natvis XML fragments via an attribute

Natvis support for Rust could be improved upon by adding support for natvis in source via an attribute. Example:

```rust
/// A rectangle in first quadrant
#[dbgvis(
    natvis(r#"
        <DisplayString>({x},{y}) + ({dx}, {dy})</DisplayString>
        <Item Name="LowerLeft">({x}, {y})</Item>
        <Item Name="UpperLeft">({x}, {y + dy})</Item>
        <Item Name="UpperRight">({x + dx}, {y + dy})</Item>
        <Item Name="LowerRight">({x + dx}, {y})</Item>
    "#))]
struct FancyRect {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}
```

## Inline Natvis XML fragments via a macro

We may want to allow developers to provide Natvis descriptions using a
pseudo macro-call syntax, rather than an attribute. One disadvantage of
using attributes is that, lexically, attributes must be specified at the
definition of a type. Since Natvis descriptions could be quite large, this
would make it hard to read or edit the type definition while also seeing the
rustdoc comments.

To solve this, we could define a `natvis!` macro, and use it like so:

```rust
use std::dbgvis::natvis;

/// A rectangle in first quadrant
struct FancyRect {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

natvis!(FancyRect, r#"
    <DisplayString>({x},{y}) + ({dx}, {dy})</DisplayString>
    <Item Name="LowerLeft">({x}, {y})</Item>
    <Item Name="UpperLeft">({x}, {y + dy})</Item>
    <Item Name="UpperRight">({x + dx}, {y + dy})</Item>
    <Item Name="LowerRight">({x + dx}, {y})</Item>
"#);
```

The `natvis!` call would specify the name of the type the visualization applies
to, along with the XML fragment. This would give developers the freedom to
place visualizations anywhere in their crate, rather than at the definition
of each type.

## Auto-discover Natvis XML files

We may want to auto-discover Natvis files by searching specific directories
for .natvis files. For example, developers create a file with the
`.natvis` file extension, and place it within the `dbgvis/natvis` subdirectory
of their crate. The `dbgvis` directory is reserved for debugger visualizations,
and the `natvis` subdirectory is reserved for Natvis visualizations. (The
name `dbgvis` was chosen to avoid conflicts with `Debug` directories created
by build systems or IDEs; often, `.gitignore` files ignore `Debug` directories.)

Cargo automatically scans for `dbgvis/natvis/*.natvis` files. This behavior
can be overridden by specifying manifest keys.

# References

* Natvis
  + [Create custom views of C++ objects in the debugger using the Natvis framework](https://docs.microsoft.com/en-us/visualstudio/debugger/create-custom-views-of-native-objects)
  + [Visual Studio native debug visualization (natvis) for C++/WinRT](https://docs.microsoft.com/en-us/windows/uwp/cpp-and-winrt-apis/natvis)
  + https://docs.microsoft.com/en-us/windows-hardware/drivers/debugger/native-debugger-objects-in-natvis

* .NET `[DebuggerDisplay]`
  + https://docs.microsoft.com/en-us/visualstudio/debugger/using-the-debuggerdisplay-attribute