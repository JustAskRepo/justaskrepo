-- Installations — the GitHub App grants, what they cover, and who may see them.
--
-- Three tables rather than one wide one, because the three facts have different
-- writers and different freshness (ADR-009):
--
--   installations              the grant itself  -- webhooks + login reconciliation
--   installation_repositories  what it covers    -- webhooks, re-fetched and replaced
--   user_installations         who may see it    -- login reconciliation only
--
-- The user link is a table and not a column on `installations` because an
-- installation can exist with nobody attached: someone can install the App
-- without ever logging in, and the webhook announcing it arrives for a user we
-- have never seen.

CREATE TABLE installations (
    -- GitHub's id, not ours. Every writer upserts on it, which is what makes the
    -- setup redirect and the installation.created webhook safe to race.
    id                   BIGINT PRIMARY KEY,

    account_id           BIGINT NOT NULL,
    account_login        TEXT   NOT NULL,
    -- Stored as GitHub sent it. 'User' and 'Organization' are what exist today,
    -- and a value GitHub adds later should not fail a webhook — so no CHECK.
    account_type         TEXT   NOT NULL,

    -- Constrained, unlike account_type, because it is load-bearing: under 'all'
    -- GitHub emits no delta for repositories created afterwards, so a reader has
    -- to know it cannot treat installation_repositories as complete.
    repository_selection TEXT   NOT NULL
        CHECK (repository_selection IN ('all', 'selected')),

    -- NULL means active. Suspension is reversible, so it must not delete the
    -- repository set that a later unsuspend expects to find.
    suspended_at         TIMESTAMPTZ,

    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE installation_repositories (
    installation_id BIGINT      NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    -- GitHub's repository id: immutable across renames.
    repository_id   BIGINT      NOT NULL,
    -- Mutable and display-only. Deliberately not UNIQUE: a rename frees the old
    -- name for another repository, and a constraint violation is not the right
    -- answer to that.
    full_name       TEXT        NOT NULL,
    private         BOOLEAN     NOT NULL,
    added_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (installation_id, repository_id)
);

-- `indexing` asks which installation covers a repository in order to mint a
-- token for it, and leads with the repository id rather than the installation.
CREATE INDEX installation_repositories_repository_id_idx
    ON installation_repositories (repository_id);

CREATE TABLE user_installations (
    user_id         BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The foreign key is what forces login reconciliation to upsert the
    -- installation before linking to it. That ordering is correct: the login
    -- path reads full installation data from GitHub, so it has everything it
    -- needs to create the row it is about to reference.
    installation_id BIGINT      NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    -- When GitHub last confirmed this link. ADR-009 accepts that it can be up to
    -- one session stale; this column is what makes that staleness measurable.
    linked_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, installation_id)
);

-- Uninstall reads this by installation to find whose sessions to revoke.
CREATE INDEX user_installations_installation_id_idx
    ON user_installations (installation_id);
