- Feature Name: `debugger-visualizer`
- Start Date: 2021-11-01
- RFC PR: [rust-lang/rfcs#3191](https://github.com/rust-lang/rfcs/pull/3191)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC aims to improve the debugging experience for Rust developers, by
enabling Rust developers to package debugger visualizer scripts with their
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
standard library via `.natvis` files or pretty printers via python scripts.
However, this support is inflexible; updating it requires modifying the Rust
toolchain itself, and either using a local build of the toolchain or waiting
for a new upstream build of the toolchain. It is not feasible for developers of
ordinary crates to update the Rust toolchain, solely to add visualizations for
their crates.

The expected outcome of this RFC is to design a way for developers to seamlessly
integrate debugger visualizations with their crates. This would mean:

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

This RFC explores making debugger visualizations extensible in Rust via Natvis and/or pretty printers.
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

The same should be applied to pretty printers defined and viewed under LLDB and GDB.

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

This RFC will describe how to support adding Natvis as well as GDB's pretty printers.

Natvis is supported by:

* The Windows Debugger (WinDbg)
* Visual Studio Debugger

Pretty printers are supported by:

* GDB
* LLDB

It should be easy for Rust developers to add debugger visualizations to their
crates.

## Supporting Natvis

This section describes how Microsoft's Natvis is supported in Rust.

To use Natvis, developers write XML documents that describe how debugger types
should be displayed using the natvis schema. (See: https://docs.microsoft.com/en-us/visualstudio/debugger/create-custom-views-of-native-objects?view=vs-2019)
The Natvis files provide patterns, which match type names, and for matching
types, a description of how to display those types. This allows for some
limited support for generic types.

Rust developers can add one or more `.natvis` files to their crate. Through
the use of a new Rust attribute, `#[debugger-visualizer]`, the compiler will
encode the contents of the `.natvis` file in the crate metadata if the target
is an `rlib`. If the target is a `dll` or `exe`, the `/NATVIS` MSVC linker flag is
set for each `.natvis` file which will embed the Natvis visualizations into the PDB.

To provide Natvis files, developers create a file with the `.natvis` file
extension.

See the below example for how rustc would embed these Natvis files in the
debuginfo for a binary.

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

## Supporting Pretty Printers

This section describes how GDB's pretty printers are supported in Rust.

To use a pretty printer, developers write python scripts that describe how a type
should be displayed when loaded up in GDB/LLDB. (See: https://sourceware.org/gdb/onlinedocs/gdb/Pretty-Printing.html#Pretty-Printing)
The pretty printers provide patterns, which match type names, and for matching
types, descibe how to display those types. (For writing a pretty printer, see: https://sourceware.org/gdb/onlinedocs/gdb/Writing-a-Pretty_002dPrinter.html#Writing-a-Pretty_002dPrinter).

Rust developers can add one or more pretty printers to their crate. This is done
in the Rust compiler via `.py` python scripts. Through the use of a new Rust attribute,
`#[debugger-visualizer]`, the compiler will encode the contents of the `.py` file in
the crate metadata if the target is an `rlib`. If the target is an executable, the
`.debug_gdb_scripts` section will include a reference to the pretty printer specified.

To provide pretty printers, developers create a file with the `.py` file
extension.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In rustc, a new built-in attribute `#[debugger-visualizer]` will be added which
instructs the compiler to take the specified file path for a debugger visualizer
and add it to the current binary being built. The file path specified must be
relative to the location of the attribute.

The `#[debugger-visualizer]` attribute will reserve multiple keys to be able to
specify which type of visualizer is being applied. The following keys will be
reserved as part of this RFC:

* `natvis_file`
* `gdb_script_file`

As more visualizer schemes arise, more keys may be added in the future to ensure
a great debugging experience for any debugger that the Rust community sees fit.

For example, to specify that a `.natvis` file should be included in the binary
being built, the following attribute should be added to the Rust source:

```rust
#![debugger-visualizer(natvis_file = "../foo.natvis")]
```

The same can be done to specify a GDB python debugger script:

```rust
#![debugger-visualizer(gdb_script_file = "../foo.py")]
```

Depending on the Rust target, the correct debugger visualizer will be selected and embedded
in the output.

The Rust compiler will serialize the contents of the file specified via the
`#[debugger-visualizer]` attribute and store it in the crate metadata. This attribute
can be used multiple times to allow for multiple debugger visualizer files to be
embedded for each crate. When generating the final binary, the contents of the
visualizer file will be extracted from the crate metadata and written to a new file
in the target directory under a new `visualizer` directory.

In the case of a Natvis file, `#![debugger-visualizer(natvis_file = "../foo.natvis")]`
the compiler will set the `/NATVIS:{.natvis file}` MSVC linker flag for each of the
Natvis files specified for the current crate as well as transitive dependencies if
using the MSVC toolchain. This linker flag ensures that the specified Natvis files
be embedded in the PDB generated for the binary being built. Any crate type that
would generate a PDB would have all applicable `.natvis` files embedded.

In the case of GDB pretty printer, `#![debugger-visualizer(gdb_script_file = "../foo.py")]`
the compiler will ensure that the set of pretty printers specified will be added to the
`.debug_gdb_scripts` section of the `ELF` generated. The `.debug_gdb_scripts` section
takes a list of null-terminated entries which specify scripts to load within GDB. The
Rust compiler currently embeds a visualizer for some types in the standard library via
the `.debug_gdb_scripts` section.

The `CrateRoot` type would need to be updated to account for debugger visualizer
files for crates within the dependency graph. The `CrateRoot` would contain
the field, `debugger_visualizers: Lazy<[DebuggerVisualizerFile]>`. The new type
`pub struct DebuggerVisualizerFile` will be created to ensure the contents of a
debugger visualizer file can be serialized and stored in the crate metadata.

There are a couple of reasons why the contents of a visualizer file passed into
rustc will be serialized and encoded in the crate metadata.

First, Cargo is not the only build system used with Rust. There are others
such as Bazel and Meson that support directly driving Rust. That might be
a minor issue to the wider community but for the people that are working
on those systems it is beneficial to pass this information through crate
metadata. That way, the information enters the dependency graph only at
the leaf nodes, and the code building the dependency graph doesn't need to
know how or why it flows through the dependency graph.

Secondly, there's also been interest within the community of supporting
binary crate packages. That is, compiling crates to rlibs, and then passing
around rlibs directly and not rebuilding the entire library. Having to
ensure that `.natvis` files are always passed along with rlibs as well
could become very difficult especially when other debugger visualizations
also become supported such as GDB's debugger scripts and WinDbg's JavaScript
debugger scripts. Packaging these sorts of things in the `rmeta` for an `rlib`
is simple, reliable and seems like the "right" thing to do here.

Another change that would need to be made here is to add a new field to the
`CrateInfo` type, `pub debugger_visualizers: FxHashSet<DebuggerVisualizerFile>`.
This will allow the Rust compiler to query the list of visualizer files being
added for a specific crate. The compiler will then be responsible for collecting
the set of visualizer files that were previously encoded and stored in the
`CrateMetadata` for a given crate dependency, will be decoded and have the
contents written to a new file in the `target` directory. In the case of Natvis,
the path of this new file will be what is passed to the `/NATVIS` linker flag.
For example, in a debug build, the contents of the `.natvis` files that were encoded
in the crate metadata will be written to new files in the directory `target/debug/deps/visualizers`.
Each visualizer file that is written will have a new name to ensure it is unique
across visualizer files for all crates with a naming scheme of `<crate_name>-<hash>.<visualizer_extension>`.
The `<visualizer_extension>` value will be `.natvis` in the case of a Natvis file
and `.py` in the case  of a pretty printer. The `<hash>` value will be the hash of the
contents of the visualizer file.

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

## Rationale

This design provides a simple mechanism to specify a debugger visualizer file
for a given crate and embed them in the resulting PDB or ELF depending on the
target. It does not need any manual intervention by a Rust developer who is
consuming such a crate to get the debugging experience to work when it is
viewed under a debugger that supports the visualizer specified.

This design does not break any existing usage of rustc. This new feature would
be strictly opt-in. The Natvis or GDB pretty printer syntax may not be familiar
to many Rust developers which may lead to a period of learning the syntax. Since
this feature would be optional, a consumer of a crate that has debugger visualizer
for types would not need to go through this learning curve.

## Alternatives

### Alternative 1: existing -C link-arg flag

Supporting this option would mean that changes to rustc are not necessary.
The changes would be limited to Cargo, which would be responsible for collecting
the set of Natvis files and passing `-Clink-arg=/NATVIS:{file-path}` for each
Natvis file.

The drawbacks for this option is that it will only collect Natvis files for the
top-most manifest. This will not walk the dependency graph and find all relevant
Natvis files so this will only work for targets that produce a `DLL` or `EXE` and
not an `.rlib`.

### Alternative 2: custom build script to set /NATVIS linker flag

Supporting this option would mean that changes to cargo and rustc are not necessary.
Each individual crate would be able to create a custom build script that would set
the rustc `link-arg` flag `cargo:rustc-link-arg=/NATVIS:{file-path}` for each Natvis
file.

The drawbacks for this option is that it would force all Rust developers to manually
create a build script and ensure it is kept up-to-date whenever the set of Natvis files
are updated. This option would also have the same drawback as above, using a build
script would be able to set the linker argument for adding Natvis but only for the top
level crate. Any dependencies or transitive dependencies would not be able to set that
linker argument in order to embed Natvis into the generated PDB. Also, for crates that
generate an `rlib`, this would also run into an issue since a PDB isn't generated for
an `rlib`.

### Alternative 3: inline Natvis XML fragments via attributes only

Supporting this option would mean that changes to cargo are not necessary.
This option could be implemented via an attribute and/or proc-macro which
would live outside of the compiler and could be ingested in via an outside crate.
Rustc would need some changes in order to collect all of the attribute usage from the
source code and create temporary files that could be passed to the MSVC linker via
the `/NATVIS` linker arg. For crate dependencies, the Natvis fragments can be combined
and embedded in the crate metadata so the Natvis can still be embedded in the final
PDB generated.

The drawbacks for this option is that it would add a lot of bloat to the Rust source
code directly if only the attribute syntax was supported. For types with many fields
or types that need extensive amounts of Natvis to appropriately visualize them in a
meaninngful way, this could distract from the contents of the code. Without being able
to pull some of the more intricate Natvis descriptions into a separate standalone
`.natvis` file, there may become an issue with the visibility of the source code.
Also, if/when other debugger visualization formats are supported, it could become
very obscure to read the source with large amounts of visualization scripts from
multiple schemas all being directly embedded in source code.

### Alternative 4: miri executes the MIR of a Debug impl within a debugger

Supporting this option would mean that changes to cargo and rustc are not necessary.
This would have the added benefit of taking full advantage of existing implementations
of the `Debug` trait. Many Rust developers already implement the `Debug` trait which is
used to format how types should be viewed, this would only ease the debugging quality of
Rust when viewed under any debugger. This option also has the added benefit of not
requiring any changes to a crate from a Rust developer by leveraging existing `Debug` impls.

The drawbacks for this option is that this has not been fully investigated to
determine its viability. This could be a great potential feature to ease
debugging Rust but without concrete data to push this towards a potential RFC,
I would assume supporting debugging in the systems that are already heavily used
by the Rust community to be a higher priority. If/when this option becomes a bit
more viable, there would be nothing stopping it from becoming a true feature.

### Alternative 5: #[link] attribute to implement this feature

```rust
#[cfg_attr(target_platform="msvc",link(file="foo.natvis", arg="/NATVIS"))]
struct Foo;
```

Supporting this option would mean that no new attributes would be needed for rustc.
This attribute currently exists today and implementing this feature on top of this
attribute would create an easy way to drop support for this feature in the future if
need be.

The drawbacks for this option is that it seems a sub-optimal in terms of user
experience. It requires the author to operate at a lower level of abstraction by
having to use a more general attribute and annotating it to tackle a specific use
case. Having a more targeted attribute, i.e. `#[debugger-visualizer]` allows for the
author to simply specify which debugger visualizer file should be included and allow
the compiler to select the right one under the covers.

## Impact

By not implementing the feature described by this RFC, the debugging quality of Rust,
especially on Windows, will be continue to be a difficult experience. The only
visualizations that exist today are for parts of the standard library. External crates
being consumed will not have debugging visualizations available and would make it
difficult to understand what is being debugged.

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
is automatically shown. The Rust compiler currently defines a pretty printer for a
limited set of types from within the standard library.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

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

# References

* Natvis
  + [Create custom views of C++ objects in the debugger using the Natvis framework](https://docs.microsoft.com/en-us/visualstudio/debugger/create-custom-views-of-native-objects)
  + [Visual Studio native debug visualization (natvis) for C++/WinRT](https://docs.microsoft.com/en-us/windows/uwp/cpp-and-winrt-apis/natvis)
  + https://docs.microsoft.com/en-us/windows-hardware/drivers/debugger/native-debugger-objects-in-natvis

* Pretty Printers
  + [Writing a Pretty Printer](https://sourceware.org/gdb/onlinedocs/gdb/Writing-a-Pretty_002dPrinter.html#Writing-a-Pretty_002dPrinter)
  + [The .debug_gdb_scripts section](https://sourceware.org/gdb/onlinedocs/gdb/dotdebug_005fgdb_005fscripts-section.html)