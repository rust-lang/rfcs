- Feature Name: cargo_project_based_examples
- Start Date: 2018-08-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)
# Summary
[summary]: #summary

This RFC enables cargo to run, test and bench examples which are arranged and stored as a project-based cargo project
in `examples` folder, in addition to existing single `examples/*.rs` and `examples/**/main.rs` file-based examples. 
A project-based example imply a complete cargo project with `Cargo.toml`, `src` folder, `main.rs` and maybe 
other necessary files in it. 

# Motivation
[motivation]: #motivation

Recently many projects, especially huge library projects, are used in a bunch of conditions, and to prove 
the universal use of the project, a wide variety of examples is needed. 
But differences are there between examples and the project itself because an example:

1. Might need more external crates than the project itself;
2. Could be compiled into various cargo targets.

For example, when developing a backend for code editors like Xray, we might need to implement its frontend 
in a diversity of forms like terminal for vim-like experience, Qt for graphic UI, or even wasm for online 
judge websites. These forms need to import different crates and are built to different targets. 
If we only use existing file-based examples, we were not even able to import external crates without editing 
the `Cargo.toml` for the project itself, which might lead to compiling unnecessary libraries to build this project. 

Likewise, other projects, especially developed for embedded platforms, also needs separate dependency 
for writing examples. By now developers have to rely on one of the following two approaches to do this. 
In the first approach developers have to use the `dev-dependencies` and write dependencies together for all 
examples into the root `Cargo.toml`, like what projects like [f3] did, and we have to compile all of them 
even if trying to build only one example. 
In the second approach developers might add all examples one by one into `[workspace]` as workspace members, 
which requires `cargo run -p` to run it and is somehow complex. 
If project-based examples could be formed for these projects, developers would be able to run the examples 
in a more graceful way as well as save compile time.

[f3]: https://github.com/japaric/f3/blob/307525cb8d541adb7375d6134d18cd1cdf5c814b/Cargo.toml#L17-L23

In addition, many library projects like [yew], [stdweb] and [wasm-bindgen], now already somehow made an attempt 
to place folders into `examples` folder to form an array of example 'projects'. 

[yew]: https://github.com/DenisKolodin/yew/tree/dcd3834dd915647f4eae1ec78b6d803b70fad1da/examples
[stdweb]: https://github.com/koute/stdweb/tree/7c2d096cd6d47d6e68b43ff1f83341885a6f6585/examples
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen/tree/b6a6dee7f102c2026a8f468dc2e1b4f75a17cf31/examples

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Generalized abstract

Before we start, we should keep in mind that firstly this RFC enhances the search range for parameter `--example` 
in subcommand `run`, `test` and `bench`, and secondly this RFC adds `--example` for subcommand `new`. 

This means, for example, you can create a example project `abc` using `cargo new --example abc`, 
and you may run it with `cargo run --example abc` as it now searches for all `examples/abc.rs` file, 
`examples/abc/main.rs` and `examples/abc/` project folder. 

In addition, an example project may include its own tests and benches. 
By using `cargo test --examples` etc., tests or benches for example projects could be executed all at one time. 
But to make things clear, it is possible but not suggested to create examples for examples. 

## When to use project-based examples

When you are developing a library project, trying to build examples in single `examples/*.rs` files or 
`examples/**/main.rs` that could be *complex enough* to:

1. import external crates that the library project itself won't use
2. be compiled into entirely different framework than the library project itself

you might need to change an approach because you are now able to build project-based examples. 

## Create a project-based example

Assume that you have a library project `my_project` and you want to build examples for it. So here we go.

Firstly, use `cd` command to direct your terminal to project root. Now the terminal might shows like this 
before your cursor:

```
XXX: my_project username$ 
```

Secondly, Create a example project using: 

```
cargo new --example abc
``` 

where `abc` is the name of your example. 
This command creates a folder for your project-based example `abc`, which is a complete templated cargo project, 
in `examples` folder. So your `my_project` have a structure like: 

```
.
├── Cargo.lock
├── Cargo.toml
├── src
│   └── lib.rs 
└── examples 
    └── abc # created by the command
        ├── Cargo.lock 
        ├── Cargo.toml # import your crates here
        └── src
            └── main.rs # write your example code here
```

By creating an example project, you are now ready to write your code in `main.rs` and `Cargo.toml`. 
Feel free to code here with your editor. 

Note that a `.gitignore` file is added to `examples/abc/.gitignore` as gitignore is supported in nested folders.  
A simple line `!Cargo.lock` will be written in it so that the gitignore rule for `Cargo.lock` is not to be 
overridden by the `.gitignore` file in the root. As the `.gitignore` in subdirectories effects the directory it is 
in, the `.gitignore` in root project is not to be effected. 

As for `Cargo.toml`, for your convenience, cargo automatically generate a dependency on your root project like:

```toml
[dependencies]
my_project = { path = "../.." }
```

Thirdly, after your code work, you need to run your example. Instead of using `cd examples/abc` and `cargo run`, 
which would also works but somehow complicated, you could run your example as simple as: 

```
cargo run --example abc
```

Then you can happily witness you example code being run in console, just as what you did before to `*.rs` example files. 

Additonally, you may include tests and benches into your example project. You can create `tests` or `benches` in 
`examples/abc/` folder, and run it by `cargo test --example abc` etc. 
Note that it's not suggested to create examples for examples.

## When conflicting with `*.rs` or `**/main.rs` files 

Your `examples` folder may include more than one of `*.rs` files, `**/main.rs` files and/or project-based 
examples as folders, thus might have conflicting names when for example, you have at least two of 
`examples/abc.rs`, `examples/abc/main.rs` and project `examples/abc/`. 

