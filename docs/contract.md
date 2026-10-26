# The contract between kiln and slake

kiln runs `slake <command>`, writes one JSON request on its standard input, and reads one JSON
response on its standard output. slake writes its logs on standard error, never on standard
output. This is version 1 of the contract: every request and every response carries
`"contract": 1`, and a change that would break kiln takes a new number.

slake renders a [Copier](https://copier.readthedocs.io) template the way Copier 9 does, byte
for byte, for the part of Copier kiln uses: same files, same modes, same `.copier-answers.yml`.
Every answers file Copier wrote is read as it is, and `copier.yml` keeps its format.

## Common to every command

### The template

```json
{"url": "https://github.com/antoine-tena/kiln.git", "ref": "v0.4.9"}
```

- `url`: a git repository, remote or local. slake keeps a bare mirror of it in its cache and
  reads every revision from there; it fetches at most once per command, and only when `ref` is
  not in the mirror yet.
- `ref`: what kiln picked (a tag, a `git describe` name, a branch or a SHA). It is recorded
  verbatim as `_commit` in the answers file, as Copier records it.

Or a template directory read as it is on disk, uncommitted changes included (kiln's own tests,
template development); `commit`, optional, is what to record as `_commit`:

```json
{"path": "/home/antoine/code/kiln", "commit": "v0.4.9"}
```

The template is the directory's `copier.yml` and its `template/` subdirectory, files ending in
`.jinja` rendered, macros imported from the root (`macros.jinja`).

### Answers

- `answers`: the answers given, Copier's `data`. They win over everything, are cast to their
  question's `type` and checked by its `validator`.
- `defaults`: the user's defaults, Copier's `user_defaults`: they replace a question's own
  default.
- A question nobody answered takes its default, rendered with the answers before it. A question
  whose `when` is false is not recorded in the answers file; its value still reaches the
  templates.

### Optional fields

- `cache`: the mirror's directory; by default `$XDG_CACHE_HOME/slake`, else `~/.cache/slake`.
- `offline`: `true` never fetches; a `ref` missing from the mirror is then an error.

### Responses and errors

Every response carries `"contract": 1` and `"slake"`, slake's version. The exit code is 0 when
the command did its work, conflicts included: they are in the report. Otherwise it is 1, and the
response is an error:

```json
{"contract": 1, "slake": "0.1.0",
 "error": {"kind": "answers", "message": "invalid answers",
           "questions": {"name": "Lowercase letters, digits and dashes only, starting with a letter."}}}
```

`kind` is one of `request` (malformed request), `template` (an invalid `copier.yml` or template
file), `ref` (revision not found), `network` (the fetch failed), `answers` (`questions` maps
each refused answer to its validator's message), `project` (not a git work tree, no answers
file, uncommitted changes), `io`.

## `slake render`

Renders a revision into a new directory: `kiln new`, and kiln's tests.

Request: `template`, `answers`, `defaults`, and `destination`, a directory that must be empty or
missing.

```json
{"contract": 1, "slake": "0.1.0",
 "commit": "v0.4.9", "sha": "f0ab621…",
 "answers": {"name": "demo", "backend": "django", "frontend": "nuxt"},
 "files": [".copier-answers.yml", "CLAUDE.md", "backend/manage.py"]}
```

`commit` is the recorded `_commit` (absent for a directory without one), `sha` the template's
commit (absent for a directory that is not a git checkout), `answers` the recorded answers
(without the `_` keys), `files` every file written, sorted.

## `slake update`

Updates a project to another revision, or the same one with changed answers: `kiln update`,
`kiln set`.

Request: `project` (the repo's root, where `.copier-answers.yml` is), `template` (`ref` alone:
the URL comes from the answers' `_src_path`; or a full template to replace it), `changes`
(answers that change), `defaults`, and `exclude`, gitignore-style patterns slake must not touch
(the files frozen by `kiln-waivers.toml`; the answers' `kept` is added by slake).

The project must be a git work tree without uncommitted changes. Then:

1. The old revision (the answers' `_commit`) is rendered with the recorded answers, under the
   recorded `_commit`: a promoted tag on the same commit as a candidate no longer renders a
   different name, so no spurious conflict at a promotion. The new revision is rendered with the
   recorded answers and the `changes`. Each once.
2. Each file is merged three ways, `git merge-file`: base the old render, ours the project,
   theirs the new render. A file only the template changed takes the new render; a file only
   the project changed stays as it is.
3. A file of the old render that the project deleted stays deleted, unless `_skip_if_exists`
   lists it (recreated when missing). A file `_skip_if_exists` lists is never touched once it
   exists. A file only in the old render is deleted, then the directories left empty.
4. A conflict is left inline, with Copier's labels (`before updating`, `last update`,
   `after updating`), and recorded in the index like Copier does: stage 1 the old render, 2 the
   project, 3 the new render.
5. The answers file is written again.

```json
{"contract": 1, "slake": "0.1.0",
 "from": {"commit": "v0.4.8", "sha": "…"},
 "to": {"commit": "v0.4.9", "sha": "…"},
 "answers": {"name": "demo"},
 "added": ["docs/backlog/urgent.md"],
 "updated": [".github/workflows/ci.yml", "charter.toml"],
 "deleted": ["scripts/old.sh"],
 "conflicted": ["charter.toml"],
 "excluded": [{"path": ".pre-commit-config.yaml", "reason": "frozen",
               "diff": "--- before\n+++ after\n@@ …"}]}
```

`updated` holds every file the update changed, merged or not; `conflicted` the ones left with
markers (also in `updated` or `added`); `excluded` each excluded path whose render changed
between the two revisions (`reason`: `frozen`, from `exclude`, or `kept`), with the unified diff
of the two renders.

## `slake adopt`

Brings a repo kiln did not generate under kiln: `kiln adopt`.

Request: `project`, `template`, `answers` (what kiln inferred from the repo), `defaults`, and
`keep`, the repo's own code folders (`["backend"]`).

slake renders the template, keeps `keep` and every rendered file the repo already has (the
answers file excepted), records that list as the `kept` answer, and lays every other file. It
never overwrites one.

```json
{"contract": 1, "slake": "0.1.0", "commit": "v0.4.9", "sha": "…",
 "answers": {"name": "demo", "kept": [".gitignore", "README.md", "backend"]},
 "laid": ["CLAUDE.md", "charter.toml"],
 "kept": [{"path": ".gitignore", "template": "/target\n.env\n…"},
          {"path": "backend", "folder": true}]}
```

`template` is what the template would have written in a kept text file, for kiln to compare or
merge (the template's `.gitignore` lines).

## `slake questions`

The questionnaire, without asking anything: `kiln new --all-questions` asks it itself.

Request: `template`, `answers` (given so far), `defaults`.

```json
{"contract": 1, "slake": "0.1.0",
 "questions": [
   {"key": "size", "type": "str", "help": "How hot does this one burn?",
    "choices": [{"label": "Spark: a script, a personal tool, a one-off analysis", "value": "spark"}],
    "default": "ember", "when": true, "answered": false, "error": null}]}
```

Every question of `copier.yml` in order, with its `help`, `choices` and `default` rendered from
the answers given, its `when` evaluated, whether it is answered, and its validator's message for
the given answer (`null` when valid). kiln asks the first question with `when` true that is not
answered, then calls again with that answer. Types: `str`, `bool`, `int`, `float`, `json`,
`yaml`.

## What slake does not do

What kiln does not use is not built: see `docs/backlog/rejected.md`.
