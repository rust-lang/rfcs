- Feature Name: annotated_diagnostic_json_output
- Start Date: 2017-01-18
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow the compiler to annotate `DiagnosticBuilder` messages with metadata
related to the contents being displayed, to allow external tooling more control
over the displayed format, as well as allowing deeper integration with the type
system.

# Motivation
[motivation]: #motivation

The compiler currently allows diagnostic messages to have
[highlighted parts](https://github.com/rust-lang/rust/pull/38955):

<img src="https://cloud.githubusercontent.com/assets/1606434/21871227/93a84198-d815-11e6-88b1-0ede3c7e28ef.png">

These highlights are purely presentational, with no semantic meaning. By
exposing these annotations, external tools can make use of it.

By changing it to have semantic meaning, these tools can take advantage of this
extra information for deeper integration, like allowing navigation to type
definition for portions of text that have been annotated as `kind` `type`, as
well as proper highlighting of sections of the text.

# Detailed design
[design]: #detailed-design

The [`JsonEmitter`](https://github.com/rust-lang/rust/tree/master/src/libsyntax/json.rs)
Diagnostic Emitter shall be expanded to include a new field
`message_annotations` in its output. This field would contain a list of objects
with metadata about portions of the diagnostic's `message` field, in a similar
way as `Span`s reffer to portions of a source code file.

The metadata shall include:

* the `kind` of annotation,
* the region of the `message` that it spans
* the text content, _(Maybe?, this information can be gotten from the `message`
  itself. The biggest benefits I see are that it makes it easier for tool
  makers to be lazy and that we could include different text from what is
  actually in the message, but both of these points may prove troublesome.)_
* an `important` boolean flag to allow us to communicate wether this annotation
  should be treated in such a way as to increase its visibility, i.e., bolded
  or highlighted in the presentation.

The kind of annotations should be:

* `type`
* `highlight` _(alternative name: `bold`)_
* `statement` _(alternative name: `code`)_
* `file` _(alternative name: `path`)_
* `url`
* ...


The existing json output of the compiler
(`rustc -Z unstable-options --error-format=json`) for messages only includes
the formatted text of the diagnostic message:

```json
{
    "message": "expected type `usize`\n   found type `&'static str`",
    "code": null,
    "level": "note",
    "spans": [],
    "children": [],
    "rendered": null
}
```

After the proposed change the output would include new field containing
semantic information for the content in the `message`:

```json
{
  "message": "expected type `usize`\n   found type `&'static str`",
  "message_annotations": [
    {
      "start": 15,
      "end": 20,
      "kind": "type",
      "content": "usize",
      "important": true
    },
    {
      "start": 37,
      "end": 49,
      "kind":"type",
      "content": "&'static str"
      "highlighted": true
    },
  ],
  "code": null,
  "level": "note",
  "spans": [],
  "children": [],
  "rendered": null
}
```

On `rustc`, these same annotations presented to tools through the json
`error-format` shall be used for colorizing and highlighting the output when
possible.


# How We Teach This
[how-we-teach-this]: #how-we-teach-this

> What names and terminology work best for these concepts and why?

*Annotation*: A message annotation is equivalent to a `Span`, only instead of
code they carry information contained in a diagnostic message.

> How is this idea best presented—as a continuation of existing Rust patterns,
or as a wholly new one?

This is a continuation of the work started to [provide highlights for
diagnostic messages](https://github.com/rust-lang/rust/pull/38955).

> Would the acceptance of this proposal change how Rust is taught to new users
at any level?

Not for end users, only for tool makers integrating error messages, and even
then the change is only additive.

> How should this feature be introduced and taught to existing Rust users?



> What additions or changes to the Rust Reference, _The Rust Programming
Language_, and/or _Rust by Example_ does it entail?

Documenting the new output _(I don't think there's any official documentation
about this flag anywhere outside of internals and the bug tracker)_.

# Drawbacks
[drawbacks]: #drawbacks

Expanding the amount of information being outputted by the compiler could get
unwildy for larger projects.

Introducing this can be seen by contributors as an encouragement to add tons of
annotations to diagnostic messages, while restraint will probably be needed to
avoid unnecessary extra information being added.

# Alternatives
[alternatives]: #alternatives

The current alternative that tools have as an option is trying to parse common
diagnostic messages in search for interesting parts, like in the case of
"expected"/"found" messages. This is not ideal, as it can lead to brittle
interactions that can break often as the diagnostic messages have their wording
and presentation refined.

A variation on this proposal s to modify the `message` field itself to contain
the annotations:

```json
{
  "message": [
    {"content": "expected type `"},
    {
      "start": 15,
      "end": 20,
      "kind": "type",
      "content": "usize",
      "highlighted": true
    },
    {"content": "`\n   found type `"},
    {
      "start": 37,
      "end": 49,
      "kind":"type",
      "content": "&'static str",
      "highlighted": true
    },
    {"content": "`"}
  ],
  "code":null,
  "level":"note",
  "spans":[],
  "children":[],
  "rendered":null
}
```

I find this output to be less ergonomic, as tools that might not want to bother
with the new metadata being exposed to explicitly have to deal with it.

# Unresolved questions
[unresolved]: #unresolved-questions

> What parts of the design are still TBD?
