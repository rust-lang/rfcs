# Improve and Solidify Path Mental Model

## Summary
RFC 2126 sought to improve the ergonomics and learnability of rust's module system.
However, there were key points in that RFC that make rust's module system *more
difficult*, especially as it relates to the mental models around `self` and
`super` as they relate to the paths of files.

This RFC proposes the following:
- Removing language around using `foo.rs` + `/foo` instead of `foo/mod.rs`.
  Unforunately, the RFC never put a word for what this new system should be
  called. We will call the new system "file-dir modules" as they use a file (`foo.rs`)
  and a folder (`foo/`) of the same name. The traditional module system shall be
  called "`mod.rs` modules".
- Change `mod foo;` to requre (via lint) a fully qualified path:
  `mod self::foo;`
- Add a requirement that `cargo new` creates a `crate/` directory instead of
  `src/`. This will make `crate::foo::bar` mean `crate/foo/bar.rs` instead of
  `src/foo/bar.rs` for the "standard" new project.

## Motivation

### Preserve the mental model of `self::`
RFC 2126 posits that the primary benefit of moving to the file-dir model
(and not using `mod.rs`) is:

> From a learnability perspective, the fact that the paths in the module system
> aren't quite in direct correspondence with the file system is another small
> speedbump, and in particular makes `mod foo;` declarations entail extra
> ceremony (since the parent module must be moved into a new directory). A
> simpler rule would be: the path to a module's file is the path to it within
> Rust code, with .rs appended.

It goes on to say that the main benefit of `mod.rs` modules are as follows:
> The main benefit to `mod.rs` is that the code for a parent module and its
> children live more closely together (not necessarily desirable!) and that it
> provides a consistent story with `lib.rs`.

It *would* be nice for `use foo::bar;` to mean `use foo/bar.rs`.  However, the
costs are not worth that benefit, and the RFC does a poor job of accurately
weighing those costs/benefits.

First of all, the current system having a consistent story/mental model with
`lib.rs` is very good and shouldn't be downplayed. It makes the mental model
for how a crate is constructed identical for how a module is constructed which
makes teaching both of them much easier. A user must learn how to construct
a crate *before* they construct a module, which means they just need to apply
that knowledge to subdirectories to grow the complexity of their crate. It also
makes it easier to refactor a sub-module into a sub-crate.

However, the most imporant failing of RFC 2126 on this issue is that it fails
to take into account the consistency of the `self::` path. To demonstrate,
let's look at a project:
```
$ super-tree src/  # display files AND their contents
src
├── a
│   ├── b.rs
│   │   1 pub fn hello() {
│   │   2     println!("hello from src/a/b.rs");
│   │   3 }
│   │
│   └── mod.rs
│       1 mod b;
│       2 pub use self::b::hello;
│
├── c.rs
│   1 pub fn hello() {
│   2     println!("hello from src/c.rs")
│   3 }
│
└── lib.rs
    1 mod a;
    2 mod c;
    3 pub use a::hello as a_b_hello;
    4 pub use c::hello as c_hello;
```

If the `src/a/mod.rs` was moved to `src/a.rs` here is what the
user would have to think for the mental model of `pub use self::b::hello;`

Module System | File | Code | Mental Model
--------------+------+------+-------------
`mod.rs`      | `src/a/mod.rs` | `pub use self::b::hello` | `use ./b.rs::hello`
file-dir      | `src/a.rs` | `pub use self::b::hello` | `use ./a/b.rs::hello`

> Note that the file-folder mental model shifted from `./b.rs` to `./a/b.rs`.

In *every other context* there is a consistent mental model for `self::`, which
is that it is the same as the "current directory" (`./`) (we are ignoring
inline modules). This is true if you are in `foo/mod.rs` or `foo/bar.rs`. The
file-dir model creates a special case where you can no longer substitute
`self::foo` for `./foo.rs` in your head. You now have to know if you are in a
`foo.rs + foo/` situation, and if you are it translates to a path depending on
the name of the file you are in. This is going to be particularily confusing
when dealing with RFC 2126's third point:

> When refactoring code to introduce submodules, having to use `mod.rs` means you
> often have to move existing files around. Another papercut.

Adding the directory`foo/` will change the meaning of `self::`* in `foo.rs`*.
If you had moved `foo.rs` to `foo/mod.rs` this would make sense, of *course*
`self::` changes meaning -- you have moved the file! But under the file-dir
model it will change meaning just because you created a directory with the same
name! This will be exteremly confusing to even veterans. The solution is
to change `self:: -> super::` (or just use a full `crate::` path).

> Notice also, this applies equally to `super::`. Any `super::` lines in `foo.rs`
> will have to be changed to `super::super::`... even though you never moved
> `foo.rs` -- you only created a directory.

Furthermore, the second and third paper cuts are not really valid:
- The second one, having multiple `mod.rs` files open, is trivial to solve in
  even in the most basic editors. In vim, if you have `src/foo/mod.rs` open in
  a buffer it is easy to just type `:b foo/m<tab>` and vim will auto-complete
  the buffer name for you.
