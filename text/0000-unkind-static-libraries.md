- Feature Name: more_link_kinds
- Start Date: 2016-02-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary


Adds a new `kind=object` (alternatively `kind=obj`) that is used to link object files.

# Motivation
[motivation]: #motivation

Rust currently does not let you link to an object file directly. You may argue "Oh, can't you just shove the object file into a static library and link to that?". Sometimes the answer to that question is no, you can not. For example, when linking to resource files on Windows you can compile a `.rc` file into a `.res` and link to that, or further into a `.obj` and link to that. However if you create a `.lib` from the `.obj` then attempting to link to it will fail. As a result there needs to be a way to link to the object file directly.

Another use case for object files is that they have higher priority than libraries. By linking an object file you can override symbols from other external libraries without having to worry about order. At least, this is the case for MSVC, I haven't tested other linkers.

# Detailed design
[design]: #detailed-design

`kind=object` can be applied the same way as any of the other `kind`s, whether via flags passed to cargo via build scripts, flags passed to rustc via the command line, or `#[link]` attributes. Object files will be passed to the linker the same way that `kind=static-nobundle` passes libraries to the linker, which is to pass it to the first immediate linker invocation and **not** bundle it into the rlib.

## dllimport and dllexport

Symbols from an object file are assumed to be static symbols, so `dllimport` will *not* be applied. The behavior should match `kind=static-nobundle`.

# Drawbacks
[drawbacks]: #drawbacks

* It adds two more `kind`s that have to be supported and tested.

# Alternatives
[alternatives]: #alternatives

* A current workaround exists where the object file is renamed to have a `.lib` extension and then passed via `kind=dylib`. The linker is smart enough to not trust the extension and so links successfully (tested with both MinGW and MSVC).
* Add support directly for resource files where rustc invokes the necessary tools to compile the resource file and links it itself. Doesn't cover the use case of linking to object files to override symbols from libraries though.

# Unresolved questions
[unresolved]: #unresolved-questions

* The name of the `kind`. Please bikeshed vigorously.
