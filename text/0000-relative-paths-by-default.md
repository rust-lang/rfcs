- Start Date: 2015-01-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

All paths should be relative by default. The current design requires the
`self::` prefix to be relative in `use` statements.

Absolute paths should require a new `crate::` prefix. The current design has
absolute paths by default in `use` statements.

# Motivation

Currently, `use` statements have absolute paths by default, but other paths
are relative paths by default. This causes confusion for beginners due to
the inconsistencies.

Additionally, this encourages misusing absolute paths when relative paths
are more logical (e.g. when importing from a sibling module). This makes
refactoring such as renaming modules harder. The cases that require
absolute paths should be rarer than the ones that require relative paths,
given a reasonable module structure.

Note that this creates a nice analogy with the file system, with `::`
instead of `/`.

# Detailed design

The original grammar for paths (from the reference) is:

```
expr_path : [ "::" ] ident [ "::" expr_path_tail ] + ;
```

The new grammar (which includes some clean up) is:

```
expr_path : [ "crate::" | "super::" + ] ? ident [ "::" expr_path_tail ] + ;
```

The behavior is simple:

- The `crate::` prefix makes the path an absolute path (i.e. starts from
  the crate root).
- The `super::` prefix makes the path start from the nth ancestor module,
  where n is the number of `super::` prefixes.
- Having no prefix makes the path a relative path (i.e. starts from the
  current module).

Note that `use foo::bar` should behave as if the contents of `foo::bar`
(`self::foo::bar` in the current syntax) is copied into the current module
(`self`). The analogy to the file system is creating a soft link named `bar`
pointing to `foo/bar`. As a consequence, `use foo::bar` followed by
`use bar::baz` is valid.

# Drawbacks

This breaks *a lot* of code.

# Alternatives

- Use the existing `::` prefix instead of the `crate::` prefix. However, I
think that this might cause some confusion for beginners.
- The status quo has problems as mentioned in the Motivation section.

# Unresolved questions

Unknown
