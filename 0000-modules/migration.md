# Migration

The changes proposed by this RFC are significant breaking changes to the
language. To complete them entirely we will need to pass a checkpoint/epoch
boundary as described in [RFC #2052][checkpoints]. However, we will begin with
a multi-phase deprecation process to ease the transition.

We will also introduce a tool that will not only automatically move a crate
onto a new system, but also *improve clarity* of the user's code by
intentionally drawing the distinction between the `pub` and `local`
visibilities that today does not firmly exist.

## The automatic upgrade path

Concurrent with the first release under this RFC, we will also release a tool
that eliminates all deprecated forms from the user's codebase and moves them
into the preview checkpoint. It will also automatically fix any other errors
introduced by the checkpoint transition.

This tool will be distributed in such a way that an upgrade to the first
version that provides it puts it on the user's system. If users are willing to
use this tool, all they will need to do is run it and they will get code
without any deprecation errors. This is the easiest and the recommended way
to upgrade to the new checkpoint and apply these module changes.

This tool will also correctly distinguish between items which need to be marked
`pub` and items which should be marked `local`, and mark those items
appropriately.

## The manual upgrade path

For users who wish to upgrade manually, the following new features will be
introduced on the current (2015) checkpoint:

* The `local` and `export` features will be provided as contextual keywords.
* The `--modules` API for rustc will be introduced.
* cargo will accept the `load-modules` flag, but it will be set to false by
default.

The following deprecations will be introduced (possibly staged over multiple
releases):

* `use` with a visibility
* Imports taking advantage of `use`'s item nature
* Restriction modifiers on `pub` (as opposed to `local`)
* `mod` statements without a visibility
* Absolute local paths without the `local::` prefix

In the new epoch, all of these will become hard errors and the `load-modules`
flag in cargo will be set to `true` by default, turning on the module loading
system.
