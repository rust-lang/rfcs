- Feature Name: `macro_rules_visibility_v3`
- Start Date: 2021-01-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC proposes to change the current visibility scoping rules for `macro_rules!` macros to the same rules as all other items, namely private by default and exported through `pub` or `pub($PATH)`. The use of `#[macro_export]` and `#[macro_use]` become hard errors.

In addition to laying out the end goal, this RFC also proposes a transition plan as well as alternatives to that plan. 

# Motivation

[motivation]: #motivation

For historical reasons the visibility of `macro_rules!` macros (referred to from now on simply as "macros") have always followed different rules than those other items such as structs, enums, and functions. These items use a module path scoped system (referred to from now on as "path scoping") while macros use a system based on the order things appear in source files (referred to from now on as "textual scope"). 

There is no real advantage to treating the scope of macros differently from other language items, but there is plenty of disadvantage. First, users must learn two distinct systems for visibility scoping. Second, textual scoping is the only language feature that relies on the order of item declaration *in a module* to determine if something is visible or not making it the arguably the more "abnormal" of the two systems when considering how other language features work. 

Moving towards a systems where macros work just like any other item when it comes to visibility scoping will go a long way to make the language simpler and more stream-lined. 

# Transition plan

[transition-plan]: #transition-plan

## What can break?

To discuss transition plans, it's first important to understand what could potentially break in users' code should macros be made to respect the same privacy scoping as other items. 

### Usage in submodules

Macros defined in parent modules are visible in child modules and can be used without the need for qualifying the macro invocation with the path to the parent module where the macro is defined.

Path scoping also makes items defined in parent modules visible to child modules, but the usage of items *requires* accessing items through the path to the parent module where the item is defined.

For example:
```rust
mod m1 {
    macro_rules! my_macro { () => {} }
    struct MyStruct;
    mod m2 {
        // Macro is useable without referencing parent module
        my_macro!(); 
        // Other items like structs must be referenced by path
        fn function() { super::MyStruct; } 
    }
}
```

Fixing this requires annotating macro invocations with the path to the parent module where the macro is defined or adding a `use` statement (though this may lead to name clashing).

### Shadowing 

Shadowing allows for the different macros with the same name to be used in the same scope with the last one defined before a given usage (in top-to-bottom textual order) being the definition used.

This works differently from path scoping where definition of two items with the same name is not allowed.

Fixing this issue requires disambiguating the macros by giving them different names.

One use case of shadowing that is not possible to emulate in a path based system where names are not allowed to collide is "anonymous" helper macros. This use case defines helper macros inside the main macro. Each time the main macro is invoked, the helper macro is redefined, shadowing its previous definition. This relies on shadowing to ensure the new helper definition is used instead of any older ones. For deeper explanation of this pattern, [see this playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=3abfacbdd030fa2bc49d56227127b6fd). 

It is unclear how widespread such a use case is.

### `#[macro_export]`

The `#[macro_export]` annotation complicates the matter by making all annotated macros available at the top level of a crate. 

Fixing this use requires marking the macro as `pub` and publicly re-exporting the macro at the top level crate module. 

### `#[macro_use]`

The `#[macro_use]` annotation has two meanings. 

First, when applied to a module it makes all all the macros in that module useable below where that module is defined. Fixing this use requires marking all the macros in that module as `pub` and importing them into the parent module.

Second, when applied to an `extern crate` item, any macro annotated with `#[macro_export]` becomes useable *anywhere* inside the consuming crate. Fixing this would require changing all uses of those macros to use the fully qualified path to that macro. 

## Summary of Fixes

To migrate to a path scoping visibility system the following would need to be updated to current uses of macros:
* Inside a module detect if there are multiple macros with the name 
    * rename these macros and their invocations in textual order.
    * if one of the macros is annotated `#[macro_export]` ensure it keeps its original name 
        * *note*: trying to export two macros with the same name is already an error
* Detect any macro invocations where the macro is defined in a parent macro.
    * Annotate the invocation with the path to the parent module where the macro is defined. 