- The third one regarding refactoring is just plain wierd: if you are breaking
  a file into a module structure, you *should* be doing significant
  refactoring. Having to `mv foo.rs foo/mod.rs` is the *least* of the busy work
  you have to do. Furthermore, having `self::` and `super::` change because you
  created a directory with the same name as your file is a much larger paper
  cut.

As for the first paper cut... typing `vim src/foo<tab>` and finding that `foo`
is a folder and not a rust file is not that bad. Finding it is a folder means
that to access `crate::foo` means you have to add `/mod.rs`. The amount of
"cut" here is not very significiant, and certainly does not pose a risk of
hurting people's understanding.

#### Comment on inline modules
Inline modules are a fairly confusing subject for newbies, as they don't
exist in many other languages. However, `self::` is not confusing in
them, since the user can *see* that they are working in an inline module.
`self::` refering to the "outer scope" is fairly straightforward from
that perspective.

### Require full path in `mod` declarations
Requiring `mod` declarations to use the full path (i.e. `mod self::foo;`) will
accomplish the following:
- Make `use` and `mod` both be fully qualified paths to unify their mental model.
- We can teach `use` and `mod` in the same way without qualifiers that `mod` is
  automatically a relative path whereas `use` is a fully qualified path. They
  will both be identical.
- Provide better distinction between "inline modules"
  (i.e. `mod foo { /* module content here */ }`) and "file modules" by forcing
  "file modules" to specify a path. Paths are more often associated with files,
  so it is more clear what is going on.
- Typing `mod self::` repeatedly will help new users understand that `self`
  always coresponds to the file's directory (obviously this depends on removing
  the file-dir system from RFC 2126, which breaks that mental model)

However, it would have a major disadantage: I don't think we would ever
want to support larger paths (i.e. `mod crate::foo::bar::baz`) as it is
unclear how they would perform that lookup (normally mod statements are
used to know the sub-modules). So while `self::` is more explicit, the
user can not get any of the power that they might expect a full path would give
them.

### Aid in improving the mental model of `crate::`
When teaching users how to import modules under RFC 2126, the `crate::` keyword
will be excellent in mapping the file system to the module layout. However, one
exception will have to be taught: that `crate::` == `src/`. To make the mental
model complete, this RFC proposes that Cargo's default layout be:

```
project-name/
├── Cargo.toml
└── crate/
    └── lib.rs

Instead of:
```
project-name/
├── Cargo.toml
└── src/
    └── lib.rs


This will make `crate::foo::bar` **actually mean** `crate/foo/bar.rs`, which
will be easier to teach in the reference material.

# Guide-level explanation
This RFC is primarily focused on making the module system easier to teach
and learn.

One of the core reasons this RFC exists is to prevent RFC 2126 from corrupting
the mental model of `self::` refering to the relative path of the file, and
improving `use` declarations to actually refer to a specific path.

However, the following would be added.

## Initializing Crates
> ... Imagine start of guide about initialzing a small binary crate

After calling `cargo init myapp --bin` your crate will look like:

```
myapp/
├── crate/
│   └── main.rs
└── Cargo.toml
```

The `crate/` folder is where your source code goes, the starting
file being `main.rs`.

> ... Imagine the rest of the guide continuing

## Declaring Modules

> ... Imagine start of guide about creating a small binary crate

There will be a point when you want to break up your `crate/main.rs` file
into multiple sub-files. For instance, if you wanted to add `crate/foo.rs` as
a module you must add the following to your `main.rs`:

```
mod self::foo;
```

> ... Guide continues

## Creating sub-module directories
This will not change. One of the benfits of this RFC is that we will
prevent there being two ways to create submodules without a clear standard
(RFC 2126 did not suggest that the file-dir model be standardized) and
that we will prevent documentation churn.

# Implementation Details
Similar to the proposed `rustfix` solution in RFC 2126, this RFC proposes
that rustfix automatically converts `mod foo;` into `mod self::foo;` in all
code. This could probably be done by `rustfmt` as well since `mod foo` implies
`mod self::foo;`.

A lint will be against `mod foo;`, suggesting `mod self::foo;`. This will
be off-by-default at first to allow for rustfmt/fix to implement it.
The lint will also give helpful advice if the user tries a path other
than `self::` (other paths are not allowed).

In addition, Cargo's template should use `crate/` instead of `src/` once
the `crate::` path is stabilized.

# Drawbacks
The primary goal of this RFC is to *remove* the drawbacks of the file-dir
module system presented in RFC 2126. Therefore the primary drawback is that
we won't have that system.

Drawbacks from other features include:
- `mod self::foo;` is more boilerplaty than `mod foo;`
- `mod self::foo;` will require some code churn, albeit it is extremely easy
  to automate.
- `mod self::` should possibly be its own RFC
- `crate/` instead of `src/` is a change to a pretty well known convention. It
  may take some time for people to adapt to the new directory structure.

# Unresolved Questions
None at this time.
