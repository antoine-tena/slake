---
paths:
  - "**/Containerfile"
  - "compose.yaml"
  - "config/**"
  - "scripts/**"
  - ".github/**"
  - "docs/operations/**"
---
<!-- Written by ion-repo and kept current by `ion repo update`: do not edit, it would be overwritten. -->
# Delivery and operations

- **[FAST-CI]** The full CI runs in under 10 minutes; a flaky test is quarantined and fixed within
  the week, never re-run until it passes.