* Remove `#[macro_export]` annotations from macros
    * mark macros as `pub`
    * publicly re-export them from the top-level module of the crate where the macro is defined
* Remove `#[macro_use]` where annotating a module
    * mark all the macros in the previously annotated module as `pub(crate)`
    * mark the module itself as pub(crate) if not already `pub(crate)` or `pub`
    * annotate the macro invocation with a path to the module where the macro is defined.
* Remove `#[macro_use]` from from `extern crate` item
    * change all uses of macros to qualified `$EXTERNAL_CRATE::$MACRO_NAME` invocations. 

These steps should be automatable so that rustfix can be used to aid in migration though there are sufficiently complex use cases that a fully automatable transition is likely to not be possible. For example, the "anonymous" helper macros use case discussed in the section on shadowing would not be able to fixed in an automated way.  

Roughly what percentage of use cases will be machine migratable is an open question. 

## Translating Common Patterns 

The following are how common patterns in macros today translate to the next path based scoping system.

### Deeply nested macros

Macro use makes all macros inside a child module available to the parent module.

```rust 

#[macro_use]
mod m {
    #[macro_use]
    mod n {
        macro_rules! define_foo {  () => { fn foo() {} } }
    }
}
    
    
define_foo!();
```

This would be translated as: 

```rust 
pub(crate) mod m {
    pub(crate) mod n {
        pub(crate) macro_rules! define_foo {  () => { fn foo() {} } }
    }
}
    
    
m::n::define_foo!();
```

### Recursive macros

Recursive macros are macros that call themselves (perhaps with different arguments)

```rust 
#[macro_use]
mod m {
    macro_rules! print_expr {
        ($e:expr) => {{
            println!("Going to do {}", stringify!($e));
            print_expr!(no_print => $e)
        }};
        (no_print => $e:expr) => {{
            $e
        }};
    }
}

fn main() {
    print_expr!(1 + 1)
}
```

Naively changing this to path based scope would not work as it is not guaranteed that the unqualified `print_expr` name is in scope. In the example above, `print_expr!` is used recursively inside the macro, but in a path scoped system the recursive call would not be in scope if the macro was called with a qualified path (e.g., user calls `m::print_expr!` which references unqualified `print_expr!` which is not in scope).

A possible way to handle this is to introduce a new macro specific keyword `$self` which is directly analogous to `$crate` except that it refers to the module where the macro is defined. This would work in the simple case but quickly breaks down in more complicated module paths. For example:

```rust 
pub mod m {
    mod n { // n is private
        macro_rules! my_macro {
            () => {
                $self::my_macro!(@)
            };
            (@) => {};
        }
    }
    pub use n::my_macro;
}
m::my_macro!()
```

If `$self` refers to the module where `my_macro!` is defined (i.e., `m::n`) then the call in the top level module would expand to `m::n::my_macro!(@)` and an error would occur due to `n` being private.

`$self` could refer to the current namespace of the top-level macro call, but we are unsure at this time if this would lead to ambiguities.

### "Private" macros

Macros can use "private" macros (i.e., macros defined inside of other macros). This can lead to an issue where a macro is defined twice which would lead to name clashing.

```rust 
macro_rules! private {
    () => {
        private!(@);
        private!(@);
    };
    (@) => {
        macro_rules! __private {
            () => {};
        }
    }
}

private!()
```

Here `__private` is defined twice which leads to an error.

How this should be overcome is not yet known. Possible ideas are:
* allowing macros defined inside of other macros to shadow one another. This may lead to ambiguities.
* Disallow this use case

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Once this feature is implemented there will be no need for additional explanation as the visibility system must already be explained for other items. All that needs to be changed is to add `macro_rules!` macros to the list of items that respect these rules.

It may be necessary to keep reference to the old system for people to read about should they encounter it in old code. 

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

Implementation of the final vision is very straightforward as the there is no longer a need for a separate scope visibility system. `macro_rules` macros will behave exactly like all other items.

# Drawbacks

[drawbacks]: #drawbacks

As noted, there are almost no benefits to macros using textual based scoping over path based. 

