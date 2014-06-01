- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

Summary
=======

Extend the pattern syntax to make it a lot more consistent, advanced, and powerful.

Motivation
==========

The current pattern syntax is inconsistent and has a number of limitations which are not necessary and reduce the expressiveness of the pattern syntax. For example, within a `match` arm, alternation (`|`) and guards are allowed, but not in `let`-statements or function parameter definitions. This aims to resolve these inconsistencies and extend the pattern syntax further to allow such things as implicit equality guards within patterns.

This proposal also allows also creates some symmetry between pattern operators and logical/set operations. For example, `@`, which previously just bound an identifier to a pattern, now acts much like a logical ‘and’ or set intersection operator.

Detailed Design
===============

Change the pattern syntax in the following ways:

* **Allow for parenthesisation of patterns.** Right now this is useless, but with the following proposed changes it would be necessary for complete expressiveness.
* **Change `@`-patterns to allow a pattern on either side.** Right now `@` patterns only allow an identifier on the left and a pattern on the right. This is confusing behaviour for newcomers to the language if they try to do things like `let (a, b) @ tuple = (1, 2);`.

  The patterns would have to be compatible—a pattern like `Some(_) @ None` wouldn’t be considered valid because the two sub-patterns can’t match the same thing. However, a pattern like `1 .. 5 @ 3 .. 8` *would* be valid, and would be equivalent to `3 .. 5`.

  The `@` operator can be considered as a set intersection operator or a boolean `&&` operator.
* **Add alternation (`|`) to patterns properly.** This makes the syntax more consistent with that of match arms, and also allows such expressions as `let Ok(e) | Err(e) = some_fn();` to be valid. This would not only be valid at the ‘top level’ of a pattern: `let Newtype(Ok(e) | Err(e)) = expr;` would be a valid statement.

  This would be the second-lowest-precedence operation within patterns.

  The `|` operator can be considered as a set union operator or a boolean `||` operator.
* **Add pattern guards (`pattern if condition`) to patterns properly.** Once again, this would make the pattern syntax consistent with that of match arms. It could also be useful within `let` bindings: `let ((a, _) if a > 5) | (_, a) = (e, f);` would define `a` as the value of `e` if `e > 5`, and otherwise bind `a` to the value of `f`. Once again, this would not only be valid at the top level of a pattern.
  
  For backward-compatibility reasons, guards would have the lowest precedence out of anything else in a pattern.
* **Treat repeated variables as implicit guards for equality.** This would make the pattern refutable. For example, the following code snippet:

  ```rust
  let triangle = (2f32, 2f32);
  
  let hypot = match triangle {
      (a, b) if a == b => (a * b * 2).sqrt(),
      (a, b) => (a.powi(2) + b.powi(2)).sqrt(),
  };
  ```

  could be written under this proposal as:

  ```rust
  let triangle = (2f32, 2f32);
  
  let hypot = match triangle {
      (a, a) => (a * a * 2).sqrt(),
      (a, b) => (a.powi(2) + b.powi(2)).sqrt(),
  };
  ```
* **Change the syntax of `match`-expressions/-statements to take a normal pattern.** That is, the `match` syntax would be change from:

  ```
  match_expr : "match" expr '{' match_arm * '}' ;
  match_arm : attribute * match_pat "=>" [ expr "," | '{' block '}' ] ;
  match_pat : pat [ '|' pat ] * [ "if" expr ] ? ;
  ```

  to:

  ```
  match_expr : "match" expr '{' match_arm * '}' ;
  match_arm : attribute * pat "=>" [ expr "," | '{' block '}' ] ;
  ```

Drawbacks
=========

Adds a lot of extra features to the pattern syntax that perhaps have little use.

Alternatives
============

* **Do a subset of this proposal.** For example, the `@` syntax could remain how it is right now as it’s a fairly minor and possibly unnecessary change.
* **Keep the pattern syntax as it is.** This has the downside of `match`s having an extension of the normal pattern syntax (adding guards & alternation).

Unresolved questions
====================

* **Is every single part of this proposal necessary?** Changes like adding guards to the pattern syntax are probably nearly useless and are only in this proposal for consistency. There are very few use cases for guards in `let`-statements.
* **Is the syntax ambiguous?** A pattern like `a if condition if condition` would be valid under this proposal, but looks strange and could be hard to parse.
