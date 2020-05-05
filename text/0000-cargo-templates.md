- Feature Name: `cargo_new_project_from_template`
- Start Date: 2020-05-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/cargo#3506](https://github.com/rust-lang/cargo/issues/3506)

# Summary

Add the ability to reference a template when running `cargo new` or `cargo init`

# Motivation

> We're doing this to make it "easy for frameworks to ship starter templates to users to give users the dependencies, stubs etc that they need" - @withoutboats 

[Source](https://internals.rust-lang.org/t/pre-rfc-cargo-new-templates/12060/10?u=jsjoeio.)

The expected outcome is to:

* support templates that live on the crates.io registry inside a directory called `templates` at the root of a crate

# Guide-level explanation

Let's create a new project using Cargo and a template from the registry. For example, we'll try this using the `hello-world` template in the `rocket` crate. To use this run the following:

```
cargo new my-rocket-app --template rocket/hello-world
```

Note: this option is also supported with the `init` command:

```
cargo init my-rocket-app --template rocket/hello-world
```

This will create a new cargo project called `my-rocket-app` using the `hello-world` template.

To view other `rocket` templates, see the `templates` directory in the root of the `rocket` repository on GitHub.

# Reference-level explanation

`cargo new` and `cargo init` will work the same way as usual. The added feature will be the ability to use a template that exists within a package on the [crates.io](http://crates.io) registry.

Templates can be created by any package as long as they follow these conventions:

* the package is on the [crates.io](http://crates.io) registry
* the template lives inside the `templates` directory, which lives at the root of the crate

Example:

```
my-crate
- templates
    - hello-world
    - basic-app
    - advanced-app 
```

If this were a real crate, the following would be valid ways to initialize a project with the `my-crate` templates:

* `cargo new my-new-app --template my-crate/hello-world `
* `cargo new my-new-app --template my-crate/basic-app`
* `cargo new my-new-app --template  my-crate/advanced-app`

Note: the same examples could be used with `cargo init` as well.

To summarize, the following is the syntax for using a template

* `cargo new <project-name> --template <crate-name>/<template-name>`
* `cargo init <project-name> --template <crate-name>/<template-name>`

# Drawbacks

## Unmaintained templates -> negative effect on the community

With `cargo` supporting templates, this may lead to a large influx of community-created templates. Over time, this will grow and many will go unmaintained. When community members try to use these templates, it may lead to issues that could have a hurt on the community.

# Rationale and alternatives

This is the best design because it keeps the MVP relatively small in scope. It does not worry about:

* templates that exist outside the [crates.io](http://crates.io) registry
* templates that exist locally

The purpose behind this is keeping it small and focusing on creating first-level template support.

# Prior art

## Does this exist in other programming languages?

Looking at other programming communities, similar implementations exist.

### JavaScript

In the JS community, two similar concepts have been implemented.

#### `npx create-<initializer>`

The npm CLI also added a convention that allows you to [initialize a project](https://docs.npmjs.com/cli/init) for any package following the convention: `create-<initializer>` which can then be used to generate a new project with `npx` (similar to `npm init` ). This has been implemented by popular projects such as [React](https://github.com/facebook/create-react-app) and [Next](https://nextjs.org/blog/create-next-app).

#### `gatsby new`

Another JavaScript framework called Gatsby.js has a command that feels most similar to `cargo new` , which is `gatsby new` . Here is how it works:

* `gatsby new` with no flags/arguments runs an interactive shell asking for the name of your project and which starter/template to use
* `gatsby new [<site-name> [<starter-URL>]]` can also start a new project using a URL to a starter (template) from GitHub. Examplegatsby new my-awesome-blog-site https://github.com/gatsbyjs/gatsby-starter-blog

#### `npx degit sveltejs/template my-svelte-project`

The [Svelte](https://svelte.dev/) framework follows a similar pattern to Gatsby. It uses a project scaffolding tool called [degit](<https://github.com/Rich-Harris/degit>). It follows the pattern `npx degit <user/repo> <name-of-project>` . You can read more about it in the [README](<https://github.com/Rich-Harris/degit>) .

#### Yeoman

 [Yeoman](https://yeoman.io/) is a "project template generator that has been in development for years, and has grown to have over 5000 project templates." - @kornel 

We don't need to have all the same features as Yeoman, but there may be lessons to learn form it. 

To scaffold a new project:
```shell
yo webapp
```

To find out the options for the project, pass the `--help` flag:
```shell
yo webapp --help
```

### Python

#### `paster`

It appears there is a `pip` package created by the community called `pastescript` , which, “[creates] file layouts for packages.” You can use it by running:

```
paster create --template=basic_package MyPackage
```

### ReasonML

#### `bsb -init my-new-project -theme basic-reason`

Using [BuckleScript](https://bucklescript.github.io/en/), you can [initialize a basic Reason project](https://reasonml.org/docs/manual/latest/installation#new-project). This includes the following: `README.md` `bsconfig.json` `node_modules` `package.json` `src` and is similar to `cargo new` .

### Has the community suggested this before? Are there crates that solve this problem already?

A [similar RFC](https://internals.rust-lang.org/t/pre-rfc-cargo-templates/5056) for cargo templates was written back in April 2017 and shared on the internals forum. It seemed like there were a lot of discussions, but no consensus reached.

The community has also created two crates that solve similar problems:

* [cargo-generate](<https://crates.io/crates/cargo-generate>): a developer tool to help you get up and running quickly with a new Rust project by leveraging a pre-existing git repository as a template
* [kickstart](https://github.com/Keats/kickstart): created by @Keats described as "A scaffolding tool to get new projects up and running quickly"
* [cookiecutter](https://github.com/cookiecutter/cookiecutter) : not a Rust project, but "A command-line utility that creates projects from cookiecutters (project templates)"

In addition, there was a lot of discussion both from the Cargo team and the community on [this issue](https://github.com/rust-lang/cargo/issues/3506). It has been decided that there [still remains disagreement](https://github.com/rust-lang/cargo/pull/8029#issuecomment-604756599) among both the Cargo team and the community on how to solve this, hence why this RFC seems to be the logical next step.

# What lessons can we learn from what other communities have done here?

There are pros and cons to having templates or some type of template ecosystem. I think the biggest question is **who will maintain them?** It may not be directly related, but it's an important point to consider should templates be added to `cargo new` .

If we return to our examples from the JavaScript industry, there are two that stick out:

## Community maintained templates

In the Gatsby.js community, there are +[300 starters](https://www.gatsbyjs.org/starters/?v=2). Only a select few are maintained by the Gatsby.js core team. The rest are added by community members.

### Pros

* Community members can contribute
* There is a wilder selection of options

### Cons

* Members can abandon their starters which can negatively impact the community

## Core team maintained templates

Revisiting the `create-react-app` and the `create-next-app` templates, those are maintained by core team members (i.e. a select group of individuals). There are some variations to the templates (i.e. regular vs. TypeScript).

### Pros

* Higher-quality
* Creates a “standard”
* Reliable

### Cons

* Requires dedicated maintainers
* Less community involvement (beyond direct contributions)

# Unresolved questions

Some questions that may require further discussion depending on how this RFC goes.

Questions that fall in the scope of this RFC:

## What is a template ?

> A template is a directory that lives inside `crate/templates/`. It must include the following
> 
> The `Cargo.toml` of a template must include the following text (or some other placeholder) literally at some point, which will be replaced wholesale by the template engine. - @CAD97

```
[package]
name = ""
authors = []
```

## Will it add the project name, edition, and authors to the `Cargo.toml`?

> I would expect it would. Ideally, the template would have the power to add/update arbitrary parts of the crate (or workspace) that is being constructed, which includes the Cargo.toml file." - @ckaran 

## Should it be able to compile immediately?

> Nice to have, but not necessary. I can see several use cases where having it generate `compile_error!()` within the code (with a nice error message) could be useful. - @ckaran 

## Should it specify version control?
  
> There are *many* good version control systems out there, tying your template engine to just one means that you've locked all the others out. - @ckaran

Therefore, we should **not** have version control system(vcs) be a feature/requirement for templates. They should be vcs agnostic.

Eventually, `--templates` could support the `--vcs` flag with [`--templates`](https://doc.rust-lang.org/cargo/commands/cargo-init.html). That way, the user could specific which version control system they want to use.  

## What command line options can be used with templates? 

> The only two command line options that should be absolutely required are `--version` and `--help` . The engine should follow the semver spec, which will guide what people have to look out for when the engine changes (e.g., `x.y.z` -> `x.y.(z + n)` changes don't require user analysis, `x.y` -> `x.(y + n)` might, etc.). If you see that the version of the template engine has changed, then `--help` can help you figure out what the changes were. Just keep a good change log around so that users that have used the engine for a while can quickly come up to date on new/modified features without having to dig through piles of stuff they already know about ([conventional commits](https://www.conventionalcommits.org) + tools like [jilu ](https://crates.io/crates/jilu) can help with this). - @ckaran

To summarize, it should support the `--version` and the `--help` options. Possibly the `--vcs`, but that can be a lower priority depending on the scope. 

## How will templates stay up to date with the crates that they're in? (i.e. new versions, which affect dependencies)

> Make templates specify the versions of the crates that they work with, but only allow them to use comparison requirements (e.g. `foo = {version = ">= 1.2, < 1.5"}` ). When the crate is updated, the template authors either need to keep up, or end users will know to avoid those templates. Ideally, this would be paired with better search facilities on [crates.io](http://crates.io) that would allow you to filter templates based on the ranges that they support. - @ckaran

To summarize,
* make templates specify the versions of the crates that they work with
* allow them to use comparison requirements
* users can use this information to determine whether or not they should use/avoid those templates

Lower priority and possibly out of the scope of this MVP:
* "Ideally, this would be paired with better search facilities on [crates.io](http://crates.io) that would allow you to filter templates based on the ranges that they support"

## Where do the docs need to be updated in the Rust Lang book or official docs?

I think we'll update [The Cargo Book](https://doc.rust-lang.org/cargo/) for now. Open to other suggestions too.

## What security measures need to be accounted for when Cargo clones from third-party templates? (i.e. what if a template contains a malicious file?)

> If a template only permits textual substitution, then there is very little that a malicious template author can do (disregarding [billion laughs](https://en.wikipedia.org/wiki/Billion_laughs_attack)); before you compile a template, you *should* look at the code that was generated. - @ckaran

> My suggestion is that you develop a strong security model, one that encompasses what you're trying to protect as completely as possible (e.g., prevent the template engine from hogging the CPU, overwriting all files, arbitrary communications with the network, whatever). Once you really know what you want, you'll have a better answer to what a template is. - @ckaran

## How do people in the community discover templates? (through the crates themselves?)

> Once again, these are two sides of the same coin. If there is a really good way to discover all and only the templates that are for a given project on [crates.io](http://crates.io), then you can keep the docs with the templates themselves. Otherwise, the library authors might mention a few good template crates, and let people search for others on their own. - @ckaran 

Questions that could be answered after the implementation:

* What guidelines should the Cargo team provide for creating templates?
* Will there be any “official” templates?

Related issues that are considered out of scope for this RFC:

* using local templates
* using templates that don't live in crates on the [crates.io](http://crates.io) registry
* extending this to a "full" templating engine

# Future possibilities

Nothing at the moment beyond the questions I noted for after the implementation.

# Thank you!

This RFC went through two Pre-RFC, which can be viewed here:
- [v1](https://internals.rust-lang.org/t/pre-rfc-cargo-new-templates/12060/10)
- [v2](https://internals.rust-lang.org/t/pre-rfc-cargo-new-templates-v2/12089)

Thanks to all that participated. 
