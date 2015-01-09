- Start Date: 2015-01-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

All paths should be relative by default. The current design requires the
`self::` prefix to be relative in `use` statements.

Global paths should require a new `crate::` prefix. The current design has
global paths by default in `use` statements.

# Motivation

Currently, `use` statements have global paths by default, but other paths
are relative paths by default. This causes confusion for beginners due to
the inconsistencies.

Additionally, this encourages misusing global paths when relative paths
are more logical (e.g. when importing from a sibling module). This makes
refactoring such as renaming modules harder. The cases that require
global paths should be rarer than the ones that require local paths,
given a reasonable module structure.

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

- The `crate::` prefix makes the path a global path (i.e. starts from
  the crate root).
- The `super::` prefix makes the path start from the nth ancestor module,
  where n is the number of `super::` prefixes.
- Having no prefix makes the path a local path (i.e. starts from the
  current module).

# Drawbacks

This breaks *a lot* of code.

# Alternatives

- Use the existing `::` prefix instead of the `crate::` prefix. However, I
think that this might cause some confusion for beginners.
- The status quo has problems as mentioned in the Motivation section.

# Unresolved questions

Unknown
