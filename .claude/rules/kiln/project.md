<!-- Written by kiln and kept current by `kiln update`: do not edit, it would be overwritten.
Project rules go in CLAUDE.md or in .claude/rules/ outside kiln/. -->
# slake: working here

kiln's template engine, in Rust: renders a Copier-format template and updates the projects made from it with a three-way merge.

Size: **Ember**, a small app or site, one or two people, low stakes (`kiln set size=…` changes it).
No stack of kiln here: its common part only (CI, hooks, rules, charter).
The repo is generated and kept current by [kiln](https://github.com/antoine-tena/kiln).

## Commands
- `kiln check`: `charter`, the rules of `charter.toml`.
  Run it before saying a change is done.
- `kiln update`: kiln's improvements and compatible dependency upgrades, on a branch, checks
  included; `kiln set key=value` changes a setting of the project (stack, team, podman…).

## Language
- Talk with the user in French.
- Docs (README, `docs/`): English. Commit messages:
  English. Code comments: English. Tickets of
  `docs/backlog/`: English. Texts users read (pages, messages,
  figures): English.
- Code identifiers are always English. The keywords of `docs/backlog/` (`Type`, `Status`,
  `todo`, section titles) stay English whatever the language.

## Where the state of the project lives

- The README describes the project as it is now: a change to a command, a setting, a route or a
  way of running the project updates it in the same branch, never "later".

- `docs/backlog/` is the only up-to-date list of work: `backlog.md` (open tickets `T-NN` by
  priority, how to write them in its `README.md`), `done.md` (closed ones), `rejected.md` (what was turned down), `urgent.md`
  (what only a human of the team can do). Skill `/kiln:backlog`.
- `charter.toml`: the checked rules and their budgets; `kiln-waivers.toml`: deliberate
  deviations; `.copier-answers.yml`: the project's settings for kiln (changed by `kiln set`,
  never by hand).

## Git

- **[NO-COMMIT-ON-MAIN]** Never commit on `main` nor on `develop`: always a dedicated branch,
  preferably in its own worktree (`/kiln:worktree`, under `.claude/worktrees/`), so that two
  sessions never cross in the main tree. It is merged into `develop` by `/kiln:merge`, directly:
  no pull request nor approval required (a PR stays possible to ask for an opinion). Enforced by
  the `no-commit-to-branch` pre-commit hook, kiln's branch guard, and GitHub rulesets (`develop`
  can be neither deleted nor force-pushed; `main` only takes releases, through `/kiln:release`).
- **[BRANCHES]** Prefixes `feat/`, `fix/`, `refactor/`, `test/`, `chore/`, `docs/`; lowercase,
  no spaces (`feat/station-map`).
- **[COMMITS]** Small and atomic: 10 files at most, a title of 50
  characters at most, prefixed by its type with an optional scope (`feat:`, `fix:`,
  `refactor:`, `style:`, `docs:`, `perf:`, `test:`, `build:`, `ci:`, `chore:`, `revert:`;
  `fix(map): …`). Whether a commit says Claude helped (`Co-Authored-By: Claude`)
  is each machine's choice (`kiln set ai_trailers=disclose|never`, kiln's branch guard enforces
  it); never a `Claude-Session:` line, never `--author`: the author stays the one of
  `git config`.
- **[MERGE]** Merge commits (`git merge --no-ff`), never squash nor rebase onto the integration
  branch; then the branch goes, local and remote, with its worktree (`/kiln:merge`).

## Way of working

- **[REPRODUCE-FIRST]** A bug is reproduced, by a failing test or a command, before it is fixed.
  A diagnosis, one's own or another session's, is checked on a copy first.
- **[DIRECT-LINKS]** A step asked of a human online (a console, a back office, a form) comes with
  its direct link, as precise as possible, never a menu path.
