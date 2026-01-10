> **First:** Read `CLAUDE.md` (project instructions) — you are a **worker**. Start in **plan mode** and propose your implementation before coding.

# Task: Scope Weblate Integration for Translations

## Objective

Research and document implementation plan for integrating Weblate to enable community translations of the Scanopy UI.

## Background

Weblate is an open-source web-based translation management system. It can:
- Host translation files and provide web UI for translators
- Sync with git repositories
- Support various i18n formats (JSON, PO, XLIFF, etc.)

## Research Questions

### 1. Current i18n State
- Does the UI currently have any i18n infrastructure?
- What framework is the frontend using (SvelteKit based on typical patterns)?
- Are there any existing translation files or string externalization?

### 2. i18n Library Selection
- What i18n libraries work well with the frontend framework?
- Options: svelte-i18n, typesafe-i18n, paraglide-js, etc.
- Considerations: type safety, bundle size, SSR support

### 3. String Extraction
- How many translatable strings exist approximately?
- What format should translation files use?
- Key naming conventions

### 4. Weblate Setup Options
- Self-hosted vs hosted.weblate.org
- Git integration approach (push/pull)
- Project structure in Weblate

### 5. Workflow
- How do new strings get added?
- How do translations get into the app?
- CI/CD integration
- Handling missing translations (fallback)

### 6. Scope Estimate
- Effort to add i18n infrastructure
- Effort to extract existing strings
- Ongoing maintenance burden

## Deliverables

1. **Current state assessment** - Existing i18n infrastructure (if any)
2. **Recommended approach** - i18n library, file format, workflow
3. **Weblate configuration** - How to set up project
4. **Implementation plan** - Steps to add i18n support
5. **Effort estimate** - Rough sizing (small/medium/large)

## Research Approach

1. Examine frontend codebase structure
2. Check for existing i18n/l10n code
3. Identify translatable strings (UI text, error messages, etc.)
4. Research i18n libraries compatible with framework
5. Review Weblate documentation for integration options

## Files to Examine

- `ui/src/` - Frontend source
- `ui/package.json` - Dependencies, scripts
- `ui/src/lib/` - Shared components (likely contain UI strings)
- Any existing locale/i18n directories

## Notes

- This is research/scoping, not implementation
- Focus on practical, maintainable approach
- Consider translator experience (Weblate UI quality)
- Document findings in TASK.md work summary

---

## Work Summary

### Current State Assessment

**Frontend Stack:** SvelteKit 2.43.5 + Svelte 5.0.0, Vite 7.1.3, static adapter

**i18n Infrastructure:** None exists
- No locale directories or translation files
- No i18n library installed
- All UI text hardcoded as English strings

**Codebase Metrics:**
- 110 Svelte component files
- 21 feature modules
- ~27,000 lines of Svelte code
- **600-800 estimated translatable strings**

**String Categories:**
- Form labels & placeholders: ~170 unique
- Validation/error messages: ~70
- Button labels & actions: ~270+ occurrences
- Toast notifications: ~50+
- Modal titles & headings: ~40+
- Empty states & help text: ~30+

### Recommended Approach

**i18n Library:** Paraglide JS
- Official Svelte CLI integration (first-party support)
- Compiler-based: generates tree-shakable message functions
- Full TypeScript support with type-safe message keys
- Small bundle impact (~1KB base + only used messages)

**File Format:** JSON with flat dot-notation keys
```json
{
  "common.save": "Save",
  "hosts.createHost": "Create Host",
  "validation.required": "This field is required"
}
```

**Key Naming:** `{namespace}.{camelCaseKey}`
- Namespaces: common, auth, hosts, networks, services, settings, validation, errors

### Weblate Configuration

**Hosting:** hosted.weblate.org (paid tier)

**Git Workflow:**
1. Developer adds strings → commits `en.json` → pushes to GitHub
2. Weblate pulls changes automatically (webhook)
3. Translators work in Weblate web UI
4. Weblate commits translations back to repo
5. CI builds with updated translations

**Project Structure:**
```
Project: Scanopy
└── Component: UI
    ├── Source: messages/en.json
    ├── File format: JSON
    ├── File mask: messages/*.json
    └── Languages: en (source), add others as community contributes
```

### Implementation Plan

**Phase 1: Infrastructure Setup**
- Install Paraglide JS and Vite plugin
- Create `project.inlang/` configuration
- Create `messages/en.json` with initial structure
- Add locale detection (browser → cookie → default)

Files to create: `ui/project.inlang/settings.json`, `ui/messages/en.json`, `ui/src/lib/i18n.ts`
Files to modify: `ui/vite.config.ts`, `ui/package.json`, `ui/src/routes/+layout.svelte`

**Phase 2: String Extraction (by priority)**
1. shared/components (~100 strings) - reused everywhere
2. auth (~50) - user-facing, critical
3. settings (~80) - user-facing
4. hosts (~100) - core feature
5. networks, services (~80) - core features
6. Remaining features (~400)

**Phase 3: Weblate Setup**
- Create project on hosted.weblate.org
- Connect GitHub repository
- Configure webhook for auto-sync
- Enable "Push on commit"

**Phase 4: CI/CD Integration**
- Add translation build step
- Validate message files (no missing keys)

### Decisions Made

- **Rollout:** Full string extraction before shipping (complete i18n coverage from day one)
- **Missing translations:** Fall back to English
- **Initial languages:** English only; add languages as community contributors volunteer

### Effort Estimate

| Task | Effort |
|------|--------|
| Paraglide setup | Small (~2 hours) |
| String extraction | Medium-Large (600-800 strings) |
| Weblate setup | Small (~1 hour) |
| CI integration | Small (~1 hour) |

**Total: Medium effort** - Infrastructure is straightforward; string extraction is the bulk of work but mechanical.

### Sources

- [Paraglide SvelteKit Documentation](https://inlang.com/m/dxnzrydw/paraglide-sveltekit-i18n/)
- [Svelte CLI Paraglide Docs](https://svelte.dev/docs/cli/paraglide)
- [Weblate Integration Guide](https://docs.weblate.org/en/latest/devel/integration.html)
- [Weblate Continuous Localization](https://docs.weblate.org/en/latest/admin/continuous.html)
