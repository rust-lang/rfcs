- Feature Name: `packages_as_namespaces`
- Start Date: 2022-03-09
- RFC PR: [rust-lang/rfcs#3243](https://github.com/rust-lang/rfcs/pull/3243)
- Rust Issue: [rust-lang/rust#122349](https://github.com/rust-lang/rust/issues/122349)

# Summary

Languages like C++ have open namespaces where anyone can write code in any namespace.  In C++'s case, this includes the `std` namespace and is only limited by convention.  In contrast, Rust has closed namespaces which can only include code from the original namespace definition (the crate).

This proposal extends Rust to have partially open namespaces by allowing crate owners to create crates like `parent::foo` that will be available as part of the crate `parent`'s namespace.  To protect the use of open namespaces, the owners of `parent` has exclusive access to publishing crates in that namespace.

# Motivation

While Rust crates are practically unlimited in size, it is a common pattern for organizations to split their projects into many crates, especially if they expect users to only need a fraction of their crates or they have different backwards compatibility guarantees.

For example, [unic](https://crates.io/search?page=1&per_page=10&q=unic-), [tokio](https://crates.io/search?page=1&per_page=10&q=tokio-), [async-std](https://crates.io/search?page=1&per_page=10&q=async-), [rusoto](https://crates.io/search?q=rusoto) all do something like this, with lots of `projectname-foo` crates. At the moment, it is not necessarily true that a crate named `projectname-foo` is maintained by `projectname`, and in some cases that is even desired! E.g. `serde` has many third party "plugin" crates like [serde-xml-rs](https://github.com/RReverser/serde-xml-rs). Similarly, [async-tls](https://crates.io/crates/async-tls) is a general crate not specific to the async-std ecosystem.

Regardless, it is nice to have a way to signify "these are all crates belonging to a single project, and you may trust them the same" and discover these related crates. When starting up [ICU4X](https://github.com/unicode-org/icu4x/), we came up against this problem: We wanted to be able to publish ICU4X as an extremely modular system of `icu-foo` or `icu4x-foo` crates, but it would be confusing to users if third-party crates could also exist there (or take names we wanted to use).

It's worth spending a bit of time talking about "projects" and "organizations", as nebulous as those terms are. This feature is *primarily* motivated by the needs of "projects". By this I mean a _single_ Rust API developed as multiple crates, for example `serde` and `serde::derive`, or `icu` and `icu::provider`, or `servo::script` and `servo::layout`. One would expect "projects" like this to live under a single Git repository according to the norms of project organization; they are logically a single project and API even if they are multiple crates.

The feature suggested here is unlikely to be used by "organizations" as this would put independent concerns in the same Rust API.  By "organizations", I mean a group of people who are coming together to build likely related crates, under the same "brand", likely developed in multiple repos under a GitHub organization.


The motivation here is distinct from the general problem of squatting -- with general squatting, someone else might come up with a cool crate name before you do. However, with `projectname-foo` crates, it's more of a case of third parties "muscling in" on a name you have already chosen and are using.

# Guide-level explanation

The owners of the `foo` crate may provide other crates under the `foo` namespace, like `foo::bar`.  For users, this makes its official status clearer and makes it easier to discover.

Users import these crates in Cargo.toml as normal:

```toml
[dependencies]
"foo" = "1.0.42"
"foo::bar" = "3.1"
```

They will then access this through a facade made of `foo` and all `foo::*` crates, for example:

```rs
let baz = foo::bar::Baz::new();
foo::render(baz);
```

Some reasons for `foo`s owner to consider using namespaces:
- Avoid name conflicts with third-party authors (since they are reserved)
- Improve discoverability of official crates
- As an alternative to feature flags for optional subsystems
- When different parts of your API might have different compatibility guarantees

When considering this, keep in mind:
- Does it makes sense for this new crate to be presented in the `foo` facade?
- How likely is a crate to move into or out of the namespace?
  - Moving the crate in or out of a namespace is a breaking change though it can be worked around by having the old crate re-export the new crate but that does add extra friction to the process.
  - There is not currently a mechanism to raise awareness with users that a crate has migrated into or out of a namespace and you might end up leaving users behind.
- If users import both `foo` and `foo::bar` but `foo` also has a `bar` item in its API that isn't just `foo::bar` re-exported, then rustc will error.

Only the owners of `foo` may _create_ the `foo::bar` crate (and all owners of `foo` are implicitly owners of `foo::bar`). After the `foo::bar` crate is created, additional per-crate publishers may be added who will be able to publish subsequent versions as usual.

# Reference-level explanation

_This section will maintain a distinction between "package" (a crates.io package) and "crate" (the actual rust library). The rest of the RFC does not attempt to make this distinction_

`::` is now considered valid inside package  names on Crates.io. For now, we will restrict package names to having a single `::` in them, not at the beginning or end of the name, but this can be changed in the future.

When publishing a package `foo::bar`, if the package does not exist, the following must be true:

 - `foo` must exist
 - The user publishing the package must be an owner of `foo`

For the package `foo::bar`, all owners of `foo` are always considered owners of `foo::bar`, however additional owners may be added. People removed from ownership of `foo` will also lose access to `foo::bar` unless they were explicitly added as owners to `foo::bar`.

Crates.io displays `foo::bar` packages with the name `foo::bar`, though it may stylistically make the `foo` part link to the `foo` package.

The [registry index trie](https://doc.rust-lang.org/nightly/cargo/reference/registries.html#index-format) may represent subpackages by placing `foo::bar` as just `foo::bar`.

`rustc` will need some changes. When `--extern foo::bar=crate.rlib` is passed in, `rustc` will include this crate during resolution as if it were a module `bar` living under crate `foo`. If crate `foo` is _also_ in scope, this will not automatically trigger any errors unless `foo::bar` is referenced, `foo` has a module `bar`, and that module is not just a reexport of crate `foo::bar`.

The autogenerated `lib.name` key for such a crate will just be `bar`, the leaf crate name, and the expectation is that to use such crates one _must_ use `--extern foo::bar=bar.rlib` syntax. There may be some better things possible here, perhaps `foo_bar` can be used here.


# Drawbacks


## Namespace root taken
Not all existing projects can transition to using namespaces here. For example, the `unicode` crate is reserved, so `unicode-rs` cannot use it as a namespace despite owning most of the `unicode-foo` crates. In other cases, the "namespace root" `foo` may be owned by a different set of people than the `foo-bar` crates, and folks may need to negotiate (`async-std` has this problem, it manages `async-foo` crates but the root `async` crate is taken by someone else). Nobody is forced to switch to using namespaces, of course, so the damage here is limited, but it would be _nice_ for everyone to be able to transition.


## Slow migration

Existing projects wishing to use this may need to manually migrate. For example, `unic-langid` may become `unic::langid`, with the `unic` project maintaining `unic-langid` as a reexport crate with the same version number. Getting people to migrate might be a bit of work, and furthermore maintaining a reexport crate during the (potentially long) transition period will also be some work. Of course, there is no obligation to maintain a transition crate, but users will stop getting updates if you don't.

A possible path forward is to enable people to register aliases, i.e. `unic-langid` is an alias for `unic::langid`.


## Requires rustc changes

There are alternate solutions below that don't require the _language_ getting more complex and can be done purely at the Cargo level. Unfortunately they have other drawbacks.


# Rationale and alternatives

This change solves the ownership problem in a way that can be slowly transitioned to for most projects.

## Slash as a separator

**For discussions about separator choice, please discuss them in [this issue](https://github.com/Manishearth/namespacing-rfc/issues/1) to avoid overwhelming the main RFC thread.**

A previous version of the RFC had `/` as a separator. It would translate it to `foo_bar` in source, and disambiguated in feature syntax with `foo/bar/` vs `foo/bar`. It had the following drawbacks:


### Slashes
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

### Dash typosquatting

This proposal does not prevent anyone from taking `foo-bar` after you publish `foo/bar`. Given that the Rust crate import syntax for `foo/bar` is `foo_bar`, same as `foo-bar`, it's totally possible for a user to accidentally type `foo-bar` in `Cargo.toml` instead of `foo/bar`, and pull in the wrong, squatted, crate.

We currently prevent `foo-bar` and `foo_bar` from existing at the same time. We _could_ do this here as well, but it would only go in one direction: if `foo/bar` exists, neither `foo-bar` nor `foo_bar` will be allowed to be published. However, if `foo-bar` or `foo_bar` exist, we would choose to allow `foo/bar` to be published, because we don't want to limit the use of names within a crate namespace due to crates outside the namespace existing. This limits the "damage" to cases where someone pre-squats `foo-bar` before you publish `foo/bar`, and the damage can be mitigated by checking to see if such a clashing crate exists when publishing, if you actually care about this attack vector. There are some tradeoffs there that we would have to explore.

One thing that could mitigate `foo/bar` mapping to the potentially ambiguous `foo_bar` is using something like `foo::crate::bar` or `~foo::bar` or `foo::/bar` in the import syntax.



### Using identical syntax in Cargo.toml and Rust source

The `/` proposal does not require changes to Rust compiler to allow slash syntax (or whatever) to parse as a Rust path. Such changes could be made (though not with slash syntax due to parsing ambiguity, see [below](#separator-choice) for more options); this RFC is attempting to be minimal in its effects on rustc.

However, the divergence between Cargo.toml and rustc syntax does indeed have a complexity cost, and may be confusing to some users. Furthermore, it increases the chances of [Dash typosquatting](#dash-typosquatting) being effective.

Some potential mappings for `foo/bar` could be:

 - `foo::bar` 
 - `foo::crate::bar`
 - `foo::/bar`
 - `~foo::bar`

and the like.

## Whole crate name vs leaf crate name in Rust source


**For discussions about separator choice, please discuss them in [this issue](https://github.com/Manishearth/namespacing-rfc/issues/1) to avoid overwhelming the main RFC thread.**

It may be potentially better to use just the leaf crate name in Rust source. For example, when using crate `foo/bar` from Cargo.toml, the Rust code would simply use `bar::`. Cargo already supports [renaming dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml) which can be used to deal with any potential ambiguities here. This also has the added benefit of not having to worry about the separator not parsing as valid Rust.

A major drawback to this approach is that while it addresses the "the namespace is an organization" use case quite well (e.g. `unicode/segmentation` vs `unicode/line-break` and `rust-lang/libc` vs `rust-lang/lazy-static`, etc), this is rather less amenable to the "the namespace is a _project_" case (e.g. `serde` vs `serde/derive`, `icu/datetime` vs `icu/provider`, etc), where the crates are related not just by provenance. In such cases, users may wish to rename the crates to avoid confusion in the code. This may be an acceptable cost.

## Separator choice


**For discussions about separator choice, please discuss them in [this issue](https://github.com/Manishearth/namespacing-rfc/issues/1) to avoid overwhelming the main RFC thread.**

A different separator might make more sense. See the [previous section](#slash-as-a-separator) for more on the original proposal of `/` as a separator.

We could continue to use `/` but also use `@`, i.e. have crates named `@foo/bar`. This is roughly what npm does and it seems to work. The `@` would not show up in source code, but would adequately disambiguate crates and features in Cargo.toml and in URLs.

We could perhaps have `foo-*` get autoreserved if you publish `foo`, as outlined in https://internals.rust-lang.org/t/pre-rfc-hyper-minimalist-namespaces-on-crates-io/13041 . I find that this can lead to unfortunate situations where a namespace traditionally used by one project (e.g. `async-*`) is suddenly given over to a different project (the `async` crate). Furthermore, users cannot trust `foo-bar` to be owned by `foo` because the vast number of grandfathered crates we will have.

Triple colons could work. People might find it confusing, but `foo:::bar` evokes Rust paths without being ambiguous.

We could use `~` which enables Rust code to directly name namespaced packages (as `~` is no longer used in any valid Rust syntax). It looks extremely weird, however.

We could use dots (`foo.bar`). This does evoke some similarity with Rust syntax, however there are ambiguities: `foo.bar` in Rust code could either mean "the field `bar` of local/static `foo`" or it may mean "the crate `foo.bar`".

Note that unquoted dots have semantic meaning in TOML, and allowing for unquoted dots would freeze the list of dependency subfields allowed (to `version`, `git`, `branch`, `features`, etc).


We could reverse the order and use `@`, i.e. `foo/bar` becomes `bar@foo`. This might be a tad confusing, and it's unclear how best to surface this in the source.


## User / org namespaces

Another way to handle namespacing is to rely on usernames and GitHub orgs as namespace roots. This ties `crates.io` strongly to Github -- currently while GitHub is the only login method, there is nothing preventing others from being added.

Furthermore, usernames are not immutable, and that can lead to a whole host of issues.

The primary goal of this RFC is for _project_ ownership, not _org_ ownership, so it doesn't map cleanly anyway.

## Feature Flags

This proposal allows for optional subsystems.  This can be created today with feature flags by adding a dependency as optional and re-exporting it.

Draw backs to feature flags
- Solutions for documenting feature flags are limited
- Feature flags can be cumbersome to work with for users
- A semver breakage in the optional-subsystem crate is a semver breakage in the namespace crate
- The optional-subsystem crate cannot depend on the namespace crate
- There is limited tooling for crate authors to test feature combinations especially in workspaces with feature unification and its slow (re-running all tests even if they aren't relevant)

# Prior art

This proposal is basically the same as https://internals.rust-lang.org/t/pre-rfc-packages-as-namespaces/8628 and https://internals.rust-lang.org/t/pre-rfc-idea-cratespaces-crates-as-namespace-take-2-or-3/11320 .

Namespacing has been discussed in https://internals.rust-lang.org/t/namespacing-on-crates-io/8571 , https://internals.rust-lang.org/t/pre-rfc-domains-as-namespaces/8688, https://internals.rust-lang.org/t/pre-rfc-user-namespaces-on-crates-io/12851 , https://internals.rust-lang.org/t/pre-rfc-hyper-minimalist-namespaces-on-crates-io/13041 , https://internals.rust-lang.org/t/blog-post-no-namespaces-in-rust-is-a-feature/13040/4 , https://internals.rust-lang.org/t/crates-io-package-policies/1041/37, https://internals.rust-lang.org/t/crates-io-squatting/8031, and many others.

Python has a similar coupling of top-level namespaces and modules with the filesystem.  Users coming from other packaging systems, like Perl, wanted to be able to split up a package under a common namespace.  A hook to support this was added in Python 2.3 (see [PEP 402](https://peps.python.org/pep-0402/#the-problem)).  In [PEP 420](https://peps.python.org/pep-0420/) they formalized a convention for packages to opt-in to sharing a namespace.  Differences:
- Python does not have a coupling between package names and top-level namespaces so there is no need for extending the package name format or ability to extend their registry for permissions support.
- In Python, nothing can be in the namespace package while this RFC allows the namespace package to also provide an API.

# Unresolved questions

Deferred to tracking issue to be resolved pre-stabilization:
- How exactly should the Cargo.toml `lib.name` key work in this world, and how does that integrate with `--extern` and `-L` and sysroots?
- Should we allow renames like `"foo::bar" = { package = "foo_bar", version = "1.0" }` in Cargo.toml?
- How precisely should this be represented in the index trie?
- How we should name the `.crate` file / download URL

Third-parties, like Linux distributions, will need to decide how to encode
cargo package names in their distribution package names according to their
individual rules.
Compared to existing ecosystems with namespaces that they package, the only new
wrinkle is that there can be 0-1 namespace levels.

# Future possibilities

We can allow multiple layers of nesting if people want it.

# FAQ

## What if I don't want to publish my crate under a namespace?

You don't have to, namespaces are completely optional when publishing.

## Does this stop people from squatting on `coolcratename`?

No, this proposal does not intend to address the general problem of squatting (See [crates.io's policy](https://crates.io/policies#squatting), a lot of this has been discussed many times before). Instead, it allows people who own an existing crate to publish sub-crates under the same namespace. In other words, if you own `coolcratename`, it stops people from squatting `coolcratename::derive`.
