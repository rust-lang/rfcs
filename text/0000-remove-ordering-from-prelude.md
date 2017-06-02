- Start Date: 2014-12-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Currently, `Ordering` and its variants are exported in the prelude. We should not
do that any more.

# Motivation

In the past, enums were not namespaced. To use each variant, they must also be
`use`d.

`Ordering` and its variants were in the prelude before my time, and so I'm not 100% sure
why they're in the prelude. I would imagine, like most things, it comes down to
convenience: importing four things to `match` on `==` is kind of a hassle.

The prelude adds names to every Rust program, and so the standard for being in
the prelude is currently 'clearly useful in every Rust program.' However,
`match`ing on an `Ordering` is not particularly common:

```bash
$ git grep --name-only Greater src 
src/doc/guide.md
src/doc/reference.md
src/etc/unicode.py
src/etc/vim/syntax/rust.vim
src/liballoc/rc.rs
src/libcollections/bit.rs
src/libcollections/btree/node.rs
src/libcollections/btree/set.rs
src/libcollections/slice.rs
src/libcollections/str.rs
src/libcollections/tree/map.rs
src/libcollections/tree/set.rs
src/libcollections/trie/set.rs
src/libcore/cmp.rs
src/libcore/iter.rs
src/libcore/prelude.rs
src/libcore/ptr.rs
src/libcore/slice.rs
src/libcore/str.rs
src/libcoretest/cmp.rs
src/libcoretest/tuple.rs
src/libregex/vm.rs
src/librustc/middle/typeck/infer/region_inference/mod.rs
src/librustc_trans/trans/_match.rs
src/librustdoc/html/render.rs
src/libstd/prelude.rs
src/libtest/stats.rs
src/libunicode/normalize.rs
src/libunicode/tables.rs
src/test/bench/shootout-k-nucleotide-pipes.rs
src/test/run-pass/bool.rs
src/test/run-pass/deriving-self-lifetime-totalord-totaleq.rs
```

Note the first result there, the Guide. A full quarter of the instances of `Greater` are
used in the Guide, because `Ordering` is used to teach about enums. Due to shadowing,
this leads to https://github.com/rust-lang/rust/issues/17967, which trips up many newbies.
See [this comment](https://github.com/rust-lang/rust/issues/17967#issuecomment-61572399)
from @bstrie, especially. Removing `Ordering` from the prelude fixes this issue
nicely.

# Detailed design

Remove these four lines:

* https://github.com/rust-lang/rust/blob/master/src/libstd/prelude.rs#L66-L67
* https://github.com/rust-lang/rust/blob/master/src/libcore/prelude.rs#L51-L52

And the fix the fallout by adding some `use` statements to the affected files. I would
be willing to do said implementation.

# Drawbacks

This is another `[breaking-change]`. It is easy to fix, though, so I do not consider
this drawback to be significant.

# Alternatives

We could remove the export of just the variants. This would not fix my Guide issue, but
would still be more conformant to our style guidelines.

# Unresolved questions

No technical ones, this is entirely a social change.
