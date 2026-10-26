# Rejected

Three kinds, not to be confused.

## Ruled out for good

Final decisions: do not propose them again, even if an audit recommends it.

- **2026-10-26** · The parts of Copier kiln does not use (the founding plan): interactive
  prompts (kiln asks its own questions); everything `unsafe=False` already forbids (`_tasks`,
  `_migrations`, `_jinja_extensions`, `_external_data`); revision resolution (PEP 440 tags,
  `vcs_ref`, `use_prereleases`: kiln picks the revision); configurable `_subdirectory`,
  `_templates_suffix`, `_answers_file`, `_envops`, `_exclude`, `_min_copier_version` (fixed to
  `template/`, `.jinja`, `.copier-answers.yml`); `.rej` conflicts, recopy, pretend, Copier's
  CLI, its settings file and trust list, several answers files, `secret` and `multiselect`
  questions.

## Declined for now

## Dropped
