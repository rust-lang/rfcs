# RFC policy - the compiler

Compiler RFCs will be managed by the compiler sub-team, and tagged `T-compiler`.
The compiler sub-team will do an initial triage of new PRs within a week of
submission. The result of triage will either be that the PR is assigned to a
member of the sub-team for shepherding, the PR is closed because the sub-team
believe it should be done without an RFC, or closed because the sub-team feel it
should clearly not be done and further discussion is not necessary. We'll follow
the standard procedure for shepherding, final comment period, etc.

Most compiler decisions that go beyond the scope of a simple PR are done using [MCP]s,
not RFCs. It is therefore likely that you should file an MCP instead of an RFC for your problem.

## Changes which need an RFC

* Significant user-facing changes to the compiler with a complex design space,
  especially if they involve other teams as well (for example, [path sanitization]).
* Any other change which causes significant backwards incompatible changes to stable
  behaviour of the compiler, language, or libraries

## Changes which don't need an RFC

* Bug fixes, improved error messages, etc.
* Minor refactoring/tidying up
* Large internal refactorings or redesigns of the compiler (needs an [MCP])
* Implementing language features which have an accepted RFC.
* New lints (these fall under the lang team). Lints are best first tried out in clippy
  and then uplifted later.
* Changing the API presented to syntax extensions or other compiler plugins in
  non-trivial ways
* Adding, removing, or changing a stable compiler flag
  (needs an FCP somewhere, like on an [MCP] or just on a PR)
* Adding unstable API for tools (note that all compiler API is currently unstable)
* Adding, removing, or changing an unstable compiler flag (if the compiler flag
  is widely used there should be at least some discussion on discuss, or an RFC
  in some cases)

If in doubt it is probably best to just announce the change you want to make to
the compiler subteam on [Zulip], and see if anyone feels it needs an RFC.

[MCP]: https://github.com/rust-lang/compiler-team/issues
[path sanitization]: https://github.com/rust-lang/rfcs/pull/3127
[Zulip]: https://rust-lang.zulipchat.com/#narrow/stream/131828-t-compiler

