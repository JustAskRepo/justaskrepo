-- What a *user* may see, as distinct from what an *installation* covers.
--
-- These are not the same set, and conflating them over-grants. GitHub exposes
-- them through two different endpoints with two different credentials:
--
--   GET /user/installations/{id}/repositories  (user token)        -> this table
--   GET /installation/repositories             (installation token)-> installation_repositories
--
-- For a personal installation they agree. For an organization they need not:
-- an installation can cover fifty repositories while a given member has access
-- to three. Authorizing from installation_repositories would show that member
-- the other forty-seven, so the dashboard and every access check read here.
--
-- One writer, like every other table in 0002: login reconciliation (ADR-009
-- decision 1), which replaces this user's rows wholesale rather than diffing.
-- installation_repositories keeps its own writer — the webhooks of phase 6 —
-- and stays empty until then.

CREATE TABLE user_repositories (
    user_id         BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- GitHub's repository id. Not a foreign key to installation_repositories:
    -- that table is written by a different source at a different time, and a
    -- user's access must not depend on whether a webhook has arrived yet.
    repository_id   BIGINT      NOT NULL,
    -- Which grant this access comes through — the installation whose token
    -- `indexing` will mint to read the repository.
    installation_id BIGINT      NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    full_name       TEXT        NOT NULL,
    private         BOOLEAN     NOT NULL,
    -- When GitHub last confirmed this row. Same purpose as user_installations
    -- .linked_at: it makes ADR-009's one-session staleness measurable.
    synced_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, repository_id)
);

CREATE INDEX user_repositories_installation_id_idx
    ON user_repositories (installation_id);
