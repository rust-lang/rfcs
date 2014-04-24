- Start Date: 2014-04-23
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Import and export macros the same way as other symbols: `use` and `pub`.

# Motivation

Currently, macros use a global namespace. An example of a macro that could share the same name but have different uses: `try!`. `std::result::try!` could be what it is now, and `std::task::try!` could try the code in a task. This isn't currently possible. I believe the behavior is unspecified if you include 2 macros with the same name. Even if it were specified that it would fail, this is unsustainable as more and more libraries are written.

# Drawbacks

- This would be difficult, since module resolution happens after macros are expanded. This RFC makes macros discoverable via module resolution.

# Detailed design

One would export macros the same way you export any other symbol: prepending the expression with `pub`.

```rust
// result.rs
pub macro_rules! try (
    ($e:expr) => (match $e { Ok(v) => v, Err(e) => return Err(e) })
);
```

```rust
// task.rs
pub macro_rules! try {
    ($b:block) => (::std::task::try(proc() {
        $b
    });
};
```

One would import macros just like any other symbol: `use`.

```rust
// foo.rs
use std::task::{spawn, try!};
use maybe! = std::result::try!;
use std::result;

fn foo() -> Result<uint, ()> {
    maybe!( try! { // or, `result::try!`
        let dirs = readdir(Path::new("/home/sean")).ok().expect("i have directories");
        dirs.len()
    } )
}
```

Additionally, to resolve macros that need `libsyntax` and `macro_registrar` attributes, they could be written this way:

```rust
pub macro_registrar! regexp(cx: &mut ExtCtxt, sp: codemap::Span, tts: &[ast::TokenTree]) -> ~MacResult {

}
```

# Alternatives

Other designs include creating a second system for macro namespaces and resolution. However, this means 2 systems to do roughly similar things, and means that users have keep both resolution systems in their head when using Rust.

Not doing this means that eventually, macros will conflict. It's possible they already have, such as wanting `try!` in both result and task.

# Unresolved questions

- I'm not sure what would happen if a macro was used to create public symbols or other macros that clashed with previous macros in the module with the same name. Presumably, the compiler would emit the same error as when you try to export 2 things with same name.
Did I miss something?
