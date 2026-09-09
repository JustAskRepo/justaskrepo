-- Symmetry with user_repositories, captured now rather than later.
--
-- `GET /installation/repositories` returns the default branch in the same
-- response that populates this table, so storing it costs no extra request.
-- Adding it later would be genuinely awkward: unlike user_repositories, which
-- is rewritten on every login, this table's only writer is a webhook — a column
-- added after the fact would stay wrong until the installation happened to
-- change again, which for a settled installation could be never.
ALTER TABLE installation_repositories
    ADD COLUMN default_branch TEXT NOT NULL DEFAULT 'main';

ALTER TABLE installation_repositories
    ALTER COLUMN default_branch DROP DEFAULT;
