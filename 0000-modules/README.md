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
    * **[Paths][paths]**
    * **[Visibility and Re-exporting][visibility]**
    * **[Migration][migration]** - this proposal involves migrating from one
    system to another, and this section describes it in detail.

Each of the detailed design subsections contains its own description of
drawbacks and alternatives.

[motivation]: 0000-modules/motivation.md
[overview]: 0000-modules/overview.md
[loading-files]: 000-modules/detailed-design/loading-files.md
[paths]: 000-modules/detailed-design/paths.md
[visibility]: 000-modules/detailed-design/visibility-and-reexport.md
[migration]: 000-modules/detailed-design/migration.md
