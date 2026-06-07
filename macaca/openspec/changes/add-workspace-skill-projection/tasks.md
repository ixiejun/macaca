## 1. Implementation

- [x] 1.1 Update the skill source set to scan Macaca-owned and common client skill roots in the requested priority order.
- [x] 1.2 Add source-location fields to skill snapshot entries so projected model paths do not erase audit provenance.
- [x] 1.3 Add a workspace projection step that copies visible skill directories into `available_skills/<stable-slug>/`.
- [x] 1.4 Render projected locations in `<available_skills>` and preserve source paths for path-policy checks.
- [x] 1.5 Add tests for discovery precedence and projected skill materialization.
- [x] 1.6 Run OpenSpec, targeted crate tests, cargo check, and GitNexus change detection.
