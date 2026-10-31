# Backlog

What is left to do on Slake, what is done, and what was turned down. **This folder is the
only up-to-date source.**

| File | Content |
|---|---|
| [backlog.md](backlog.md) | **Everything open**: the single list ordered by priority, one `T-NN` ticket per thing to do with its type, status and details |
| [done.md](done.md) | The closed tickets, one line each, most recent first |
| [rejected.md](rejected.md) | What was turned down, dropped, or ruled out for good |
| [urgent.md](urgent.md) | What only a human on the team can do (access, payment, decision), one checkbox per action |

An audit writes its dated report next to these files (`audit-<topic>-YYYY-MM-DD.md`) while it
runs, copies its open findings into [backlog.md](backlog.md) (with a pointer to their lines),
then moves to `docs/_archive/YYYY-MM-DD-topic/`. Archives **can be read, but everything in them
is old and must be checked** against the current code; they are never edited.

## How to use (humans and agents)

**A ticket** = a `### T-NN · Title` heading, then five fixed lines, then the details:

    ### T-NN · Short title that says what to achieve
    - **Type**: task
    - **Status**: todo
    - **Opened**: YYYY-MM-DD
    - **Effort**: S · **Owner**: … · **Due**: —
    - **Origin**: where it comes from · audit `docs/_archive/<folder>/README.md` l. 10-20

    Why, what to do (as a `- [ ]` checklist when there are several steps), where to look.

- **ID** `T-NN`: stable, never reused, quoted as is in commits and branches. The next free one
  is written at the top of [backlog.md](backlog.md) (`Next free: T-NN`).
- **Type**, one of: `bug` (something is broken); `improvement` (it works, it could work
  better); `idea` (nothing exists yet, to be explored); `task` (work that is neither: a release,
  a decision, a check).
- **Status**, one of: `todo`; `in progress — branch <name>, since YYYY-MM-DD`;
  `blocked — waiting for <who or what>`; `to verify — <where>`.
- **Priority** = the section the ticket lives in; changing it means moving the ticket.
  🔴 critical (now), 🟠 important (before the north star), 🟡 normal, ⚪ background
  (long-running work, debt, ideas still to be explored).
- **Effort**: XS (under an hour), S (half a day), M (one or two days), L (more).
- **Owner**: `—` (free, anyone may take it), a member's GitHub login (`@login`) or first name
  (the ticket is reserved for them), or `contributor — #NN` (kept for an outside contributor);
  never `Claude`: an agent is not an owner. An agent only works on a free ticket or one owned
  by the account it works for (`gh api user --jq .login`): it skips the others and, asked for
  one, says who owns it instead of doing it. **Each member writes only their own login**: taking a free ticket, or
  turning their own first name into their login; never someone else's (only the owner hands a
  ticket over).
- **Releases**: the `## 🚀 Releases` section at the top places every open ticket in a planned
  release (`/repo:backlog plan`); numbers are planned, not promised, until the candidate is cut.
- **Closing a ticket**: remove it from its section and add a line at the top of
  [done.md](done.md): `- **YYYY-MM-DD** · T-NN · Title — what was done (branch) · opened YYYY-MM-DD`.
- **Details in archived audits**: when a ticket quotes lines of `docs/_archive/`, the original
  finding is there in full; it is old, check it against the current code.
- **Locations quoted are hints, not addresses.**
- Overview: `grep -E '^(## |### T-|- \*\*Status)' docs/backlog/backlog.md`.
