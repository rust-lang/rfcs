- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

The `cfg` system should be extended to include conditional compilation of items based on the version
of the compiler, similar to the one found in the [RubySpec project].

# Motivation

Implementing this feature makes maintenance of Rust codebases that must support multiple compiler versions 
easier, especially in the case of significant syntactical or API changes. Currently there is no easy
way to target multiple compiler versions in one file, and the `cfg` system in its current form is 
not adequate to solve the problem, as it only strips items after they have been parsed, so it will not
work in the case of language syntax changes.

Additionally, this feature has been nominated for a P-backcompat-lang milestone (Issue #3795 in the Rust repository)

# Drawbacks

Adding this feature requires a method of excluding language items before they are parsed.
This would require an overhaul of the current `cfg` system, or adding a new system to do so,
as well as potentially requiring new syntax to mark the end of the item that the attribute refers
to, as the code would not have been parsed into crates/modules/items yet.

# Detailed design

This RFC would add an extension to the `cfg` system to allow conditional compilation of language items
based off the language version. To accomplish this, a new rust_version condition would be added, with
the following syntax:

```
#[cfg(rust_version="version")]
```

Where version is an optional comparison operator (<, >, <=, >=) followed by a version string in
[semantic versioning] form. The rust version is then compared to the version string
using the provided operator (using equality if no operator is given).  
Examples: 

```rust
//Compile foo if the Rust version is greater than v. 1.0.0
#[cfg(rust_version=">1.0.0")]
fn foo() { ...}

//Compile foo if the Rust version is 0.5.0-pre
#[cfg(rust_version="0.5.0-pre")]
fn foo() { ... }
```

If an invalid operator is given or the version is not in semver form, the item will be excluded,
same as the current behavior for other `cfg` attributes. To implement this behavior, a special case would
need to be added to the `test_cfg` function in attr.rs similar to the current special case for `#[cfg(not(...))]`.
This is because the current `contains` function the matcher uses only string equality of the MetaItems, which will
not work due to the need to parse the versions and do comparisons. This is not ideal, but is the only way unless the `cfg`
matcher is changed to allow configurations to define their own matching function. Rust's libsemver will be used to do the
parsing and comparison of the two version numbers.

The other issue that needs to be addressed is making the system able to handle syntactical changes to the language. Currently,
conditional compilation works by removing items from the AST after it is constructed. If older syntax that is no longer valid is
included within a file compilation will fail before it is able to be excluded. In order for version based compilation to work,
language items need to be excluded before syntax checking. To do so, items that are marked to be conditionally compiled would be parsed
into a placeholder that is later expanded, much like macros are now. To do this, a new attribute would be added to indicate the start and end of
a block, with the restriction that the contents must expand into a single item. For example:

```rust
#[cfg(rust_version=">1.0.0")]
#[begin_item]
fn foo() { ... }
#[end_item]
```

By using special syntax to mark the beginning and end of the item no syntactical requirements are imposed on the contents within the block,
which means that this system will work even if significant changes to the language are made (function declaration or braces for example).
The placeholder items would be expanded after the first stripping pass but before macro expansion, which leaves the rest of the compilation process
unaffected.

# Alternatives

* Doing nothing  

Version-based conditional compilation is simply a convenience for targeting multiple compiler versions.
Not implementing it would make doing so more difficult, but not possible. Other methods, such keeping 
multiple versions of a file and using the Makefile to select the correct at compile time would accomplish
the same thing. However, that approach takes more work and requires code duplication, making the code harder
to maintain.

* Use the proposal, but without new syntax

Instead of adding new syntax to handle expansion, other alternatives could be used to determine what goes into the placeholder,
such as following matching braces. However, this would require committing to certain syntactical constructs, which is counterintuitive
to the purpose of the RFC.

# Unresolved questions

* What would be the best syntax for delineating placeholder items?
* How exactly does the parser need to be changed for this to work?

[semantic versioning]: http://semver.org
[RubySpec project]: http://rubyspec.org/guards/
