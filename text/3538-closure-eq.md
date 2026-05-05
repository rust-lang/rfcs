- Feature Name: `closure_eq`
- Start Date: 2023-10-30
- RFC PR: [rust-lang/rfcs#3538](https://github.com/rust-lang/rfcs/pull/3538)
- Rust Issue: None yet

# Summary
[summary]: #summary

Add a mechanism and syntax to implement `PartialEq` and `Eq` for closures:

```rust
// Simple example:
fn multiply_by(n: i32) -> impl PartialEq + Eq + Fn(i32) -> i32 {
	impl PartialEq + Eq move |v: i32| v * n
}

let double = multiply_by(2);
let multiply_by_two = multiply_by(2);
let triple = multiply_by(3);
assert!(double == multiply_by_two);
assert!(double != triple);

// React-like example:

/// Hypothetical button widget and its properties
#[derive(PartialEq)]
struct Button<F> {
	/// Closure to invoke when the button is clicked
	on_click: F,
}
impl<F: Fn() + PartialEq> Button<F> {
	/// Given the old and new button parameters, determine if the widget
	/// needs to be updated.
	pub fn rerender(old: &Self, new: &Self) -> Option<Widget> {
		// If the closure has changed, create a new widget with the new closure
		if old != new {
			Some(create_ui_widget(new))
		} else {
			None
		}
	}
}

fn render_my_button(input: IntegerInput) {
	let value: i32 = input.get();
	render(Button {
		on_click: impl PartialEq move || println!("Value is: {}", value),
	})
}
```

# Motivation
[motivation]: #motivation

In some situations, there is a need to compare closures to see if the values they have closed over are different.

For example, React-like UI frameworks optimize by only re-rendering UI elements if their parameters have changed, which necessitates that their parameters implement `PartialEq`. They also need to support passing in functions within these properties - for example, a callback to call when a button is clicked.

Because closures don't implement `PartialEq`, UI frameworks have to use workarounds to support callbacks, for example, by bundling a function with a `PartialEq`-implementing "state" structure that is passed along side the normal arguments when the callback is called. This involves a lot of boilerplate, and makes users unable to use the conveniences involving the `Fn`, `FnMut`, and `FnOnce` items, since they can't use those traits.

This RFC solves this issue by allowing closures to opt into implementing the `PartialEq` and `Eq` traits, similar to how closures can currently implement `Clone` and `Copy`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

By prefixing a closure with `impl PartialEq` or `impl PartialEq + Eq`, closures can opt into implementing the `PartialEq` and `Eq` traits. All values of the closure must implement `PartialEq` or `Eq`.

```rust
// `x` is `PartialEq + Eq`, so the closure returned by `gen_print_int` is `PartialEq + Eq`:
fn gen_print_int(x: i32) -> impl Eq + Fn() {
	impl PartialEq + Eq move || println!("{}", x)
}
let print_two = gen_print_int(2);
let print_2 = gen_print_int(2);
let print_three = gen_print_int(3);
assert!(print_two == print_2);
assert!(print_two != print_three);

// `y` is `PartialEq` but not `Eq`, so the closure returned by `gen_print_float`
// is `PartialEq` but not `Eq`:
fn gen_print_float(y: f32) -> impl PartialEq + Fn() {
	impl PartialEq move || println!("{}", y)
}
let print_half = gen_print_float(0.5);
let print_one_div_two = gen_print_float(0.5);
let print_pi = gen_print_float(3.14);
assert!(print_half == print_one_div_two);
assert!(print_half != print_pi);
```

Noe that this does not change the fact that closures are anonymous types. Therefore, only closures originating from the same code will have the same type and will be comparable.

The implementation of `PartialEq` and `Eq` is opt in, to avoid issues where closures may unintentionally gain or lose `PartialEq` or `Eq` in an API breaking way.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Alter closure parsing to accept an optional prefx of `impl [trait-bounds]`.

If the optional prefix is specified and the trait bounds is `PartialEq`, the closures's closed over values must also implement `PartialEq`, and the closure will implement `PartialEq<Self>` by comparing all of its closed over values.

If the optional prefix is specified and the trait bounds is `Eq`, `PartialEq + Eq`, or `Eq + PartialEq`: The closures's closed over values must also implement `Eq`; the closure will implement `PartialEq<Self>` by comparing all of its closed over values (as with `impl PartialEq`); and the closure will additionally implement `Eq`.

If the prefix is not specified, closures are parsed and generated as normal.

Implementation should be similar to how closures currently implement `Clone` and `Copy` automatically.

# Drawbacks
[drawbacks]: #drawbacks

Similar to `Clone` and `Copy` on closures, users will have to reason about which variables are being captured to understand whether or not a closure is `PartialEq` or `Eq`. The explicit syntax helps highlight that `PartailEq` or `Eq` is necessary.

Because closures are anonymous types, the same closure code in two different places will not be comparable, since their types differ. This may be confusing for users who expect two closures that look the same to be comparable. However, this problem already exists with assigning closures to variables and fields.

This may be confusing for types with shared ownership and interior mutability, as closures may compare equal but update unrelated objects. However this issue exists on its own when comparing the objects outside of closures as well.

If a guarentee is made that closures must use all closed over variables in their `PartialEq` implmenetation, it may forbid optimizing out the variables.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC proposes an opt-in syntax to `PartialEq` and `Eq`, based on feedback and concerns about closures gaining or losing `PartialEq` or `Eq` unexpectedly and the resulting API breakage. This is currently in contrast to how `Clone` and `Copy` are currently automatically implemented in applicable closures. Doing the same automatic implementation should be possible for `PartialEq` and `Eq`, if desired.

# Prior art
[prior-art]: #prior-art

Most languages with first-class functions implement equality comparison, but only check if two closures are the same object, rather than comparing closed-over variables. Examples of such are Python, Lua, and JavaScript.

Some projects pass in a combination of a function and state, passing a reference to state into the closure when it is called and using `PartialEq` on the state object. This is similar to how closures are already implemented under the hood. For example, the Yew UI framework has a `Deps` generic on many of its hooks that accept closures for the state object.

The [`serde_closure`](https://lib.rs/crates/serde_closure) crate implements a procedural macro that implements `Clone`, `PartialEq`, `serde` traits, and a few others on closures by scanning a closure body for references to undeclared variables, then storing them in a structure that represents the closure - essentially, a manual implementaiton of closures.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should this be op-in, or automatic like `Clone` and `Copy` are? If not, is there any concern of the `[Partial]Eq`-ness of a closure being part of a public API?
  - In particular, how would API breakage happen? Closures can only be passed/returned via `impl` and `dyn` syntaxes, where `PartialEq` must be added for the PartialEq-ness of the closure to be visible.
- In discussion about this RFC, there was some commentary about the automatic implementations of `Clone` and `Copy` not being desired. We could potentially address this in a backwards compatible way where if the `impl ... ||` syntax is used, the closure will not automatically implement `Clone` and `Copy` and must opt into them manually. Should this happen, as part of this RFC or otherwise?
- What guarentees does/should the compiler make regarding optimizing away closed over variables and how those would be used in the closure's implementation of `PartialEq`/`Eq`? How would this also apply for generators and coroutines?
  - What guarentees do closures currently have about `Clone`? It's likely that everything is cloned.
- Concerns about function code merging affecting equality. Could a `dyn` reference to a closure share a vtable with another closure, and be downcasted to another closure type via `Any`?
  - Does this happen today with closures that implement `Any`? I.e. is there a case where a closure defined in two different places share `TypeID`s?
  - Does it matter, considering they will have the same code? For the React-like UI case, it shouldn't, but there may be other use cases where it does.
  - We currently allow `PartialEq` between `fn` pointers, which can also be merged.

# Future possibilities
[future-possibilities]: #future-possibilities

The opt-in syntax can be extended to other traits as well - for example, `Debug`.

The syntax could also be extended to be used with `async [move] {}` blocks and generators, where it may be desireable for them to implement `PartialEq`, `Eq`, `Clone`, or other traits.
