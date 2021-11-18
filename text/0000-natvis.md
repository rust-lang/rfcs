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

Since Natvis is supported via WinDbg and the VS Debugger, this support is
specific to Windows and the MSVC toolchain.

For example, given the following instance of `HashMap<T>`:
```rust
fn main() {
    let mut map = HashMap::new();
    map.insert(1, 1);
    map.insert(2, 2);
    map.insert(3, 3);
}
```

Viewed under the Windows Debugger (WinDbg), the following is shown:
```text
> Variables
  > map: [Type: std::collections::hash::map::HashMap<i32,i32,std::collections::hash::map::RandomState>]
    > [+0x000] base: [Type: hashbrown::map::HashMap<i32,i32,std::collections::hash::map::RandomState,alloc::alloc::Global>]
      > [+0x000] hash_builder: [Type: std::collections::hash::map::RandomState]
      > [+0x010] table: [Type: hashbrown::raw::RawTable<tuple$<i32,i32>,alloc::alloc::Global>]
        > [+0x000] table: [Type: hashbrown::raw::RawTableInner<alloc::alloc::Global>]
          > [+0x000] bucket_mask: 0x3 [Type: unsigned __int64]
          > [+0x008] ctrl [Type: core::ptr::non_null::NonNull<u8>]
          > [+0x010] growth_left: 0x0 [Type: unsigned __int64]
          > [+0x018] items: 0x3 [Type: unsigned __int64]
          > [+0x000] alloc: [Type: alloc::alloc::Global]
        > [+0x000] marker: [Type: core::marker::PhantomData<tuple$<i32,i32> >]
        ...
```

With Natvis applied, WinDbg results in the following:
```text
> Variables
  > map: { len=0x1 } [Type: std::collections::hash::map::HashMap<i32,i32,std::collections::hash::map::RandomState>]
    > [<Raw View>] [Type: std::collections::hash::map::HashMap<i32,i32,std::collections::hash::map::RandomState>]
    > [len]: 0x1 [Type: unsigned __int64]
    > [capacity]: 0x3
    > [state]: [Type: std::collections::hash::map::RandomState]
    > ["1"]: 1 [Type: int]
    > ["2"]: 2 [Type: int]
    > ["3"]: 3 [Type: int]
```

Currently, Rust provides visualizations for a handful of types defined in its
standard library via `.natvis` files or python scripts. However, this support
is inflexible; updating it requires modifying the Rust toolchain itself,
and either using a local build of the toolchain or waiting for a new upstream
build of the toolchain. It is not feasible for developers of ordinary crates
to update the Rust toolchain, solely to add visualizations for their crates.

The expected outcome of this RFC is to design a way for developers to seamlessly
integrate Natvis debugger visualizations with their crates. This would mean:

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
  Since the Rust compiler has embedded the Natvis visualizations that Alice wrote
  into the debuginfo for the binary and the debugger is able to load up and serve the
  Natvis visualizations, the `CoolType` value is displayed using its defined debugger
  view in the debugger. Bob did not need any knowledge, a priori, of how debugger
  visualizations worked or that Alice had written any debugger visualizations.
  From Bob's point of view, debugging `CoolType` "just worked".

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

The debugger shows the structure of the data, not its meaning. It is not very
useful for Carol. Even the implementor of `regex` would have a hard time knowing
how to decode this. In reality, when trying to understand the state of the captures
variable there are several methods defined for a `Captures` type that paint
the actual picture in terms of the information a Rust developer would like to
extract from this variable. In order to meaningfully understand what the `Captures`
type is truly trying to tell us, it would be very helpful to visualize this data
differently in the debugger.

What we _want_ is something like this:

```text
> Variables:
  > captures: {...}
    > 1: "SOME_CONSTANT"
    > 2: "42"
    > 3: "// some developer comment"
```

This RFC will describe how to support adding Natvis visualizations which is supported by:

* The Windows Debugger (WinDbg)
* Visual Studio Debugger.

It should be easy for Rust developers to add debugger visualizations to their
crates.

## Supporting Natvis

This section describes how Microsoft's Natvis is supported in Rust.

