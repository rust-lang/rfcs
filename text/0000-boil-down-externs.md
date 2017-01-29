- Feature Name: boil_down_externs
- Start Date: 2017-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes to change the current `extern crate` syntax to allow
for multiple crates to be listed.

# Motivation
[motivation]: #motivation

This proposes will improve the ergonomics of the Rust language for
projects that have many dependencies.

# Detailed design
[design]: #detailed-design

Implementation would take the current `extern crate` but allow a list of
crates instead of a single crate. A single create would just be a single
list item.

Example:
```
extern crate {
   rocket, rocket_contrib, serde_json as json, chrono,
   dotenv, postgres, r2d2,  r2d2_diesel, tera as template,
   serde_derive, toml, glob
};
```

```
pub extern crate {rocket, rocket_contrib};
```

Meta items before the `extern` ie. `#[macro_use]` would be applied to
all crates listed.


Example:
```
#[macro_use]
extern crate {diesel, diesel_codegen, lazy_static, serde_derive};
```


Alternatively no braces


```
extern crate rocket, rocket_contrib, serde_json as json, chrono,
   dotenv, postgres, r2d2,  r2d2_diesel, tera as template,
   serde_derive, toml, glob;
```


Duplicate externs would error like it does currently.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

No new names or terminology needed to teach this. Examples should be
updated to include the list syntax.

Examples in both books, The Rust Programming Language and Rust by
Example, should be updated where more then one crate is used. A small
section should be added to http://rustbyexample.com/crates.html

rustfmt should have an opinion how to format the list of crates as well.

# Drawbacks
[drawbacks]: #drawbacks

1. Adds multiple ways to do things to a language.
2. Diffs can mask changes in lists.
3. Would be replaced buy future improvements to crate ecosystem
4. Differing opinions on `extern crate` and it's relationship to `mod`

# Alternatives
[alternatives]: #alternatives

A crate could be release with a macro. Like the example but one that supports meta values.

```
macro_rules! externs {
    ( $( $x:ident ),* ) => {
        $(
            extern crate $x;
        )*
    };
}
externs![rocket, rocket_contrib, serde_json]
```

Supporting meta values could be an issue with the macro.
Recursion could also be an issue. [Details discussed
here](https://botbot.me/mozilla/rust-internals/2017-01-29/?msg=80105364&page=1)

# Unresolved questions

