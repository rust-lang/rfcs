- Feature `static_mut_2024`
- Start Date: 2024-01-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Deprecate usage of `static mut` for the 2024 edition of Rust, directing users to switch to interior mutability with subsequent removal of the syntax entirely in the 2027 edition. (This is not pertinent to `&'static mut`)

# Motivation
[motivation]: #motivation

The existing `static mut` feature is difficult to use correctly (it's trivial to obtain aliasing exclusive references or encounter UB due to unsynchronised accesses to variables declared with `static mut`) and is becoming redundant due to the expansion of the interior mutability ecosystem which easily replaces `static mut`'s functionality.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`static mut` is meant to provide statics that can be modified after their initial value is set; variables declared with `static mut` can prove quite problematic when used, however:
```rust
static mut X: i32 = 0;

fn main() {
  let a = unsafe { &mut X };
  let b = unsafe { &mut X };

  println!("{a} {b}");
}
```
Recall Rust's borrowing rules: 
- At any given time, you can have either one mutable reference or any number of immutable references to a place.
- References must always be valid.

The first rule is violated as we have 2 exclusive (mutable) references to the same datum at the same time and are actively using them in an entirely overlapping fashion. This violation means that our code's behaviour is undefined, and the optimiser is free to do with it as it wishes, potentially breaking it. The code is not guaranteed to print "0 0" and may fail to do so under some circumstances.

`static mut` also allows for unsynchronised accesses across multiple threads which
can cause data races which are also undefined behaviour.
```rust
use std::thread::spawn;

static mut X: usize = 0;

const N: usize = 16;

fn main() {
    let mut thread_pool = Vec::with_capacity(N);
    
    for i in 0..N {
        thread_pool.push(spawn(move || {
            unsafe {
                X = i;
            }
            println!("{}", unsafe { FOO });
        }));
    }

    for thread in thread_pool {
        thread.join().unwrap();
    }
}
```

Here, since the usize is not an atomic (with predictable and defined relative ordering) nor synchronised with a `Mutex` or `RwLock` a data race takes place, printing numbers between 0 and 16 in a vaguely increasing fashion. This is undefined behaviour and means that our code is not correct. This and the previous example show UB that is almost trivial to cause which makes it prone to occur by accident in a large codebase.

Let's try to use `static mut` for FFI purposes (a common application of it), this is usually achieved in this fashion:

```rust
// using a symbol exported by C code
extern "C" { static mut _c_symbol: Ty; }

// exporting a symbol from rust code for use by C code
#[no_mangle]
pub static mut _rust_symbol: Ty = val;
```
This puts our code at risk of causing UB on access as we saw before. Accesses to `static mut` can become difficult to track and reason about very quickly as the size of the codebase increases. As such, by the 2024 edition, we get a deprecation warning (or even deny-by-default lint):
```rust
// WARNING: `static mut` syntax is deprecated as of edition 2024 and is slated
// for removal in edition 2027. Consider using std::cell::SyncUnsafeCell<T> instead. 
// Read more at (somewhere, maybe rust blog post).
// Note/fix: 
// - extern "C" { static mut _c_symbol: Ty; }
// + extern "C" { static _c_symbol: std::cell::SyncUnsafeCell<Ty>; }
extern "C" { static mut _c_symbol: Ty; }

// WARNING: `static mut` syntax is deprecated as of edition 2024, and is slated
// for removal in edition 2027. Consider using std::cell::SyncUnsafeCell<T> instead. Read 
// more at (somewhere, maybe rust blog post).
// Note/fix: 
// - pub static mut _rust_symbol: Ty = val;
// + pub static _rust_symbol: std::cell::SyncUnsafeCell<Ty> = std::cell:SyncUnsafeCell::new(val);
#[no_mangle]
pub static mut _rust_symbol: Ty = val;
```
If we try to do the same thing in the 2027 edition, we get a hard syntax error for not migrating:
```rust
// ERROR: expected one of `:`, `;`, or `=`, found `mut`
// ERROR: error: missing type for `static` item
// Note:
// `static mut` syntax has been removed as of edition 2027: for equivalent behaviour, use std::cell::SyncUnsafeCell<T> instead.
// Fix:
// - extern "C" { static mut _c_symbol: Ty; }
// + extern "C" { static _c_symbol: std::cell::SyncUnsafeCell<Ty>; }
extern "C" { static mut _c_symbol: Ty; }

// ERROR: expected one of `:`, `;`, or `=`, found `mut`
// ERROR: error: missing type for `static` item
// Note:
// `static mut` syntax has been removed as of edition 2027: for equivalent behaviour, use std::cell::SyncUnsafeCell<T> instead.
// Fix:
// - pub static mut _rust_symbol: Ty = val;
// + pub static _rust_symbol: std::cell::SyncUnsafeCell<Ty> = std::cell:SyncUnsafeCell::new(val);
#[no_mangle]
pub static mut _rust_symbol: Ty = val;
```
Migration from `static mut` in favor of `SyncUnsafeCell` makes code easier to audit, as some operations previously unsafe to perform on `static mut` (such as obtaining a raw pointer to the static) become safe, shifting focus fully to the areas where problems might arise (where the raw pointers are dereferenced) as it is at those points where we create references from raw pointers or use the raw pointers to access the underlying data. Keep in mind, however, that while `SyncUnsafeCell `is a less obvious type/technique to find (harder for beginners to fall into using) and a more verbose one to use, it is still highly unsafe and still does allow someone determined to create aliasing exclusive references to a place; caution should be taken by users of `SyncUnsafeCell` and `UnsafeCell` in general. 

If we follow the diagnostics given by the compiler, we can migrate our code to a safer version of itself and make it easier to audit for any mistakes by better isolating where they can occur. The use of intermediate raw pointers to obtain references also produces marginally better output from the [Miri tool](https://github.com/rust-lang/miri) which allows for better automated detection of problems in the code.

Things to (please) note:
- Interior mutability types ***are*** present in `core`. This change does not irreversibly break `no_std` code; `std` reexports the types from `core`.
- `std::cell::SyncUnsafeCell<T>` ***is*** the standard type that behaves most equivalently to `static mut`, but it is not necessarily the type you want:
	- `std::cell::SyncUnsafeCell<T>` ***is***  `repr(transparent)`, which means that it has the same layout as the wrapped type within, this is the main reason the type exists.
	- `std::cell::SyncUnsafeCell<T>` ***does not*** have any more overhead than `std::cell::UnsafeCell<T>`, which is nominally zero. Тhis also means that it has no greater overhead than `static mut` by extension.
	- `std::cell::SyncUnsafeCell<T>` ***is*** bound on `T: Sync`, but creating a custom type can alleviate any problems with needing to put `!Sync` types in a `static`. One can make it into `std` (through `core`) at some point.
	- `std::cell::SyncUnsafeCell<T>` (like `std::cell::UnsafeCell<T>`) ***does not*** provide any runtime safety, which one should opt for whenever possible.
- `static mut` ***is not*** the only way to initialise globals; `std::sync::LazyLock<T,_>` (unstable) and `std::sync::OnceLock<T>` provide initialisation on first access and initialisation once, respectively.
- Unsafe code ***should***(probably) be avoided or at least put you on guard.
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
A lint can be declared for 2024 with a `FutureIncompatibilityReason` of `EditionError` triggered upon detection of a declaration in HIR or the AST. For 2027 a check can be added to the `Parser` struct in the `rustc_parse` crate.

There is little to no use of `static mut` in the compiler, it is mostly present in `std` and in the implementation of the runtime and in a fashion similar to the above code examples; the few points at which `static mut` is used can be migrated to `SyncUnsafeCell` without causing too much ado.

A quick code search seemingly shows thousands of usages of `pub static mut`, and, at first glance, this gives the impression of mass breakage, but it is important to consider the following:
1. Whether or not the declared variable is actually reachable from the crate root.
2. Whether or not it is actual API. (Using mutable statics as API surface is present in C, but likely is highly discouraged and rare in Rust).

In any case, use of old edition `static mut` variables can be allowed for backwards compatibility.
# Drawbacks
[drawbacks]: #drawbacks
Verbosity increases slightly as values need to be wrapped in order to be placed in statics and methods need to be used to get at the underlying data. 

There is a hurdle to migrating to the 2027 edition (deny-by-default lints apparently don't count as breakage, so 2024 is free to have a warn-by-default or deny-by-default lint for deprecation), `cargo fix` can potentially perform some fixes, but the effort of implementing `cargo fix` for this has not yet been gauged.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
- Do nothing.
- Deprecate `static mut` but don't remove it.
- Replace `static mut` declarations with a `legacy_mutable_static` attribute on `static` declarations.

Doing nothing would result in a redundant feature that serves no purpose other than being a potential trap for users.

Deprecation without removal could be done, but after the presumed migration of at least a majority of the ecosystem after the deprecation lint there is no reason to keep the feature in the language. 

Adding an attribute is a decent addition, as it allows people to continue to declare `static mut` variables for any reason (though none come to mind) instead of removing it outright with no way to get at the old feature, but it is still redundant.
# Prior art
[prior-art]: #prior-art
## Work/discussion directly relevant
- Consider deprecation of UB-happy static mut at [#53649](https://github.com/rust-lang/rust/issues/53639)
- Disallow *references* to `static mut` [Edition Idea] at [#114447](https://github.com/rust-lang/rust/issues/114447)
- Deprecate static mut (in the next edition?) on [IRLO](https://internals.rust-lang.org/t/deprecate-static-mut-in-the-next-edition/19975/12)

## Notable, not directly relevant
- `SyncUnsafeCell` at [#95439](https://github.com/rust-lang/rust/issues/95439)
- `LazyCell/Lock` at [#109736](https://github.com/rust-lang/rust/issues/109736)
- `OnceCell/Lock` at [#74465](https://github.com/rust-lang/rust/issues/74465)
- `AssertThreadSafe`at [T-libs#322](https://github.com/rust-lang/libs-team/issues/322) (IMPORTANT)

Many have tried to remove/deprecate `static mut` before, the feature is now sufficiently redundant and 
subject to replacement to put forth a plan for its eventual removal.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should the deprecation lint be warn-by-default or deny-by-default?
- Should we have `cargo fix` support for migration? It's technically possible, but reasonably high effort.
- Should an attribute be included to allow declaring new `static mut` variables even after the feature is removed?

# Future possibilities
[future-possibilities]: #future-possibilities
I can't think of any besides what was mentioned as of yet.