To use Natvis, developers write XML documents that describe how debugger types
should be displayed using the natvis schema. (See: https://docs.microsoft.com/en-us/visualstudio/debugger/create-custom-views-of-native-objects?view=vs-2019)
The Natvis files provide patterns, which match type names, and for matching
types, a description of how to display those types. This allows for some
limited support for generic types.

Rust developers can add one or more `.natvis` files to their crate. The
`Cargo.toml` file specifies the path of these Natvis files via a new
manifest key. Cargo would then pass the set of `.natvis` files that were
specified in `Cargo.toml` and pass it to the rustc invocation for the crate
via a new `-Z` flag. See the below example for how rustc would embed these
Natvis files in the debuginfo for a binary as well as new compiler options
to be added to rustc.

To provide Natvis files, developers create a file with the `.natvis` file
extension and specify this Natvis file in the `Cargo.toml` under a new section
that will be added as part of this RFC.

As an example, consider a crate `foo` with this directory structure:

```text
/Cargo.toml
/Foo.natvis (Note: the .natvis file does not have to match the name of the crate.)
  +-- src
      +-- main.rs
```

Where `main.rs` contains:

```rust
/// A rectangle in first quadrant
struct FancyRect {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
}

fn main() {
    let mut fancy_rect = FancyRect::new(10.0, 10.0, 5.0, 5.0);
    println!("FancyRect: {:?}", fancy_rect);
}
```

and `Foo.natvis` contains:

```xml
<?xml version="1.0" encoding="utf-8"?>
<AutoVisualizer xmlns="http://schemas.microsoft.com/vstudio/debugger/natvis/2010">
    <Type Name="foo::FancyRect">
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

When viewed under WinDbg, the `fancy_rect` variable would be shown as follows:

```text
> Variables:
  > fancy_rect: (10, 10) + (5, 5)
    > LowerLeft: (10, 10)
    > UpperLeft: (10, 15)
    > UpperRight: (15, 15)
    > LowerRight: (15, 10)
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In rustc, a new `-Z natvis={comma-separated list of .natvis files}` flag
will be added which instructs the compiler to take the set of .natvis
files for a given crate and store them in the crate metadata. When running the
linker, using the `MSVC` toolchain, a `/NATVIS` linker option would be set
for each `.natvis` file. This includes `.natvis` files from all crate dependencies,
if any exist, as well as the current crate and embed them into the PDB.

The MSVC linker supports embedding debugger visualizations defined in a Natvis file
(`.natvis`) into a PDB generated by LINK through the use of the `/NATVIS` linker flag.
Since the `MSVC` linker is the only one that supports embedding Natvis files
into a PDB, this feature would be specific to the `MSVC` toolchain only.

Cargo would add new syntax in `Cargo.toml` for specifying the list of Natvis
files to be added to the crate. The new manifest key, `natvis` would be added
to a new `[debug-visualizations]` section. This would be in control of setting
the `-Z natvis` flag in rustc.

For example:

`Cargo.toml`:

```toml
cargo-features = ["natvis"]

[package]
name = "natvis"
version = "0.1.0"
edition = "2018"

[debug-visualizations]
natvis = ["a.natvis", "b.natvis"]
```

This would generate a call to rustc similar to the following,
(for simplicity purposes, most of the rustc command line has been removed):

`rustc -Z natvis=path/to/file/a.natvis,path/to/file/b.natvis`

The `CrateRoot` type would also need to be updated to account for `.natvis`
files for crates within the dependency graph. To reflect this a new field,
`natvis_files: Lazy<[String]>,` would be added. This will store the list of
`.natvis` files that were passed to the invocation of rustc for the specific crate.

Another change that would need to be made here is to add a new field to the
`CrateInfo` type, `pub natvis_files: Vec<String>`. This will allow the `MsvcLinker`
type to query the list of Natvis files that exist within the crate dependency graph
and add the `/NATVIS` linker arg for each `.natvis` file.

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

* Windows Debugger (WinDbg)
* Visual Studio Debugger (VS Debugger)
* GDB/LLDB

## Windows Debugger (WinDbg)

Natvis is a framework that customizes how native types appear when viewed under
a debugger. The Visual Studio Natvis framework is supported out of the box on
WinDbg. The debugger has the ability to load `.natvis` files via the `.nvload`
command and directly apply them to types within loaded modules. WinDbg is also
able to load `.natvis` files that have been embedded in the PDB for a binary and
serve up the resulting views after applying those visualizations as well. This
allows for a very smooth debugging experience which would not depend on any manual
loading of Natvis files.

## Visual Studio Debugger (VS Debugger)

The Visual Studio Debugger also supports Natvis. Similar to WinDbg, the VS Debugger
is also able to apply Natvis on the fly by loading user-specified `.natvis` files.
As with WinDbg, it also supports loading `.natvis` files that were embedded in the
PDB for a binary and automatically applying the Natvis visualizations to types from
that binary.

When using Visual Studio to build a C++ project, a developer can add a Natvis file
via the `.vcxproj` file. To add a Natvis file to a project the following can be
added to the `.vcxproj` file:

```text
<ItemGroup>
  <Natvis Include="Foo.natvis" />
</ItemGroup>
```

## GDB/LLDB

GDB and LLDB also support debugger views but in a different way than WinDbg and the
VS debugger. Natvis is not supported by either GDB or LLDB but they do support pretty
printers. Pretty printers work in the similar manner as Natvis in which they tell
the debugger to serve up a specific visualization when viewing a type in the debugger.
Pretty printers are written as python scripts and then have to be imported in to the
debugger. When a type is viewed under the debugger that has a pretty printer, that view
is automatically shown.

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