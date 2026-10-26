---
paths:
  - "**/Containerfile"
  - "compose.yaml"
  - "config/**"
  - "scripts/**"
  - ".github/**"
  - "docs/operations/**"
---
<!-- Written by kiln and kept current by `kiln update`: do not edit, it would be overwritten. -->
# Delivery and operations

- **[FAST-CI]** The full CI runs in under 10 minutes; a flaky test is quarantined and fixed within
  the week, never re-run until it passes.
