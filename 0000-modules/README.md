- Feature Name: modules
- Start Date: 2017-08-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This is a redesign of the Rust module system, intended to improve its
ergonomics, learnability, and locality of reasoning. Because this is a
relatively large proposal, it has been broken into multiple text files.

# Table of Contents

* **[Motivation][motivation]** - why we propose to make this change
* **[Overview][overview]**  - a high level overview of what it will be like to
use the new system as a whole
* **Detailed design** - the details of the proposal, broken into multiple
sections:
    * **[Loading Files][loading-files]**
    * **[The `local` keyword][local]**
    * **[Use, mod, and export][use-mod-export]**
* **[Migration][migration]** - this proposal involves migrating from one system
to another, and this section describes it in detail.

Each of the detailed design subsections contains its own description of
drawbacks and alternatives.

[motivation]: motivation.md
[overview]: overview.md
[loading-files]: detailed-design/loading-files.md
[local]: detailed-design/local.md
[use-mod-export]: detailed-design/use-mod-export.md
[migration]: migration.md
