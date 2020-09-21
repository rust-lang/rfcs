- Feature Name: `stable_rustdoc_urls`
- Start Date: 2020-09-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/2988)
<!-- TODO -->
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Make the URLs that rustdoc generates stable relative to the docs being generated,
not just relative to the rustdoc version.

# Motivation
[motivation]: #motivation

[Rustdoc] generates a separate HTML page for each [item] in a crate.
The URL for this page is currently stable relative to rustdoc; in other words,
Rustdoc guarantees that updating `rustdoc` without changing the source code will not change the URL generated.
This is a 'de facto' guarantee - it's not documented, but there's been no breaking change to the format since pre-1.0.

However, Rustdoc does _not_ currently guarantee that making a semver-compatible change to your code will preserve the same URL.
This means that, for instance, making a type an `enum` instead of a `struct` will change the URL,
even if your change is in every other way semver-compatible. After this RFC, Rustdoc will guarantee that the URL would stay the same.

The primary motivation for this feature is to allow linking to a semantic version
of the docs, rather than an exact version. This has several applications:

- [docs.rs] could link to `/package/0.2/path` instead of `/package/0.2.5/path`, making the documentation users see more up-to-date ([rust-lang/docs.rs#1055])
- blogs could link to exact URLs without fear of the URL breaking ([rust-lang/rust#55160 (comment)][55160-blog])
- URLs in the standard library documentation would change less often ([rust-lang/rust#55160][55160])

Note that this is a different, but related, use case than [intra-doc links].
Intra-doc links allow linking consistently in the presence of re-exports for _relative_ links.
This is intended to be used for _absolute_ links. Additionally, this would allow linking consistently
outside of Rust code.

[Rustdoc]: https://doc.rust-lang.org/rustdoc/
[item]: https://doc.rust-lang.org/reference/items.html
[docs.rs]: https://docs.rs/
[could link]: https://github.com/rust-lang/docs.rs/issues/1055
[55160]: https://github.com/rust-lang/rust/issues/55160
[55160-blog]: https://github.com/rust-lang/rust/issues/55160#issuecomment-680751534
[intra-doc links]: https://github.com/rust-lang/rfcs/blob/master/text/1946-intra-rustdoc-links.md
[rust-lang/docs.rs#1055]: https://github.com/rust-lang/docs.rs/issues/1055

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rustdoc will make the following changes to URL structure:

- Item pages will be dependent only on the namespace, not the type of the item.

	Consider the struct `std::process::Command`.
	Currently, the URL for it looks like `std/process/struct.Command.html`.
	This RFC proposes to change the URL to `std/process/t.Command.html`.
	Pages named `kind.name.html` would still be generated (to avoid breaking existing links),
	but would immediately redirect to the new URL.

- Re-exports will generate a page pointing to the canonical version of the documentation.

	Consider the following Rust code:
	
	```rust
	pub struct Foo;
	```
	
	Rustdoc currently generates a page for this at `struct.Foo.html`.
	Now, consider what happens when you move the struct to a different module and re-export it
	(which is a semver-compatible change):

	```rust
	pub mod foo { pub struct Foo; }
	pub use foo::Foo;
	```

	This generates a page at `foo/struct.Foo.html`, but _not_ at `struct.Foo.html`.
	After this change, rustdoc will generate a page at the top level which redirects
	to the version nested in the module.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Item pages will be dependent only on the namespace

Rust has [three namespaces][Namespace]. For simplicity, this will only consider items that can be at the module level,
since function locals cannot be documented.

1. The value namespace. This includes `fn`, `const`, and `static`.
2. The type namespace. This includes `mod`, `struct`, `union`, `enum`, `trait`, and `type`.
3. The macro namespace. This includes `macro_rules!`, attribute macros, and derive macros.

Rust does not permit there to be overlaps within a namespace;
overlaps in globbing cause the glob import to be shadowed and [unusable].
This means that a name and namespace is [always sufficient][find-name-namespace] to identify an item.

Rustdoc will use the following links, depending on the namespace:

- `v.Name.html` for values
- `t.Name.html` for types
- `m.Name.html` for macros

Rustdoc will continue to use directories (and `index.html`) for modules.

[Namespace]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/def/enum.Namespace.html
[find-name-namespace]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/struct.AssociatedItems.html#method.find_by_name_and_namespace
[unusable]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=548f2f5e08600d4ad732c407ab3dd59f

## Re-exports will generate a page pointing to the canonical version

The redirect page will go in the same place as the re-export would be if it
were inlined with `#[doc(inline)]` after this RFC.

There will _not_ be a page generated at `kind.name.html` at the level of the re-export, since it's not possible for there to be any existing links there that were not broken.

# Drawbacks
[drawbacks]: #drawbacks

- Rust is case-sensitive, but some filesystems (especially on Windows) are not,
  so there are naming collisions in the files Rustdoc generates ([#25879]).
  If Rustdoc combines several 'kinds' into one namespace, there will be more conflicts than currently:

```rust
struct Command; // page generated at `t.Command.html`
enum command {} // page generated at `t.command.html`
```

**@nemo157** has kindly conducted a survey of the docs.rs documentation and found
that there are about 700,000 items that [currently overlap]. After this change,
that would go up to about [850,000 items that overlap][overlap-after-change].
docs.rs has 308,064,859 total items in the inventory, so previously 0.23% files conflicted
and after this RFC 0.28% files will conflict.

In the opinion of the author, since this is an existing problem,
it does not need to be solved in order to go forward with the RFC.

[currently overlap]: https://ipfs.io/ipfs/QmfZatebkVFdEUYQtPaAitsBbmLKAtQgaWLciSnjtLAWfv/case-conflicts.txt.zst
[overlap-after-change]: https://ipfs.io/ipfs/QmfZatebkVFdEUYQtPaAitsBbmLKAtQgaWLciSnjtLAWfv/cased-namespace-conflicts.txt.zst

[#25879]: https://github.com/rust-lang/rust/issues/25879

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## How were the URLs chosen?

There were three main criteria for choosing the URLs (in vauge order of priority):

1. They should be based on the namespace, not the 'kind' of the item. Otherwise there's not much point to the RFC, because the URLs won't be stable.
2. They should make sense when viewed; for example `a`, `b`, `c` would be bad choices for the names.
3. They should be fairly short, so they're easy to type; for example `type_namespace.` would not be a great choice.

`t.` and `m.` were partly chosen based on precedent in [#35236] (but see 'Naming alternatives' below for the main reason).

### Naming alternatives

Note that these names are easy to 'bikeshed' and don't substantially change the RFC.

- Rustdoc could remove the `v.` prefix for items in the value namespace.
  This would make the URLs for functions slightly less confusing, but introduce a conflict for functions named `index()`, since rustdoc has to generate `index.html` for modules.
- Rustdoc could lengthen the prefixes to `value.`, `type.` and `macro.`. This makes the URLs easier to read, at the cost of making them more confusing for traits (consider `type.Trait.html`).
- Rustdoc could use the existing specific names only when there is no risk of a semver-compatible change being able to change the kind. This would need careful inspection to make sure there is in fact no risk. It would also be slightly inconsistent with other URLs.

[#35236]: https://github.com/rust-lang/rust/pull/35236

## Alternatives

These alternatives are substantial changes to the RFC.

- Rustdoc could stabilize the links it uses, but without keeping backwards compatibility by not generating `kind.name.html`. This has little benefit over the RFC,
  other than slightly less disk space used and implementation complexity.
- Rustdoc could keep the status quo. This can cause no naming conflicts on Windows, but has the drawback that links could silently break even for semver-compatible changes.
- Rustdoc could choose to make URLs stable _neither_ across rustdoc versions nor the version of the code being documented,
  for example by using `kind.name.SHA256SUM(rustdoc version).html`. This makes it more clear that the URLs are not intended to be stable,
  at the cost of breaking links across much of the ecosystem.

# Prior art
[prior-art]: #prior-art

- `go doc` generates all documentation on one page and uses URL hashes, without namespacing.
   This causes conflicts when two items from different namespaces are in the same package.
- `java` only allows classes at the top-level, so `javadoc` has no need for namespacing.
   To distinguish between methods and fields, `javadoc` includes `()` in the URL fragment for methods.
- `Racket` only allows functions at the top-level, and so has no need for namespacing.
- `doxygen` names HTML pages after their C++ source files, and appends a random hash in the URL fragment to avoid namespace conflicts.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is there a way to resolve the naming conflicts on Windows? If not, is that worth blocking the RFC, given there are existing conflicts?
- Are there other semver-incompatibilities in the current pages that haven't been addressed?

# Future possibilities
[future-possibilities]: #future-possibilities

Rustdoc could stabilize page hashes:

- Associated items for traits will use the same hash as for types, unless there is a conflict with the hash for a type.

	A change from
	```rust
	struct S;
	impl S { fn f() {} }
	```
	to
	```rust
	struct S;
	trait T { fn f(); }
	impl T for S { fn f() {} }
	```
	is semver compatible, but currently breaks the hash (it changes from `#method.f` to `#tymethod.f`).
	Rustdoc could change it to use `#method.f` when there is no conflict with other traits or inherent associated items.
	For example, the second version of the code above would use `#method.f`, but the code below would use `#tymethod.f`
	for the version in the trait:
	```rust
	struct S;
	impl S { fn f() {} }

	trait T { fn f(); }
	impl T for S { fn f() {} }
	```

	This matches Rust semantics: `S::f()` refers to the function for the type if it exists,
	and the method for a trait it implements if not.

- Associated items for traits will contain the name of the trait if there is a conflict.

	Currently, the `from` function in both of the trait implementations has the same hash:
	```rust
	enum Int {
		A(usize),
		B(isize),
	}
	impl From<usize> for Int {
		fn from(u: usize) {
			Int::A(u)
		}
	}
	impl From<isize> for Int {
		fn from(i: isize) {
			Int::B(i)
		}
	}
	```
	This means it is _impossible_ to refer to one or the other (which has [caused trouble for intra-doc links][assoc-items]).
	Rustdoc could instead include the name and generic parameters in the hash: `#method.from-usize.from` and `method.from-isize.from`.
	It is an unresolved question how this would deal with multiple traits with the same name,
	or how this would deal with types with [characters that can't go in URL hashes][hashes] (such as `()`).
	Rustdoc could possibly use percent-encoding for the second issue.

- All other URL fragments would be kept the same:
	+ `#variant.{name}` for enum variants
	+ `#structfield.{name}` for struct fields
	+ `#variant.{parent}.field.{name}` for anonymous structs in enums (`enum Parent { A { field: usize }}`).
	   This may require redesign to avoid conflicts in fields between different variants.
	+ `#associatedconstant.{name}` for associated constants in traits. This may require redesign when [RFC 195] is implemented.
	+ `#associatedtype.{name}` for associated types (same as above)

[hashes]: https://url.spec.whatwg.org/#fragment-state
[assoc-items]: https://github.com/rust-lang/rust/issues/76895
[RFC 195]: https://github.com/rust-lang/rfcs/blob/master/text/0195-associated-items.md#inherent-associated-items
