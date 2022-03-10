- Feature Name: `packages_as_namespaces`
- Start Date: (fill me in with today's date, 2022-03-09)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

Grant exclusive access to publishing crates `parent/foo` for owners of crate `parent`.

Namespaced crates can be named in Rust code using underscores (e.g. `parent_foo`).

# Motivation

While Rust crates are practically unlimited in size, it is a common pattern for organizations to split their projects into many crates, especially if they expect users to only need a fraction of their crates.

For example, [unic](https://crates.io/search?page=1&per_page=10&q=unic-), [tokio](https://crates.io/search?page=1&per_page=10&q=tokio-), [async-std](https://crates.io/search?page=1&per_page=10&q=async-), [rusoto](https://crates.io/search?q=rusoto) all do something like this, with lots of `projectname-foo` crates. At the moment, it is not necessarily true that a crate named `projectname-foo` is maintained by `projectname`, and in some cases that is even desired! E.g. `serde` has many third party "plugin" crates like [serde-xml-rs](https://github.com/RReverser/serde-xml-rs). Similarly, [async-tls](https://crates.io/crates/async-tls) is a general crate not specific to the async-std ecosystem.

Regardless, it is nice to have a way to signify "these are all crates belonging to a single organization, and you may trust them the same". When starting up [ICU4X](https://github.com/unicode-org/icu4x/), we came up against this problem: We wanted to be able to publish ICU4X as an extremely modular system of `icu-foo` or `icu4x-foo` crates, but it would be confusing to users if third-party crates could also exist there (or take names we wanted to use).

It's worth clarifying, the use of "organization" here can refer to "projects" as well, where a project wishes multiple sub-crates of a particular project to be under the same umbrella. For example, `serde-derive` refers to "the `derive` component of the `serde` project", and `icu-provider` refers to "the provider component of the `icu` project".

This is distinct from the general problem of squatting -- with general squatting, someone else might come up with a cool crate name before you do. However, with `projectname-foo` crates, it's more of a case of third parties "muscling in" on a name you have already chosen and are using.

# Guide-level explanation

If you own a crate `foo`, you may create a crate namespaced under it as `foo/bar`. Only people who are owners of `foo` may _create_ a crate `foo/bar` (and all owners of `foo` are implicitly owners of `foo/bar`). After such a crate is created, additional per-crate publishers may be added who will be able to publish subsequent versions as usual.

The crate can be imported in Cargo.toml using its name as normal:

```toml
[dependencies]
"foo/bar" = "1.0"
```


In Rust code, the slash gets converted to an underscore, the same way we do this for dashes.

```rs
use foo_bar::Baz;
```

# Reference-level explanation

`/` is now considered a valid identifier inside a crate name Crates.io. For now, we will restrict crate names to having a single `/` in them, not at the beginning or end of the name, but this can be changed in the future.

When publishing a crate `foo/bar`, if the crate does not exist, the following must be true:

 - `foo` must exist
 - The user publishing the crate must be an owner of `foo`

For the crate `foo/bar`, all owners of `foo` are always considered owners of `foo/bar`, however additional owners may be added. People removed from ownership of `foo` will also lose access to `foo/bar` unless they were explicitly added as owners to `foo/bar`.

Crates.io displays `foo/bar` crates with the name `foo/bar`, though it may stylistically make the `foo` part link to the `foo` crate.

The [registry index trie](https://doc.rust-lang.org/nightly/cargo/reference/registries.html#index-format) may represent subpackages by placing `foo/bar` in `foo@/bar`, placed next to where `foo` is in the trie (i.e. the full path will be `fo/foo@/bar`).

No changes are made to `rustc`. When compiling a crate `foo/bar`, Cargo will automatically pass in `--crate-name foo_bar`, and when referring to it as a dependency Cargo will use `--extern foo_bar=....`. This is the same thing we currently do for `foo-bar`.

If you end up in a situation where you have both `foo/bar` and `foo-bar` as active dependencies of your crate, your code will not compile and you must [rename](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml) one of them.

The `features = ` key in Cargo.toml continues parsing `foo/bar` as "the feature `bar` on dependency `foo`", however it now will unambiguously parse strings ending with a slash (`foo/` and `foo/bar/`) as referring to a dependency, as opposed to feature on a dependency. Cargo may potentially automatically handle the ambiguity or error about it.

# Drawbacks

## Slashes
So far slashes as a "separator" have not existed in Rust. There may be dissonance with having another non-identifier character allowed on crates.io but not in Rust code. Dashes are already confusing for new users. Some of this can be remediated with appropriate diagnostics on when `/` is encountered at the head of a path.


Furthermore, slashes are ambiguous in feature specifiers (though a solution has been proposed above for this):

```toml
[dependencies]
"foo" = "1"
"foo/std" = { version = "1", optional = true }

[features]
# Does this enable crate "foo/std", or feature "std" of crate "foo"?
default = ["foo/std"]
```


## Namespace root taken
Not all existing projects can transition to using namespaces here. For example, the `unicode` crate is reserved, so `unicode-rs` cannot use it as a namespace despite owning most of the `unicode-foo` crates. In other cases, the "namespace root" `foo` may be owned by a different set of people than the `foo-bar` crates, and folks may need to negotiate (`async-std` has this problem, it manages `async-foo` crates but the root `async` crate is taken by someone else). Nobody is forced to switch to using namespaces, of course, so the damage here is limited, but it would be _nice_ for everyone to be able to transition.


## Dash typosquatting

This proposal does not prevent anyone from taking `foo-bar` after you publish `foo/bar`. Given that the Rust crate import syntax for `foo/bar` is `foo_bar`, same as `foo-bar`, it's totally possible for a user to accidentally type `foo-bar` in `Cargo.toml` instead of `foo/bar`, and pull in the wrong, squatted, crate.

We currently prevent `foo-bar` and `foo_bar` from existing at the same time. We _could_ do this here as well, but it would only go in one direction: if `foo/bar` exists, neither `foo-bar` nor `foo_bar` will be allowed to be published. However, if `foo-bar` or `foo_bar` exist, we would choose to allow `foo/bar` to be published, because we don't want to limit the use of names within a crate namespace due to crates outside the namespace existing. This limits the "damage" to cases where someone pre-squats `foo-bar` before you publish `foo/bar`, and the damage can be mitigated by checking to see if such a clashing crate exists when publishing, if you actually care about this attack vector. There are some tradeoffs there that we would have to explore.

One thing that could mitigate `foo/bar` mapping to the potentially ambiguous `foo_bar` is using something like `foo::crate::bar` or `~foo::bar` or `foo::/bar` in the import syntax.


## Slow migration

Existing projects wishing to use this may need to manually migrate. For example, `unic-langid` may become `unic/langid`, with the `unic` project maintaining `unic-langid` as a reexport crate with the same version number. Getting people to migrate might be a bit of work, and furthermore maintaining a reexport crate during the (potentially long) transition period will also be some work. Of course, there is no obligation to maintain a transition crate, but users will stop getting updates if you don't.

A possible path forward is to enable people to register aliases, i.e. `unic-langid` is an alias for `unic/langid`.

# Rationale and alternatives

This change solves the ownership problem in a way that can be slowly transitioned to for most projects.


## Using identical syntax in Cargo.toml and Rust source

This RFC in its current form does not propose changes to the Rust compiler to allow slash syntax (or whatever) to parse as a Rust path. Such changes could be made (though not with slash syntax due to parsing ambiguity, see [below](#Separator choice) for more options); this RFC is attempting to be minimal in its effects on rustc.

However, the divergence between Cargo.toml and rustc syntax does indeed have a complexity cost, and may be confusing to some users. Furthermore, it increases the chances of [Dash typosquatting](#Dash typosquatting) being effective.

## `foo::bar` on crates.io and in Rust

While I cover a bunch of different separator choices below, I want to call out `foo::bar` in particular. If we went with `foo::bar`, we could have the same crate name in the Rust source and Cargo manifest. This would be _amazing_.

Except, of course, crate `foo::bar` is ambiguous with module `bar` in crate `foo` (which might actually be a reexport of `foo::bar` in some cases).

This can still be made to work, e.g. we could use `foo::crate::bar` to disambiguate, and encourage namespace-using crates to ensure that `mod bar` in crate `foo` either doesn't exist or is a reexport of crate `foo::bar`. I definitely want to see this discussed a bit more.


## Whole crate name vs leaf crate name in Rust source

It may be potentially better to use just the leaf crate name in Rust source. For example, when using crate `foo/bar` from Cargo.toml, the Rust code would simply use `bar::`. Cargo already supports [renaming dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml) which can be used to deal with any potential ambiguities here. This also has the added benefit of not having to worry about the separator not parsing as valid Rust.

A major drawback to this approach is that while it addresses the "the namespace is an organization" use case quite well (e.g. `unicode/segmentation` vs `unicode/line-break` and `rust-lang/libc` vs `rust-lang/lazy-static`, etc), this is rather less amenable to the "the namespace is a _project_" case (e.g. `serde` vs `serde/derive`, `icu/datetime` vs `icu/provider`, etc), where the crates are related not just by provenance. In such cases, users may wish to rename the crates to avoid confusion in the code. This may be an acceptable cost.

## Separator choice

A different separator might make more sense.

We could continue to use `/` but also use `@`, i.e. have crates named `@foo/bar`. This is roughly what npm does and it seems to work. The `@` would not show up in source code, but would adequately disambiguate crates and features in Cargo.toml and in URLs.

We could perhaps have `foo-*` get autoreserved if you publish `foo`, as outlined in https://internals.rust-lang.org/t/pre-rfc-hyper-minimalist-namespaces-on-crates-io/13041 . I find that this can lead to unfortunate situations where a namespace traditionally used by one project (e.g. `async-*`) is suddenly given over to a different project (the `async` crate). Furthermore, users cannot trust `foo-bar` to be owned by `foo` because the vast number of grandfathered crates we will have.

Another separator idea would be to use `::`, e.g. `foo::bar`. This looks _great_ in Rust code, provided that the parent crate is empty and does not also have a `bar` module. See the section above for more info.

Triple colons could work. People might find it confusing, but `foo:::bar` evokes Rust paths without being ambiguous.

We could use `~` which enables Rust code to directly name namespaced packages (as `~` is no longer used in any valid Rust syntax). It looks extremely weird, however.

We could use dots (`foo.bar`). This does evoke some similarity with Rust syntax, however there are ambiguities: `foo.bar` in Rust code could either mean "the field `bar` of local/static `foo`" or it may mean "the crate `foo.bar`".

Note that unquoted dots have semantic meaning in TOML, and allowing for unquoted dots would freeze the list of dependency subfields allowed (to `version`, `git`, `branch`, `features`, etc).


We could reverse the order and use `@`, i.e. `foo/bar` becomes `bar@foo`. This might be a tad confusing, and it's unclear how best to surface this in the source.


## Separator mapping

The proposal suggests mapping `foo/bar` to `foo_bar`, but as mentioned in the typosquatting section, this has problems. There may be other mappings that work out better:

 - `foo::bar` (see section above)
 - `foo::crate::bar`
 - `foo::/bar`
 - `~foo::bar`

and the like.


## User / org namespaces

Another way to handle namespacing is to rely on usernames and GitHub orgs as namespace roots. This ties `crates.io` strongly to Github -- currently while GitHub is the only login method, there is nothing preventing others from being added.

Furthermore, usernames are not immutable, and that can lead to a whole host of issues.

## Registry trie format

Instead of placing `foo/bar` in `foo@/bar`, it can be placed in `foo@bar` or something else. 

# Prior art

This proposal is basically the same as https://internals.rust-lang.org/t/pre-rfc-packages-as-namespaces/8628 and https://internals.rust-lang.org/t/pre-rfc-idea-cratespaces-crates-as-namespace-take-2-or-3/11320 .

Namespacing has been discussed in https://internals.rust-lang.org/t/namespacing-on-crates-io/8571 , https://internals.rust-lang.org/t/pre-rfc-domains-as-namespaces/8688, https://internals.rust-lang.org/t/pre-rfc-user-namespaces-on-crates-io/12851 , https://internals.rust-lang.org/t/pre-rfc-hyper-minimalist-namespaces-on-crates-io/13041 , https://internals.rust-lang.org/t/blog-post-no-namespaces-in-rust-is-a-feature/13040/4 , https://internals.rust-lang.org/t/crates-io-package-policies/1041/37, https://internals.rust-lang.org/t/crates-io-squatting/8031, and many others.

# Unresolved questions

 - Is `/` really the separator we wish to use?
 - How do we avoid ambiguity in feature syntax
 - Is there a way to avoid `foo/bar` turning in to the potentially ambiguous `foo_bar`?
 - Can we mitigate some of typosquatting?
 - How can we represent namespaced crates in the registry trie?
 - How do we represent namespaced crates in the URLs of crates.io and docs.rs?

# Future possibilities

We can allow multiple layers of nesting if people want it.

# FAQ

## What if I don't want to publish my crate under a namespace?

You don't have to, namespaces are completely optional when publishing.

## Does this stop people from squatting on `coolcratename`?

No, this proposal does not intend to address the general problem of squatting (See [crates.io's policy](https://crates.io/policies#squatting), a lot of this has been discussed many times before). Instead, it allows people who own an existing crate to publish sub-crates under the same namespace. In other words, if you own `coolcratename`, it stops people from squatting `coolcratename/derive`.
