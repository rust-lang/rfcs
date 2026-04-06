- Feature Name: crates-io-username-identity
- Start Date: 2026-04-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Crates.io Issue: [rust-lang/crates.io#0000](https://github.com/rust-lang/crates.io/issues/0000)

# Summary
[summary]: #summary

Someday, we would like to enable people to log in to crates.io with other services in addition to
GitHub. This RFC is not yet about adding other services for login. It is proposing that crates.io
change to have the concept of a "crates.io username" separate and possibly different from users'
GitHub usernames. Crates.io needs this change to make authenticating with different services
possible while minimizing confusion.

> 🚨 After this RFC is accepted and implemented, you will still only be able to log in to crates.io
> via GitHub. This is a prerequisite of the eventual goal to add other methods of logging in. 🚨

The biggest changes to crates.io as a result of this RFC will be:

- There will be a crates.io username that may not always match the associated GitHub username
- Crates.io will no longer automatically update your crates.io username if you rename your GitHub
  account

# Motivation
[motivation]: #motivation

Crates.io's code currently has a one-to-one mapping between crates.io accounts and GitHub accounts.
The URL `https://crates.io/users/some_username` displays the crates owned by the user with the
GitHub account `some_username`, and running `cargo owner --add some_username` adds `some_username`
as an owner of the current crate. Owners of a crate appear in the sidebar. Crate ownership conveys
trust.

Eventually (after future RFCs and additional work after this RFC), we'd like to add the ability to
create crates.io accounts by logging in via OAuth with accounts from services other than GitHub, as
well as associating OAuth accounts from multiple services to one crates.io account.

The same username on GitHub is not guaranteed to belong to the same person on other services, and
one person's usernames across different services are not guaranteed to be the same. When we add
more services, crates.io's codebase needs to be able to handle these situations and clearly convey
crates.io user identities to minimize the possibility of confusion or deliberate impersonation.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section will address changes to what users will experience on the crates.io website and via
the `cargo owner` CLI.

Today, crates.io usernames always match the GitHub username of the account used to log in to
crates.io (with exceptions for renamed or deleted GitHub accounts that will be discussed below).

After this change, there will be the concept of a crates.io username that may or may not match the
GitHub username of the associated account. When there are multiple ways of logging in in the
future, the crates.io username may or may not match the usernames on the other services. All
existing active (that is, not deleted or renamed) accounts will have their crates.io username set
to their current username, their GitHub username.

When you visit your account settings page, you will be able to edit your username to anything that
isn't already claimed as a crates.io username. Thus, crates.io usernames will become
first-come-first-served as crate names are today. Crates.io admins will not change an account's
username without the consent of the current username holder, except in cases such as Code of
Conduct or usage policy violations like impersonation (see [Unresolved
Questions](#unresolved-questions) about username squatting).

When you visit a user's page at `https://crates.io/users/example_username` or see a user account
listed as an owner of a crate in the crate's sidebar and the account's crates.io username differs
from the GitHub username associated with the account, you will see a warning icon similar to ⚠️ and
text that says something like "username does not match GitHub username". Given that the common
case, and what people are used to being able to know, will be that the GitHub and crates.io
usernames will match, this will make it obvious in cases where that assumption does not hold. We
may decide after some transition period (say, 1-2 years) that the username mismatch warning is no
longer needed (especially once crates.io supports OAuth services other than GitHub).

If you run `cargo owner --add example_username` and the account's crates.io username differs from
the GitHub username associated with the account, the command will error with a message similar to:

```console
$ cargo owner --add example_username
error: username `example_username` is possibly ambiguous

Caused by:
  The crates.io account `example_username` is associated with:

  - https://github.com/something_else
  - [any other accounts once we have that ability]

  To confirm this is the account you want to add, please run one of the following:

  $ cargo owner --add cratesio:example_username
  $ cargo owner --add github:something_else

  If this is not the account you want to add, verify the crates.io username of the account you want.
```

Returning an error and requesting the user re-run a command with a disambiguation prefix to confirm
is the easiest way to maintain compatibility with existing versions of Cargo. With some additional
work on Cargo, newer versions could be made that only require a `y` or `n` confirmation; see the
"Prior Art" section on Keybase for one possibility.

After this RFC is implemented, if you create an account on crates.io with an OAuth account (GitHub
or otherwise), whether or not the associated OAuth account's username is currently claimed on
crates.io, you will be asked to register your crates.io account by choosing a username that hasn't
yet been taken on crates.io. The crates.io username field will be prefilled with the associated
OAuth account's username and an indication of whether that username is available on crates.io or
not.

## Renamed and deleted GitHub accounts

GitHub allows users to change their username (but keep the same GitHub ID number so that crates.io
can know it's the same account) or delete their account, which makes the username available for
someone else to claim (with a different GitHub ID number than was previously associated with it).

Crates.io currently does not proactively check GitHub for account status. If someone takes these
actions:

1. Creates GitHub account with the username "example"
2. Logs in to crates.io with that account so that they have the crates.io username "example"
3. Renames their GitHub account to "something_else"
4. Never logs in to crates.io with that GitHub account again

Their crates.io username will remain "example", until such a point that they decide to log in to
crates.io again. Currently, crates.io will then update their username in our database to match.

This RFC proposes decoupling GitHub account renaming from crates.io username completely, so that
GitHub account renames do NOT automatically become crates.io account renames.

A similar situation occurs when a user deletes their GitHub account. The username "example" will
then be available for someone else to claim on GitHub, but will remain claimed on crates.io.

If a different user does one of the following:

- Creates the GitHub account "example" and logs in to crates.io
- Tries to edit their username to "example"
- Logs in to crates.io with an account on some service other than GitHub with username "example"

At that point, crates.io will:

- See that the crates.io username "example" is taken
- Require the user with the GitHub username "example" to pick a different crates.io username

If the old "example" account had it via their associated GitHub account (and thus didn't have the
mismatch ⚠️ warning discussed above), then a new associated GitHub account logs in with the GitHub
username "example" (and a different GitHub ID), at that point we know the GitHub account "example"
does NOT belong to the crates.io account "example" and the crates.io account "example" should get
the mismatch ⚠️ warning.

If a user manually changes their crates.io username to `best_rust_programmer_ever` (and doesn't
have the matching GitHub account and thus has the warning symbol), and then later someone creates a
GitHub account with the username `best_rust_programmer_ever` and logs in to crates.io, the GitHub
user `best_rust_programmer_ever` will need to choose a different crates.io username. Both crates.io
accounts will have the warning symbol. The latter user may see this as unfair, but this is where
the first-come-first-serve policy should be enforced.

## Crates.io username requirements

Crates.io usernames will largely use the same rules that GitHub usernames use today. All existing
crates.io accounts will be valid under whatever rules we decide on.

Crates.io usernames must:

- Only contain alphanumeric characters `[a-zA-Z0-9]`, hyphens `-`, and underscores `_`
  [^why_underscores].
- Be unique case insensitively and hyphen/underscore insensitively, much like crate names. That is,
  uniqueness will be determined by normalizing case and normalizing hyphens and underscores
  together. For example, the crates.io usernames `hello-there` and `Hello_There` will be considered
  to be the same: once a user named `hello-there` exists, a user named `Hello_There` will not be
  allowed.
- Not start with a hyphen or underscore. See [Unresolved Questions](#unresolved-questions) for why
  not prohibiting ending with these characters or prohibiting two of these characters in a row.
- Not exceed 39 characters.

[^why_underscores]: Even though `github.com` does not allow you to create an account with a
username that contains an underscore, [Enterprise Managed
Users](https://docs.github.com/en/enterprise-cloud@latest/admin/managing-iam/iam-configuration-reference/username-considerations-for-external-authentication) get a username that ends in `_[enterprise
shortcode]`. We have accounts of this sort in crates.io's database today.

We have a list of reserved crate names that no one may register that includes top-level Rust
standard library modules and keywords, reserved Windows filenames, and some swear words or slurs
(which will never be exhaustive but contains the most common ones in English). We'll have a similar
list of reserved usernames that no one may use; GitHub's Terms of Service is providing us some
protection currently that we'd need to manage ourselves.

These requirements will be clearly documented on a page on crates.io as well as in the signup form
when we are requiring the person to pick a crates.io username.

## Crates.io account rename restrictions

The biggest concerns with allowing crates.io username changes are impersonation and resurrection
attacks.

Impersonation is already possible and is already against [crates.io
policies](https://crates.io/policies), but of course we don't want to make it easier to falsely
gain the trust of crates.io users by pretending to be a well-known person. We plan to add
typosquatting checks on usernames similar to those we're already doing for crate names. We also
plan to limit how often you can change your crates.io username (say, not more often than once every
30 days).

Resurrection attacks are a subset of impersonation, where a user named `carols10cents`, for
example, renames away from that username or deletes their account and another user claims the
`carols10cents` username to appear to be that person to users who don't know about the rename or
deletion. We plan to limit the re-use of usernames, using a similar mechanism that we have today
that prevents re-use of a deleted crate name, so that no one could claim an abandonded username
for, say, 30 days.

We also plan to mitigate the effectiveness of impersonation attacks by making the display of the
linked accounts associated with a crates.io account very clear so that anyone is able to feel
confident that the crates.io account has the same owner as the GitHub, GitLab, etc account they
trust.

In the database, accessible by admins only, we will track history of username changes (starting
from whenever the feature is implemented; we don't have historical data of GitHub username
changes). This could be useful for forensic investigation of accounts that may be attempting to
impersonate other users. We could display historical usernames and their dates on a user's page for
transparency, but this would be problematic in cases such as someone transitioning and wanting to
remove all association with their deadname (if their name was part of their crates.io username). We
will update [the privacy policy](https://rustfoundation.org/policy/privacy-policy/) section on
crates.io to make this retention clear, and we will delete even admin-only viewable information
from the database on request.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section will address changes to crates.io's HTTP API. It will not address low-level
implementation details of the crates.io database schema or backend code changes; those will be
worked out during implementation of this RFC.

## User API

The `find_user` API is currently defined to respond to URLs in the form `/api/v1/users/{user}`,
where `{user}` is currently the GitHub username that crates.io has been told about for that account.

This route would be changed such that `{user}` would be assumed to be the crates.io username only.
So for a user with crates.io username `carols10cents` and GitHub username `carolgithub`, the API
request `/api/v1/users/carols10cents` would return this user's information, and requesting
`/api/v1/users/carolgithub` would return a 404 Not Found, because there is no crates.io username
`carolgithub`.

We could choose to have this route's implementation attempt a lookup in the table of GitHub
usernames (and eventually in other services' tables when those are supported) if no crates.io
username is found, but that seems like it could cause confusion.

See the [Unresolved Questions](#unresolved-questions) section for possibly changing this API's
parameter to optionally accept a prefix with the username to allow lookup by GitHub (or other
service) username.

`/api/v1/users/{user}` currently returns this information (for the `carols10cents` user):

```json
{
	"user": {
		"id": 396, // crates.io database ID
		"login": "carols10cents", // crates.io and GitHub username
		"name": "Carol (Nichols || Goulding)", // display name (as set in GitHub)
		"avatar": "https://avatars.githubusercontent.com/u/193874?v=4", // from GitHub
		"url": "https://github.com/carols10cents" // assumes GitHub
	}
}
```

This RFC would change the following:

- `login` would be the crates.io username
- `name` would be deprecated and the UI would use `login` (we could continue to provide the `name`
  attribute with its value set to `login` to ease migration. Also see the "display name" Unresolved
  Question)
- There would still be an `avatar` URL returned; for how that is managed, see the [Unresolved
  Questions](#unresolved-questions)
- `url` would be deprecated in favor of explicitly requesting linked account information (we could
  continue to provide a GitHub URL if the user has a linked GitHub account)
- A field named `github_useranme_matches` (with a value of a boolean that the user's GitHub
  username is either the same as, or different from, their crates.io username)

So that the response eventually looks like this (without the values supporting deprecated fields
that we may choose to offer for more compatibility) for crates.io user `carols10cents` that has
GitHub username `carolgithub`:

```json
{
	"user": {
		"id": 396, // same as before; crates.io database ID
		"login": "carols10cents", // now the crates.io username
		"avatar": "https://avatars.githubusercontent.com/u/193874?v=4",
        "github_username_matches": false,
	}
}
```

This request would require querying the `users` table and the `oauth_github` table, but not any
other services' linked tables to limit database load by default. The `github_username_matches`
would be used to decide whether to show the ⚠️ warning about username mismatches discussed in the
Guide section.

If desired, the requester could instead use `/api/v1/users/{user}?include=linked_accounts` (much
like the current crate API allows for opt-in of returning related data) which would query all OAuth
tables and return all account information for this user once crates.io supports more services:

```json
{
	"user": {
		"id": 396, // same as before; crates.io database ID
		"login": "carols10cents", // now the crates.io username
		"avatar": "https://avatars.githubusercontent.com/u/193874?v=4",
        "github_username_matches": false,
	},
    "linked_accounts": [
        {
            "service": "github",
            "login": "carolgithub",
            "avatar": "https://avatars.githubusercontent.com/u/193874?v=4"
        },
        {
            "service": "gitlab",
            "login": "carols10cents"
            "avatar": "https://secure.gravatar.com/avatar/5eefdbf7a532f1a36d5cdce703a3b346cadbebc6098c4fce5354af871f662f55"
        }
    ]
}
```

The `?include=linked_accounts` variant would be called by the frontend when visiting
`https://crates.io/users/carols10cents`, to be able to display all of a user's linked accounts.

We would likely not request linked accounts for crate pages when displaying ownership information;
we could add a way to view that information on a crate page on-demand, for example when hovering
over an owner (or tapping on an icon next to the owner that the frontend shows when viewed on
mobile devices), we could request the linked account information then and display a "detail card"
for that owner showing the information on their linked accounts.

## Owner APIs

The current API request for inviting user owners or adding team owners consists of a `PUT` request
to `/api/v1/crates/[crate name]/owners` with the following JSON (using a request to add user
`some_user` and team `some_team` from the `some_org` GitHub organization as an example):

```json
{
    "owners": [
        "some_user",
        "github:some_org:some_team"
    ]
}
```

The backend processes owner strings starting with `github` and containing two colons as an organization name and a team name; this behavior will be unchanged.

This request will begin to accept owners specified by strings containing one colon and starting
with `cratesio`[^1], `github`, and any other OAuth service we eventually add. An owner specification
of `cratesio:some_user` will only query `users.username` and not any other table. An owner
specification of `github:some_user` will only query `oauth_github.login` and not any other table.
As other services are added, we will add a prefix that can be used to only look up usernames in
that service's table. If the username isn't found in the specified table (say, the `cratesio`
prefix that specfies the `users` table), the request will return an error even if the username is
in another table (such as the `oauth_github` table, for this example).

[^1]: The `cratesio` prefix may possibly be `crates.io`, `crates_io`, `crates-io`, or all of them,
to be bikeshed during implementation.

If the owner specification doesn't contain any colons, the behavior is similar to that of the users
API: we assume it's a crates.io username and look it up in `users.username` only. We will also
query the `oauth_github` table to see if the crates.io username and GitHub username match. If they
do match, we will continue with adding this user as an owner. If they don't match, we will return
an error containing information about the mismatch and asking the user to rerun the command with a
service prefix and colon in front of the username to ensure we're adding the account they mean to
add.

An error response would look something like this:

```json
{
    "errors": [
        {
            "detail": "username `some_user` is possibly ambiguous. The crates.io account
                       `example_username` is associated with:

- https://github.com/something_else
- [any other accounts once we have that ability]

To confirm this is the account you want to add, please run one of the following:

$ cargo owner --add cratesio:example_username
$ cargo owner --add github:something_else

If this is not the account you want to add, verify the crates.io username of the account you want.
                      "
        }
    ]
}
```

This maintains backwards compatibilty with existing `cargo` versions. We could do additional work
on Cargo and add more fields if a newer version of Cargo is making the request, to support a
"confirmation" flow as presented in the "Prior Art" section under Keybase.

The "remove owner" API would behave similarly as the "add owner" API - it will support `cratesio:`
or `github:` (etc) prefixes to usernames and will return an error if there is no current owner with
the specified username in the specified service's table. If given a username without a prefix, the
"remove owner" API will only return an error if there are two current owners of the crate that have
the username on different services, and will then ask the user to rerun with a prefix to
disambiguate. That is, if the user runs `cargo owner --remove some_user` and there's a crates.io
user with the `users.username` of `some_user` and a different account that has the GitHub user
`some_user`, the API will only return an error if both these accounts are owners of the crate the
request is being made about. If only one account is an owner, that account will be removed as an
owner.

# Drawbacks
[drawbacks]: #drawbacks

- Impedes signup flow if you have to choose a username or try multiple usernames before finding an
  available one
- Could cause confusion during signup and user lookup
- People who can't or don't want to have their GitHub username and crates.io username match will
  have a warning by their username that they might not want to have there and might imply there's
  something wrong or untrustworthy about their account when that isn't the case

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could choose to diverge from crates.io's current behavior more than proposed here, such as:

- We could force everyone to specify whether they mean the crates.io username or GitHub username
  for every lookup, to force education that they're no longer guaranteed to be the same. That is,
  if someone runs `cargo owner --add example`, we'd always return an error and ask them to rerun
  `cargo owner --add cratesio:example` or `cargo owner --add github:example` explicitly even if
  they both refer to the same account. This could be confusing for the most common case, but would
  be a way to force communication with people that something is changing.
- We could implement the backend changes for this RFC but choose to wait to allow username editing
  until we have multiple ways of logging in.

## "Disambiguation page" alternative

We could choose not to have a `username` field on the `users` table at all. When visiting
`https://crates.io/users/example`, crates.io would always look up usernames in all available
`oauth_*` tables. If only one `user` record was associated with `oauth_*` records that had the
username `example`, we'd display that user's information. This would nicely handle the most common
case of having one user with a unique GitHub username.

If, however, there is one crates.io `user` record (id 1234) associated with the `oauth_github`
record that has the username `example`, and another crates.io `user` record (id 5678) associated
with the `oauth_gitlab` record that has the username `example`, we could show content similar to a
Wikipedia "disambiguation page", something like:

> There are multiple users with the username "example". Did you mean:
>
> - [example on GitHub](https://crates.io/users/example/1234)
> - [example on GitLab](https://crates.io/users/example/5678)

and then you'd have to click an extra time. We could support direct linking to these users either
by including their crates.io user record ID (something like
`https://crates.io/users/example/1234`), or by including the name of the service where they hold
the username (something like `https://crates.io/users/example/github`).

For the `cargo owner --add` CLI, we could show similar disambiguation text and exit with an error:

```console
$ cargo owner --add example

error: There are multiple users with the username "example".

If you meant https://github.com/example, rerun with `cargo owner --add github:example`.
If you meant https://gitlab.com/example, rerun with `cargo owner --add gitlab:example`.
```

The disambiguation page and extra specification for users who happen to have colliding usernames
with users from other services is a bit more friction and annoyance for those people, through no
real fault of their own. However, this case should be fairly rare.

Looking up a user would be more complex and require more queries to more tables, which may impact
overall performance.

This idea may not match people's expectations of how crates.io usernames work, causing confusion.
If a GitHub user with the username `best_rust_programmer_ever` was well known in Rust spaces but
actually never logged in to crates.io, but someone else claimed the username
`best_rust_programmer_ever` on GitLab and _did_ log in to crates.io, the content on
`https://crates.io/users/best_rust_programmer_ever` would only show the GitLab user's information
with no indication that the GitHub user even exists. We'd need to make the page content clear that
there was only a GitLab account attached, not a GitHub account attached as most people would expect
in most cases.

# Prior art
[prior-art]: #prior-art

Crates.io appears to be unique among the major OSS package registries in only offering GitHub
OAuth, so there aren't direct lessons we can draw from other ecosystems. Here are a few examples:

[PyPI](https://pypi.org) [does not currently support changing a
username](https://pypi.org/help/#username-change). Instead, you can create a new account with the
desired username, add the new account as a maintainer of all the projects your old account owns,
and then delete the old account, which will have the same effect. There is no OAuth support.

[npm](https://www.npmjs.com/) (JavaScript) does not have any OAuth login mechanisms. [Their
policies](https://docs.npmjs.com/policies/disputes) state they are "extremely unlikely to transfer
control of a username, as it is totally valid to be an npm user and never publish any packages".
[It is not currently possible to change your npm
username](https://docs.npmjs.com/changing-your-npm-username) other than creating a new account and
migrating data manually. When npm accounts are deleted, usernames become available for anyone to
claim again after 30 days.

[Maven Central](https://central.sonatype.com/) (Java) allows you to create an account and log in
via email or OAuth with Google, GitHub, or Microsoft. There is no way to rename, update or change
your Maven Central username. If you want a different username, you have to create a new account.
However, usernames don't appear to be as important as they are on crates.io. Maven Central is
organized around domain-based namespaces registered through DNS, and it's the namespace that
conveys authority.

[Keybase](https://keybase.io/) is a service that tries to make working with public key cryptography
easier. They have ways of proving ownership of various accounts on other services to help people
ensure they're communicating with the account that belongs to the intended person. Keybase also has
a CLI with a confirmation flow that we could use as inspiration for the `cargo owner --add` user
flow. See [the Keybase documentation](https://book.keybase.io/docs/server), under the heading "Step
3: the human review":

> Recall, in Step 2 your client proved "maria" has a number of identities, and it cryptographically
> verified all of them. Now you can review the usernames it verified, to determine if it's the
> maria you wanted.
>
> ```console
> ✔ maria2929 on twitter: https://twitter.com/2131231232133333...
> ✔ pasc4l_programmer on github: https://gist.github.com/pasc4...
> ✔ admin of mariah20.com via HTTPS: https://mariah20/keybase.tx...
>
> Is this the maria you wanted? [y/N]
> ```

With `cargo owner --add`, once we support multiple logins, the CLI could look something like this:

```console
$ cargo owner --add carols10cents

Crates.io account `carols10cents` is associated with:
✔ https://github.com/carols10cents
✔ https://brand-new-code-hosting-platform.dev/some_other_username

Is this the `carols10cents` you wanted? [y/N]
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How would we define "squatting" of usernames that would be clear cases for admins to make
  available again? Accounts that don't have any crates and that don't have any valid associated
  OAuth logins (that is, all the associated accounts have been deleted)?
- We currently have the concept of a "display name" that is associated with and managed through
  your GitHub "display name" that's currently used on crate pages for owners and user pages. Do we
  want to have an authentication-independent "display name"? Or should we get rid of the "display
  name" concept completely and only show crates.io usernames everywhere? It seems like the display
  name would be a vector for possible impersonation (one that's technically possible today. It is
  against crates.io policies, and as far as I can remember we haven't had an account impersonation
  reported).
- Avatars are also indicators of identities.
  - Once we have multiple authentication services, each possibly providing an avatar, which do we
    display when a crates.io user account has different accounts associated and different avatars?
  - We currently don't host avatar images; we don't really want to
  - Could we let the user choose between:
    - Avatar associated with a linked OAuth account, hosted by the linked service (like we do today
      with GitHub avatars)
    - Gravatar associated with the user's verified crates.io email address (which the user could
      customize via Gravatar)
    - One placeholder image we provide (ex: Ferris)
  - Again, I think impersonation via avatar is technically possible today with only GitHub unless
    GitHub policy enforcement disallows that, and I don't think the decision on avatar resolution is
    as important as username resolution, but it might make implementation/database queries nicer if
    we make a similar decision with avatars as with usernames.
- We are not yet committing to the support of organization/team owners from other services, but
  adding teams already requires specifying a literal `github:` before the `org:team` when adding
  team owners so there shouldn't be as much confusion around the identity of a team if we choose to
  add different team owners via different services.
- Should we have a URL for user pages that uses their crates.io ID number (ex:
  `https://crates.io/users/id/1234` would run the query `SELECT * FROM users WHERE id = 1234`), and
  thus won't change no matter what happens with the account's username or connected accounts?
  - How would people discover what their crates.io ID is (without needing to look in the API
    response)?
    - Should we start displaying it on user pages?
    - Should we start using these ID-based URLs as the canonical user URLs? That is, should
      visiting `https://crates.io/users/carols10cents` redirect to `https://crates.io/users/id/396`?
    - Should we accept it in the CLI, such as `cargo owner --add id:396`?
- Should we have a URL for user pages or for the user API that allows specifying a prefix of an
  OAuth service, to allow for looking up a crates.io user when you only know, say, their GitHub
  username? For example, the user page at `https://crates.io/users/{user}` calls the `find_user`
  API, which is currently defined to respond to URLs in the form `/api/v1/users/{user}` where
  `{user}` is the username (which is currently the GitHub username but when this RFC is implemented
  will be the crates.io username).

  This route could be changed and expanded to allow disambiguation between GitHub, other services,
  and crates.io usernames. So for a user with crates.io username `carols10cents` and GitHub
  username `carolgithub`, these API requests could return the same information for this user:

  ```text
  /api/v1/users/carols10cents            // assumes this is the crates.io username
  /api/v1/users/cratesio:carols10cents
  /api/v1/users/github:carolgithub
  ```

  Is this behavior useful and do we want to commit to it in our public API? Do we want to offer it
  in an experimental form to see how/if it's used?
- Is there a way we could avoid having both hyphens and underscores in usernames and needing to
  normalize them together for uniqueness purposes, which can be confusing?
- There are usernames in the database currently that contain two hyphens in a row or end in a
  hyphen. Example: [@ra--](https://github.com/ra--). I suspect GitHub didn't initially prohibit
  this, but now prohibit new accounts from doing this. If we wanted to disallow consecutive hyphens
  or ending in a hyphen, and someone with a legacy GitHub account like this who hasn't signed up
  for crates.io before but does after this change, we could force them to have a crates.io username
  that doesn't match their GitHub account. This would mean they'd get the ⚠️ which would be unfair
  because there's no way they could match their GitHub account. Alternatively, we could allow
  consecutive hyphens or ending in hyphen _only_ if that matches your GitHub account exactly. Would
  this be confusing? Is this a significant number of accounts worth handling specially?

# Future possibilities
[future-possibilities]: #future-possibilities

This functionality change would also enable a way of creating crates.io accounts without any
associated identity/reputation, only an email address. But this opens more potential for spam and
abuse as it's easier to create anonymous email addresses than it is to maintain accounts in good
standing on services like GitHub. When we choose which services to add as OAuth providers, we will
assess in what ways the candidate services also provide these protections if we want to continue to
have this benefit.
