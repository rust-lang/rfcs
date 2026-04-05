- Feature Name: `crates_io_bitbucket_oauth`
- Start Date: 2026-04-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Tracking Issue: [rust-lang/crates.io#0000](https://github.com/rust-lang/crates.io/issues/0000)

## Summary
[summary]: #summary

Add Bitbucket Cloud as an OAuth 2.0 login provider for crates.io,
allowing users who host their Rust projects on Bitbucket to authenticate
and publish crates without requiring a GitHub account. This RFC also
establishes the provider-abstraction layer that future OAuth providers
(GitLab, etc.) can plug into.

## Motivation
[motivation]: #motivation

crates.io has required a GitHub account for login since its launch in
2014. This has been a persistent pain point
([crates.io#326](https://github.com/rust-lang/crates.io/issues/326),
open since 2015) and creates several concrete problems:

1. **Single point of failure for identity.** Deleting or losing access
   to a GitHub account makes the user's crates.io account and all
   associated crates permanently inaccessible. There is no account
   recovery path that does not involve GitHub.

2. **Enterprise exclusion.** Organizations that standardize on Atlassian
   tooling (Bitbucket, Jira, Confluence) often have corporate SSO
   federated through Bitbucket Cloud. Requiring developers to maintain a
   separate GitHub account solely for crate publishing is friction that
   discourages internal crate ecosystem adoption.

3. **Privacy concerns.** GitHub OAuth with the `read:org` scope exposes
   private organization membership to crates.io
   ([crates.io#3027](https://github.com/rust-lang/crates.io/issues/3027)).
   Bitbucket's `account` scope does not leak workspace membership by
   default, giving privacy-conscious users an alternative.

4. **Ecosystem growth.** The Rust ecosystem benefits when publishing
   crates is accessible to all Rust developers regardless of their
   preferred source hosting platform.

### Use cases

- **Enterprise Rust teams on Atlassian Cloud** can publish internal and
  public crates using their existing Bitbucket identity, avoiding the
  need for shadow GitHub accounts.

- **Open-source maintainers with Bitbucket-hosted projects** can link
  their crate to their Bitbucket repository and authenticate with a
  single identity.

- **Workspace-based team ownership** lets Bitbucket workspaces function
  as crate co-owners, analogous to GitHub organization teams today.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Logging in with Bitbucket

The crates.io login page gains a "Sign in with Bitbucket" button
alongside the existing "Sign in with GitHub" button. Clicking it
initiates a standard OAuth 2.0 Authorization Code Grant flow with
Bitbucket Cloud:

1. The user is redirected to Bitbucket's authorization page.
2. Bitbucket asks the user to approve the `account` and `email` scopes.
3. On approval, the user is redirected back to crates.io with an
   authorization code.
4. crates.io exchanges the code for an access token and refresh token,
   fetches the user's profile, and creates or links the crates.io
   account.

After login, the experience is identical to a GitHub-authenticated user:
publishing, yanking, ownership management, and API token generation all
work the same way.

### Linking multiple providers

A user who already has a GitHub-based crates.io account can link their
Bitbucket identity from their account settings page. Once linked, they
can log in with either provider. The crates.io user ID remains the same;
only the set of linked OAuth identities changes.

A user cannot link a Bitbucket identity that is already associated with
a different crates.io account. In that case, the user must first unlink
the Bitbucket identity from the other account.

### Team ownership with Bitbucket workspaces

Bitbucket workspaces and user groups can be added as crate owners using
the same `cargo owner` interface:

```sh
# GitHub team (existing syntax, unchanged)
cargo owner --add github:rust-lang:core

# Bitbucket workspace user group (new)
cargo owner --add bitbucket:my-workspace:my-group
```

Membership is verified against the Bitbucket API at add-time and at
publish-time, just as it is for GitHub teams today.

### What does not change

- **API tokens** continue to work exactly as before. They are not tied
  to an OAuth provider.
- **Existing GitHub-based accounts** are unaffected. No migration is
  required.
- **The `cargo login` / `cargo publish` flow** is unchanged for users
  who already have an account.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Provider abstraction layer

The core design introduces a provider-agnostic identity model. Instead
of `gh_*` columns on the `users` table, each OAuth provider gets its own
association table. The existing `oauth_github` table (introduced in
January 2026) already follows this pattern.

#### New table: `oauth_bitbucket`

```sql
CREATE TABLE oauth_bitbucket (
    account_id  VARCHAR(63) PRIMARY KEY,
    -- Bitbucket user UUID, e.g. "{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}"
    user_id     INTEGER NOT NULL REFERENCES users(id),
    login       VARCHAR NOT NULL,
    avatar      VARCHAR,
    encrypted_token   BYTEA NOT NULL,
    encrypted_refresh_token BYTEA NOT NULL,
    token_expires_at  TIMESTAMPTZ NOT NULL,
    UNIQUE (user_id)
);

CREATE UNIQUE INDEX idx_oauth_bitbucket_user_id
    ON oauth_bitbucket(user_id);
```

Key differences from `oauth_github`:

| Aspect | `oauth_github` | `oauth_bitbucket` |
|---|---|---|
| `account_id` type | `BIGINT` (GitHub integer ID) | `VARCHAR(63)` (Bitbucket UUID string) |
| Refresh token | Not stored (GitHub tokens don't expire) | Stored, encrypted, rotated on use |
| Token expiry | None | `token_expires_at` column |

#### Generalized `OAuthProvider` trait

```rust
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Provider identifier used in team login strings, e.g. "github",
    /// "bitbucket".
    fn name(&self) -> &'static str;

    /// Build the authorization URL with appropriate scopes.
    fn authorize_url(&self, csrf_state: CsrfToken) -> (Url, CsrfToken);

    /// Exchange an authorization code for tokens.
    async fn exchange_code(
        &self,
        code: AuthorizationCode,
    ) -> Result<TokenResponse>;

    /// Refresh an expired access token.
    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse>;

    /// Fetch the authenticated user's profile.
    async fn current_user(
        &self,
        token: &AccessToken,
    ) -> Result<OAuthUser>;

    /// Check if a user is a member of a team/group.
    async fn team_membership(
        &self,
        org_id: &str,
        team_id: &str,
        username: &str,
        token: &AccessToken,
    ) -> Result<Option<TeamMembership>>;
}
```

The existing `GitHubClient` trait methods map directly onto this
interface. The Bitbucket implementation calls the corresponding
Bitbucket API endpoints:

| Operation | Bitbucket API endpoint |
|---|---|
| Current user | `GET /2.0/user` |
| User email | `GET /2.0/user/emails` |
| Workspace info | `GET /2.0/workspaces/{workspace}` |
| User group info | `GET /2.0/workspaces/{workspace}/permissions` |
| Group membership | `GET /2.0/workspaces/{workspace}/permissions` filtered by `user.uuid` |
| Workspace membership | `GET /2.0/workspaces/{workspace}/members/{user_uuid}` |

#### OAuth scopes

| Scope | Purpose |
|---|---|
| `account` | Read user profile, workspace membership, group membership |
| `email` | Read user email addresses (required for email verification) |

These are read-only scopes. No write access to Bitbucket repositories
or settings is requested.

### Token refresh

Bitbucket access tokens expire after 2 hours. This is a fundamental
departure from GitHub, where tokens do not expire. crates.io currently
uses stored GitHub tokens at publish-time to verify team membership,
which may occur days or weeks after login.

The solution is transparent token refresh:

1. When a Bitbucket token is needed (team membership check at
   publish-time, admin sync, etc.), the system first checks
   `token_expires_at`.
2. If the token is expired or within a 5-minute grace window, the system
   uses the stored refresh token to obtain a new access token and
   refresh token pair from Bitbucket's token endpoint.
3. Both new tokens are encrypted and persisted atomically.
4. If the refresh token itself has been revoked (user deauthorized the
   app), the API call fails with a clear error message asking the user
   to re-authenticate.

This refresh logic is encapsulated in a `TokenManager` that wraps any
`OAuthProvider` and handles expiry transparently.

### Session flow changes

The `/api/private/session/begin` endpoint gains an optional `provider`
query parameter:

- `GET /api/private/session/begin?provider=github` (default, backwards
  compatible)
- `GET /api/private/session/begin?provider=bitbucket`

The `/api/private/session/authorize` endpoint similarly gains a
`provider` parameter and dispatches to the appropriate `OAuthProvider`
implementation.

Session state stores `oauth_provider` alongside the existing
`oauth_state` (renamed from `github_oauth_state`) to ensure the
callback is routed to the correct provider.

### Account linking

When a user authenticates with a provider and the provider identity is
not yet associated with any crates.io account:

- If the user is **not** logged in: a new crates.io account is created.
- If the user **is** logged in (has an active session from another
  provider): the new provider identity is linked to the existing
  account.

When the provider identity is already linked to an account:

- If the user is not logged in: they are logged in to that account.
- If the user is logged in to a **different** account: an error is
  returned.

### Team ownership

The existing team login format `provider:org:team` already supports
arbitrary provider prefixes (the `Team::split_login()` parser handles
this). The changes needed are:

1. `add_team_owner` in `owners.rs` accepts `"bitbucket:"` as a valid
   prefix (currently only `"github:"` is accepted).
2. Team verification calls `BitbucketProvider::team_membership()` which
   queries `GET /2.0/workspaces/{workspace}/permissions` and filters
   for the user group.
3. The `teams` table schema is extended:

```sql
ALTER TABLE teams ADD COLUMN provider VARCHAR NOT NULL DEFAULT 'github';
ALTER TABLE teams ALTER COLUMN github_id DROP NOT NULL;
ALTER TABLE teams ADD COLUMN provider_team_id VARCHAR;
ALTER TABLE teams ADD COLUMN provider_org_id VARCHAR;
```

The `github_id` and `org_id` integer columns are retained for backwards
compatibility with existing GitHub teams. New Bitbucket teams use
`provider_team_id` (workspace user group UUID) and `provider_org_id`
(workspace UUID) as strings.

### Migration path for `users.gh_*` columns

This RFC does **not** propose removing the `gh_*` columns from the
`users` table. That is a separate migration effort that depends on the
`oauth_github` table being fully adopted as the source of truth. This
RFC layers Bitbucket support on top of the existing multi-provider
groundwork without requiring that migration to complete first.

### Configuration

New environment variables:

| Variable | Purpose |
|---|---|
| `BB_CLIENT_ID` | Bitbucket OAuth consumer key |
| `BB_CLIENT_SECRET` | Bitbucket OAuth consumer secret |
| `BITBUCKET_TOKEN_ENCRYPTION_KEY` | 64-char hex key for AES-256-GCM encryption of Bitbucket tokens |

The existing `GH_CLIENT_ID`, `GH_CLIENT_SECRET`, and
`GITHUB_TOKEN_ENCRYPTION_KEY` variables are unchanged.

### API changes

#### Modified endpoints

| Endpoint | Change |
|---|---|
| `GET /api/private/session/begin` | Accepts `?provider=github\|bitbucket` |
| `GET /api/private/session/authorize` | Accepts `?provider=github\|bitbucket` |
| `GET /api/v1/me` | Response includes `linked_providers: ["github", "bitbucket"]` |
| `PUT /api/v1/crates/:crate/owners` | Accepts `bitbucket:workspace:group` in the `users` array |

#### New endpoints

| Endpoint | Purpose |
|---|---|
| `POST /api/private/session/link` | Link an additional OAuth provider to the current account |
| `DELETE /api/private/session/link/:provider` | Unlink a provider (must have at least one remaining) |

### Frontend changes

- Login page shows provider selection buttons.
- Account settings page shows linked providers with link/unlink
  controls.
- The popup-based OAuth flow is generalized: `github-auth-loading.html`
  becomes `oauth-loading.html` with a `provider` parameter.

## Drawbacks
[drawbacks]: #drawbacks

1. **Maintenance burden.** Each OAuth provider adds API surface to
   maintain, test, and monitor. Bitbucket's API has different rate
   limits, pagination patterns, and error formats than GitHub's.

2. **Token refresh complexity.** Bitbucket's 2-hour token expiry
   introduces a refresh mechanism that does not exist today. This adds a
   failure mode (expired refresh tokens) and requires background or
   just-in-time token rotation logic.

3. **Lower demand than GitLab.** Bitbucket has a smaller market share
   among Rust developers than GitLab. However, the provider abstraction
   layer designed here makes GitLab support a straightforward follow-on,
   and enterprise Atlassian shops represent a distinct constituency from
   GitLab users.

4. **Team ownership complexity.** Bitbucket workspaces and user groups
   do not map 1:1 to GitHub organizations and teams. Workspace
   permissions are more granular (admin, collaborator, member) and user
   groups are a separate concept from workspace membership.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why a provider abstraction rather than just adding Bitbucket?

A direct Bitbucket integration without abstraction would duplicate the
GitHub-specific patterns (hardcoded columns, provider-specific
controllers). The `oauth_github` table already signals intent to
decouple provider identity from the core user model. This RFC extends
that pattern with a trait-based abstraction that makes the third provider
(GitLab, or any OIDC-compliant IdP) a straightforward implementation
exercise rather than another ad-hoc integration.

### Why not OIDC/generic OpenID Connect instead?

A generic OIDC provider would be more flexible but raises significant
trust and moderation questions (who can register an IdP? how are
namespaces managed?). Bitbucket Cloud has a well-defined OAuth 2.0
implementation and a known, bounded user population. Starting with
named providers and evolving toward generic OIDC later is lower risk.

### Why not just use API tokens and skip OAuth entirely?

API tokens solve the publish workflow but not the identity problem. Users
still need to create an account, and the account creation flow requires
OAuth. Additionally, team-based ownership verification requires
real-time API calls to the source hosting provider.

### What is the impact of not doing this?

crates.io remains GitHub-exclusive. Enterprise teams on Atlassian
tooling continue to need shadow GitHub accounts. The single-provider
dependency risk persists.

## Prior art
[prior-art]: #prior-art

- **npm** supports GitHub, Google, and email/password authentication.
  Team/organization ownership is managed internally rather than being
  delegated to a source hosting provider.

- **PyPI** uses email/password and supports OIDC trusted publishers for
  GitHub Actions, GitLab CI, Google Cloud Build, and ActiveState.
  PyPI's OIDC trusted publishing model (RFC 3691 for crates.io)
  is orthogonal to user authentication but demonstrates the ecosystem
  trend toward multi-provider support.

- **Docker Hub** supports GitHub, Google, and email/password
  authentication. Organization membership is managed internally.

- **RubyGems.org** uses email/password with optional MFA. No OAuth
  provider login, though there have been proposals to add it.

- The crates.io codebase itself has begun preparing for multi-provider
  auth: the `oauth_github` table (January 2026) and the
  `Team::split_login()` parser that already handles arbitrary provider
  prefixes are direct precursors to this RFC.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- **Should refresh token rotation be synchronous or asynchronous?**
  Just-in-time refresh at publish-time is simpler but adds latency.
  A background job that proactively refreshes tokens nearing expiry
  reduces publish-time latency but adds infrastructure complexity.
  This should be resolved during implementation.

- **What happens when a Bitbucket refresh token is revoked?** The user
  must re-authenticate, but should the system email them proactively
  when it detects a revoked token (e.g., during a background refresh
  attempt), or only surface the error at the next interactive action?

- **Should the `teams` table be fully generalized now or later?** This
  RFC proposes adding `provider`, `provider_team_id`, and
  `provider_org_id` columns alongside the existing `github_id` and
  `org_id`. An alternative is a separate `teams_v2` table with only
  string-typed provider IDs. The migration path should be decided
  during implementation.

- **Display name precedence.** When a user has both GitHub and Bitbucket
  linked, which login/avatar is shown by default? This RFC suggests the
  first-linked provider but the UX decision should be finalized with
  the crates.io frontend team.

## Future possibilities
[future-possibilities]: #future-possibilities

- **GitLab OAuth.** The provider abstraction layer makes GitLab support
  a direct follow-on. GitLab's OAuth 2.0 flow is similar to GitHub's
  and its tokens also do not expire by default (when using personal
  access tokens), simplifying the integration.

- **Generic OIDC.** Once multiple named providers are supported, a
  generic OIDC provider option could allow enterprise IdPs
  (Okta, Azure AD, Auth0) to authenticate directly.

- **Trusted publishing for Bitbucket Pipelines.** RFC 3691 establishes
  OIDC-based trusted publishing for GitHub Actions. Extending this to
  Bitbucket Pipelines would let Bitbucket-hosted projects publish
  crates from CI without long-lived API tokens.

- **Provider-agnostic team ownership.** Once multiple providers support
  team ownership, cross-provider teams could be explored (e.g., a crate
  co-owned by a GitHub org team and a Bitbucket workspace group).

- **Deprecation of `users.gh_*` columns.** Once the `oauth_*` tables
  are the established source of truth for all providers, the legacy
  `gh_*` columns on the `users` table can be dropped in a future
  migration.
