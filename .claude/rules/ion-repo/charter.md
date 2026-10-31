<!-- Written by ion-repo and kept current by `ion repo update`: do not edit, it would be overwritten. A
deliberate deviation goes through `ion repo waive`; a rule of this project goes in CLAUDE.md or in
.claude/rules/ outside ion-repo/. -->
# Charter

The norm of this repository's code, for humans and agents alike: new code follows it. Rules for
a stack (backend, frontend, operations) live next to this file and load when Claude works on
matching files.

## How it works

- Each rule has a stable identifier in brackets, such as **[FILE-SIZE]**. Code, tests,
  `charter.toml`, reviews and commits cite a rule by its identifier, never by a line number. An
  identifier never changes and is never reused.
- Three regimes. **Checked**: no offence tolerated, the `charter` command fails (after each edit,
  before each commit, in CI). **Ratchet**: an existing debt has a budget in `charter.toml` that
  only goes down; lower it in the commit that reduces the debt. **Review**: no machine check,
  applied in review (`charter --unverified` lists them).
- On conflict: security and data integrity > published API contract > readability > form.
- **[WAIVERS]** A deliberate deviation from a rule is declared with `ion repo waive <RULE> --reason
  "…"` (in `kiln-waivers.toml`), never worked around in silence: a week at first, then a month,
  three months and six months at each renewal; an expired waiver no longer covers anything. A
  generated file is frozen the same way (`ion repo waive <file>`). Never add nor renew a waiver
  without the user's explicit agreement, and never to make a check pass. [PERSONAL-DATA],
  [SECRETS], [NO-COMMIT-ON-MAIN] and [LOCKED-DEPENDENCIES] cannot be waived.
- **[REPORT-TO-ION-REPO]** A defect in what ion-repo provides (a generated file, a rule of this charter,
  a skill or hook of ion-repo's Claude Code plugin, a reusable workflow) is not fixed in this repo:
  `ion repo update` would undo it. Report it with `/repo:report`: a pull request when the fix is
  clear and local, an issue otherwise. It is public, under the user's GitHub account: only with
  their explicit agreement, in English, with nothing private from this repo.

## Size and structure

- **[FILE-SIZE]** 300 lines at most per file (ratchet).
  Exempt: generated files, those `.gitattributes` marks `linguist-generated`, its only list.
- **[FUNCTION-SIZE]** One function, one responsibility, 30 lines at most (outside component
  templates and docstrings); a test may reach 40 when splitting it would hide the scenario.
- **[ONE-CLASS]** One class per file. Exceptions, still under [FILE-SIZE]: the schemas of one
  domain, the exceptions of one domain, the test classes of one test file.

## Names

- **[GENERIC-NAMES]** No generic names: `data`, `df`, `res`, `val`, `tmp`, `info`, `obj`, `item`
  or `result` alone, `x`. The name says *what* the variable holds (`sales_by_month`,
  `station_coordinates`). Only exception: a parameter imposed by a framework's signature.
- **[VOCABULARY]** The code speaks English, in the words of its domain; texts users read follow
  the project's language. Page addresses are public URLs and need not follow the code.
- **[BOOLEANS]** A boolean is prefixed: `is_`, `has_`, `should_`, `can_` in Python (`is_active`).
- **[CASE]** Classes in PascalCase; constants in `UPPER_SNAKE_CASE`;
  files follow their language's convention.

## Values

- **[NO-MAGIC-VALUES]** No business literal in the code (status, role, threshold, delay, price,
  colour, repeated column name): `enum.StrEnum` or a named constant at the top of the module in Python.
  Not magic: `0`, `1`, `-1`, `""`, `None`, indexes and literals in tests.
- **[MONEY]** Every amount is an integer in minor units (`*_cents`), in the database, the API and
  the front; never a float for money, converted only for display.
- **[TIME-ZONE]** Store instants in UTC. Every calendar notion (today, a day, a month, the bounds
  of a period) is computed in `Europe/Paris`, never with `.date()` on a UTC instant.

## Personal data and secrets

- **[PERSONAL-DATA]** No personal data (email, name, phone, ID document, full IP) in logs, error
  messages, URLs, cache keys or figures: log the identifier (`user_id`), not the person.
- **[SECRETS]** No secret in the code nor in the repository: everything goes through the
  environment ().
  A secret committed by mistake is compromised: it gets changed.

## Errors, inputs and comments

- **[VALIDATE-AT-BOUNDARIES]** Every piece of data that comes in (third-party API, source file,
  `localStorage`, URL parameter, command-line argument) is validated against a schema at run
  time before use: static types only protect what the compiler sees.
- **[EXCEPTIONS]** Never swallow an exception (`except: pass`, `catch {}`), never catch broadly
  (`except Exception`) without re-raising or logging it.
- **[AUDIT-REFERENCES]** A comment states the constraint in the present, never the history: no
  reference to an audit or a finding ("audit H36", "(CART-06)") in the code. History goes in the
  commit message (checked).

## Dependencies and tools

- **[UV-PODMAN]** Python goes through uv (`uv add`, `uv run`, `uv tool install`): never pip, pipx
  nor `requirements.txt`. Containers go through podman (`Containerfile`, `compose.yaml`,
  `podman-compose`): never docker, `Dockerfile` nor `docker-compose.yml` (checked).
- **[LOCKED-DEPENDENCIES]** Every dependency is resolved by a committed lockfile; CI installs it frozen and
  fails when it drifts.
- **[NEW-DEPENDENCY]** A new dependency is justified in its commit: maintained (a release less
  than a year old), compatible licence, no equivalent already there, no name close to a known
  package (typosquatting). Upgrades go through `ion repo update` (`--major` for major versions), one
  branch per pass; Dependabot only proposes security fixes.
- **[VULNERABILITY-AUDIT]** CI audits the dependencies at each commit; a high or critical
  vulnerability blocks the merge unless a dated, justified waiver covers it.

## Working with AI

- **[AI-ACCOUNTABILITY]** Code written by an agent follows the same charter and the same review
  as human code; whoever merges it answers for it.
- **[AI-CONTEXT]** These rules are the agents' reference: a rule an agent keeps breaking is a
  rule badly written, to be rephrased where it lives (`/repo:report` for ion-repo's rules).
- **[AI-CONFIDENTIALITY]** No secret and no real personal data is pasted into a conversation with
  an agent; work on test or public data.
- **[AI-FEATURES]** A feature that calls a language model goes through a queue, caps its cost per call, logs the model and the tokens used,
  and never lets generated text trigger an action without validation (prompt injection).