Yet cargo already have supported `*.rs` and `**/main.rs`. If there are conflicts between them, 
or another word you have both of them in your `examples` folder, you will get this message 
when trying to run this example:

```
error: failed to parse manifest at `<Project Root>/Cargo.toml`

Caused by:
  found duplicate example name <NAME>, but all example targets must have a unique name
``` 

After the third approach implemented as `examples/abc/` project, if there are still conflicts, the above message
is given as well. And what you need to do to resolve this conflict is to rename the folders or the files to 
make the names unique.

## Notes for path-related macros

As project-based examples might affect path-related macros, there are two macros we should pay attention to: 
`module_path!()` and `file!()`. As project-based examples should be treated like a unique project, the 
outputs of these two macros should refer to the path of the example instead of root project path. 
For example, if we have a `abc` example project for root project `my_project` and the `main` function in 
`examples/abc/src/main.rs` contains `println!("{} {}", module_path!(), file!())`, 
it should print `abc src/main.rs` rather than `abc examples/abc/src/main.rs`.
 
There are plenty of macros like `line!()` and `column!()` whose value after being parsed are related to
the line number and column number. Fortunately, the parsing logic of these macros above are not effected 
if being written into project-based example codes. 

## Conclusion

By including `cargo new --example` and enhancing `cargo run --example` etc., this RFC provides a more 
convenient way for you to write examples for your project.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Enhanced `--example` for run, test and bench

To implement this feature, firstly we enlarges `--example` search scope. We've already got `examples/*.rs` 
and `examples/**/main.rs` searched by cargo every time it tries to run, test or bench an example. 
But to implement `examples/**/` project we should detect if this folder contains a valid cargo project. 
This could be done by detecting `Cargo.toml` the same way we detect when trying to run `cargo run` in an 
invalid folder for cargo projects. As for `--examples` on `test` and `bench`, the search scope is also 
enlarged to test or bench all examples at one time, including all project-based example projects.

When operating example project, we compile the whole example project as what we do on the root project. 
We could share the `target` folder with it then incremental compiling can be enabled to save time. 
The target of example projects are stored just like what we did for file-based examples.

Running, testing and benching project-based examples should be treated the same as what Rust already do 
for single-file examples. Same toolchain should be applied to them by default. 

Operating example projects with `--example <NAME>` is just like a syntax sugar in programming, 
which help us save time when executing examples.  

For path-related macros, refer to the paragraph 'Notes for path-related macros'.

## `cargo new --example <NAME>`

Another feature `cargo new --example <NAME>` requires creating a new argument. 
If argument `--example <NAME>` is found, we search for if this folder exists, if so we fire an error like 
`error: example <NAME> already exists`; and if not, a folder is created in `examples` and a cargo project 
for runnables where there is `main.rs` is built inside.

Note that `cargo new --example <NAME>` can only be executed inside a cargo project. 
Execution with no cargo project detected should be denied with an error message like 
`error: no cargo project found to create an example` or similiar messages.

Additionally, what `cargo new --example <NAME>` differs from `cargo new` is only that the former command, 
as is mentioned above, generates a dependency on the root project using `../..` references. 
Despite this, the procedure should be the same, including what we should write into the `.gitignore` file 
for it, so it should be possible to `cd` into it and run `cargo run` directly.

# Drawbacks
[drawbacks]: #drawbacks

If we implement project-based examples into cargo, it might be backward incompatible if we want to use 
`--example <NAME>` in other ways in the future.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Not implementing this feature

It may also be suggested that we could somehow *not* implement this project-based cargo examples, 
but suggest the developers to build an example by adding all of them as the workspace members, and use the 
`cargo run -p <NAME>` command to run, test or bench it as it indicates the path of the root project, 
like what we already have in projects like [diesel] and [quicli]. 
By this way we share the `target` folder with the root project to improve compile speed as well as 
type less `cd` commands, but we have to add all our examples one by one as subpackages into the workspace.

It could also be possible if we auto-import every project-based subdirectories in `/examples` into 
the workspace. By this way we don't have to manually import every example folder path into `[workspace]`,
but there could be further detailed designs to do on customization and simplization. 

[diesel]: https://github.com/diesel-rs/diesel/blob/b8d8620b1e6e9f0c0830d16e8762e215930b8a5c/Cargo.toml#L1-L26
[quicli]: https://github.com/killercup/quicli/blob/879dd74a2a0e3c47b2e76f41694920042317a0c9/Cargo.toml#L38-L44

## By adding another subcommand 

There could be another way to implement project-based examples by introducing `cargo example` subcommand. 
However by doing this we must change our way to run examples now by `cargo run --example <NAME>` which 
is already widely accepted by rust community. 

## By enhancing `[[examples]]` in `Cargo.toml`

It could be also possible to enhance the tag `[[examples]]` by adding a key named `cargo-toml-path` or others, 
to make it possible to locate own `Cargo.toml` file for this example, rather than using `dev-dependencies` only.
By this way we use unique `Cargo.toml` for different examples without changing the way to run them.

## Another way to write `.gitignore`

On `.gitignore`, it could be a good idea to rewrite the `.gitignore` file in the root changing 
the `Cargo.lock` to `/Cargo.lock` to avoid it search for every cargo locks nestedly 
thus a `!Cargo.lock` is not needed in the example project path. 
However it would totally change the way how we write `.gitignore` for Rust, thus this alternative 
is remained for the Rust authors to judge.

# Prior art
[prior-art]: #prior-art

None by now.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is there any more graceful way to replace the `my_project = { path = "../.." }` in `Cargo.toml` file 
for all example projects?
- Should path-related macro `file!()` refer the path related to the root project path?
