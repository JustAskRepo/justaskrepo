-- The dashboard's repo card renders the default branch, and `indexing` will
-- need it to know what to clone. GitHub returns it on the same listing that
-- already populates this table, so it costs no extra request.
--
-- Backfilled to 'main' for rows written before this column existed, then the
-- default is dropped so every future insert must supply the real value. The
-- backfilled guess corrects itself on the owner's next login: reconciliation
-- replaces a user's rows wholesale rather than patching them (ADR-009).
ALTER TABLE user_repositories
    ADD COLUMN default_branch TEXT NOT NULL DEFAULT 'main';

ALTER TABLE user_repositories
    ALTER COLUMN default_branch DROP DEFAULT;