However, the use of `#[macro_use]` on `extern crate` items in particular requires less typing than the path based alternative. Path based scoping requires explicitly naming the path to the macro at least once in each module where the macro is used while `#[macro_use]` means macros can be used without their qualified path everywhere in the importing crate. We believe this slight hit to ergonomics is worth the price of consistency. 

### Transition plan drawbacks

[transition-drawbacks]: #transition-drawbacks

Fully transitioning in one edition poses several drawbacks:
* This arguably goes directly against the edition system as laid out in [RFC 2052](https://github.com/rust-lang/rfcs/blob/master/text/2052-epochs.md):
    * > Thus, code that compiles without warnings on the previous edition (under the latest compiler release) will compile without errors on the next edition (modulo the usual caveats about type inference changes and so on).
    * This needs to be weighed against the disadvantages of the multi-step transition plan discussed in the alternatives section.
* Many have expressed informal desire for a less "exciting" edition than Rust 2018. While transitioning in one edition may avoid a less than ideal temporary state before the full transition is achieved, it is also less of a big change all at once. 

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

No alternatives are currently being explored for the end vision of this RFC. The alternatives mentioned here are alternatives to the transition which all will ultimately end in fully path scoped macros.

## Deprecating `#[macro_export]` and `#[macro_rules]` in Rust 2021

An alternative to the full transition to path scoping is to deprecate the use of `#[macro_export]` and `#[macro_use]` in favor of path scoping. 

In particular this means the following:
* By default, emit a warning when using `#[macro_export]` or `#[macro_use]` suggesting the user to use `pub` annotations and access macros by path instead.
* Any macro marked with `pub` stops following textual scoping rules and can be accessed by path like any other item. 
    * One consequence of this means annotating a module with `#[macro_use]` will not have any impact on the visibility of macros marked as `pub` inside that module.
    * We can potentially warn against the mixed use of `pub` and `#[macro_use]`
* Any macro not marked with `pub` continues to follow textual scoping rules
    * This includes shadowing and not being usable before being defined
* Marking a macro as `pub` and annotating with `#[macro_export]` is a hard error.

### Advantages 

This has the advantage not forcing users to upgrade their code at the point of moving to a new edition. They will receive warnings but their code continues to compile and can be gradually transitioned. 

Thus this avoids the [drawbacks](#transition-drawbacks) of the proposed full transition above.

### Disadvantages

There are several downsides to this proposal:
* The existence of two scoping systems at once can be confusing especially if users mix the two usages. 
    * Currently, it is possible to "convert" a macro to use path based scoping [by reexporting a macro from a module](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=275c52e038b5ab3db526a605874fbd17), but this use case is rare. The proposal above would likely see much deeper mixing of the two systems. 
* Even if the user never uses the deprecated annotations, macros defined and used locally to a module still follow textual scoping rules unlike every other item in the language. 

## Opting into textual scoping

Another possibility is to allow users to opt-into textual scoping through some annotation such as `#[enable_textual_scoping]`. This would allow users of sufficiently advanced uses cases to retain the old semantics should they choose. This would also allow `rustfix` to fall back to annotating any macro definitions which cannot be moved to the path based scoping scheme in an automated way. 

# Prior art

[prior-art]: #prior-art

This is a natural progressing of the work that started with Rust 2018 where the use of `#[macro_use]` on `extern crate` was no longer required and users could use macros by path.

Additionally, implementation work for this RFC has already begun in [#78166](https://github.com/rust-lang/rust/pull/78166) behind a feature flag.

Most importantly, the prior art is that of how all other items work in Rust. After all, the aim of this RFC is for consistency.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

The open questions have been explored through out this document. In particular, the most important question is which transition plan is best. Determining this likely hinges on how easily crates be transitioned to the new rules. While it is unlikely that `rustfix` could be made to cover all uses, it is possible that it can cover a sufficient amount to make transition in Rust 2021 acceptable.

We have proposed fully transitioning in Rust 2021, but if that is deemed unacceptable an alternative must be considered. 

# Future possibilities

[future-possibilities]: #future-possibilities

No future possibilities are currently being considered.