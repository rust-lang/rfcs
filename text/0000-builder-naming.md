- Start Date: 2015-04-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Provide guidelines for builder types in the standard library.

# Motivation

We use builder pattern in a few places in the standard library, but they are not consistent. We
have `thread::Builder`, `process::Command` and there’s `fs::OpenOptions`. They are somewhat ad-hoc
and inconsistency makes forming the intuition about them hard.

A common example is people not knowing about `OpenOptions` builder and being confused by its name
and lack of discoverability.

# Detailed design

## Guidelines

### Naming

These guidelines suggest a few naming schemes, depending on situation at hand:

* If the module, which exports the builder, is expected to only export a single builder, even in
   the future, and the module itself has a descriptive name, calling the builder “Builder” is
   recommended. For example:

   * `thread::Builder` builds and spawns threads;
   * `process::Builder` builds and spawns processes;

   but *not* `fs::Builder`, which opens files;
* If the builder produces one type of objects, use the name of buildee suffixed with “Builder”.
    This scheme has an advantage of being searchable and self-documenting. For example:
    `FileBuilder` is a builder and produces objects of type `File`;
* If there is no clear buildee (most likely because it contains multiple finalizers), a noun
    describing all the buildable objects suffixed with “Builder” may be used. For example:
    `ThreadBuilder`, even though it produces `JoinHandle`s and `JoinGuard`s rather than `Thread`s;
* Finally, if the majority (community) decides there exists a superior name for the builder, but it
    does not match any of the guidelines above, generously point out the existence of the builder
    in the documentation of buildee and module, if appropriate.

### Methods

Option setters – methods which represent options that can be set on the builder – should receive
the builder object as `&mut self` rather than `self` unless there is a good reason to do otherwise.

Finalizers – methods which produce the built object – should receive self via reference, unless
there is a good reason to do otherwise.

In order for method listing in documentation to be predictable, methods in the builder’s `impl`
block should be declared following this order:

* `new` – a method to create the builder object comes first in the `impl` block;
* followed by one or more option setters;
* followed by one or more finalizers.

This ordering stems from conventional use of the builder pattern: create a builder, set the
options and call a finalizer.

#### Method naming

Option setters should be nouns or adjectives. Avoid `is_*` prefix and nouns derived from
adjectives. For example:

* `name`;
* `stack_size`;
* `read` or `readable` (rather than `is_readable` or `readability`);
* `write` or `writable` (rather than `is_writable` or `writability`).

Finalizers should be verbs. For example:

* `spawn`;
* `create`;
* `open`.

## Proposed changes to the standard library

### `thread::Builder`

`thread::Builder` is adjusted to have the following interface:

```rust
pub struct Builder { … }
impl Builder {
    fn new() → Builder;
    fn name(&mut self, name: String) → &mut Builder;
    fn stack_size(&mut self, size: usize) → &mut Builder;
    fn spawn<F>(&mut self, f: F) → Result<JoinHandle> where F: FnOnce(), F: Send + 'static;
    fn scoped<'a, T, F>(&mut self, f: F) → Result<JoinGuard<'a, T>>
        where T: Send + 'a, F: FnOnce() → T, F: Send + 'a
}
```

and `impl Clone for Builder` is provided.

In summary, all methods are changed to take and return `Builder` by mutable reference rather than
by value.  Strictly speaking this is a breaking change, but most users of the `thread::Builder`
should not encounter any breakage.

### `process::Command`

`impl Clone for Command` is provided.

### `fs::OpenOptions`

`OpenOptions` is renamed to `FileBuilder`. `OpenOptions` shall stay as a deprecated alias for
`FileBuilder` for a period.

# Drawbacks

Breaking change after 1.0-beta.

# Alternatives

* Only apply these guidelines to builders introduced after 1.0-beta;
* Construct builders via a method implemented on the buildees. The API looks like this:

        let file = File::builder()
            .write(true)
            .truncate(false)
            .open(path);

    This allows users of the API never reference the builder type. On the other hand, it is only
    suitable for a subset of builders. For example it doesn’t work for `thread::Builder`, because
    the finalizers return guards rather than an instance of `Thread`;

    Proposed by [@sfackler][sfackler].
* Rename `process::Command` to `process::Builder`.

[sfackler]: https://github.com/rust-lang/rfcs/pull/1044#discussion_r28082235

# Unresolved questions

None known to the author.
