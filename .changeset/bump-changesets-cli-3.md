---
"@googleworkspace/cli": patch
---

chore(deps): bump `@changesets/cli` (devDependency) from 2.29.8 to 3.0.1 to resolve 7 Dependabot alerts against transitive `js-yaml`/`picomatch` versions pulled in by `@changesets/cli`'s own dependency tree (dev-only, CI-only exposure — no untrusted input, no impact on the distributed binary or npm installer package). v3 changed the default behavior for `private: true` packages: `changeset version` now silently skips them unless `privatePackages.version` is explicitly enabled in `.changeset/config.json`, so that option is added here. No behavior change to the CLI itself.
