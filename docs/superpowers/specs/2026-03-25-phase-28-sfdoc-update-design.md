# Phase 28: `sfdoc update <file>` — Single-File Documentation Update

## Goal

Re-document a single file without scanning and rebuilding the entire project. Fast inner loop for developers actively editing Salesforce metadata.

## CLI Interface

```
sfdoc update <target> [options]
```

### Positional Argument

- `target` — a file path or a name

### Target Resolution

1. If `target` contains `/` or ends in a known extension (`.cls`, `.trigger`, `.flow-meta.xml`, `.object-meta.xml`, `.js-meta.xml`, `.flexipage-meta.xml`, `.md-meta.xml`, `.cmp`, `.validationRule-meta.xml`) → treat as **file path**, verify it exists.
2. Otherwise → treat as **name**: scan `--source-dir`, find the first file whose stem matches (case-insensitive).
3. If multiple files match a name (e.g., both an Apex class and a flow) → error listing the matches, ask user to specify the full path.
4. If no match found → error with Levenshtein-based suggestions (distance ≤ 2).

### Flags

Shared with `generate` (same defaults):

| Flag | Default | Notes |
|------|---------|-------|
| `--source-dir` | `force-app/main/default` | Where to scan for source files |
| `--output` / `-o` | `docs/` or `site/` | Output directory |
| `--provider` | `gemini` | AI provider |
| `--model` | Provider default | Model name |
| `--format` | Auto-detect, fallback `markdown` | Output format |
| `--verbose` / `-v` | `false` | Verbose logging |

**Not included** (not relevant for single-file update):
- `--concurrency`, `--rpm` — only one API call
- `--force` — `update` always regenerates
- `--type`, `--name-filter`, `--tag` — user already specified the file

## Pipeline

```
1. Validate preconditions
   ├─ Check output dir exists
   ├─ Check .sfdoc-cache.json exists → error if not
   └─ Load cache

2. Resolve target
   ├─ Path mode: verify file exists, determine metadata type from extension
   └─ Name mode: scan source-dir, find matching file, determine type

3. Parse
   └─ Call the appropriate parser for the metadata type

4. Hash
   └─ SHA-256 of source content (same logic as generate)

5. Generate documentation
   ├─ Build system_prompt + user_prompt for the metadata type
   ├─ Create AI client (provider + model)
   └─ Call send_request, deserialize response

6. Update cache
   └─ cache.update_*(key, hash, model, documentation)

7. Auto-detect format (if --format not specified)
   └─ Check for index.html → HTML, index.md → Markdown, else default markdown

8. Render updated page
   └─ Write single output file (e.g., classes/OrderService.md)

9. Rebuild index
   ├─ Load ALL docs from cache (all metadata types)
   ├─ Build AllNames for cross-linking
   └─ Regenerate full index file

10. Save cache
```

## Refactoring

Extract reusable functions from `main.rs` so both `generate` and `update` share the same code paths:

- `resolve_metadata_type(extension) -> MetadataType` — determine type from file extension
- `parse_single_file(source_file, metadata_type) -> ParsedItem` — enum over all metadata types
- `generate_single_doc(parsed_item, client) -> Documentation` — enum over all doc types
- `build_all_names(cache) -> AllNames` — build cross-linking index from cache
- `render_single_page(parsed_item, docs, all_names, format) -> String` — render one output page

The existing `generate` command calls these same functions in its loop, keeping behavior identical.

## Error Handling

| Condition | Message |
|-----------|---------|
| No cache exists | `Error: No existing documentation found in '{output_dir}'. Run 'sfdoc generate' first, then use 'sfdoc update' to refresh individual files.` |
| Target not found (name) | `Error: No source file matching '{name}' found in '{source_dir}'.` + Levenshtein suggestions if close matches exist (distance ≤ 2) |
| Target not found (path) | `Error: File not found: '{path}'` |
| Unsupported file type | `Error: Cannot determine metadata type for '{path}'. Supported extensions: .cls, .trigger, ...` |
| Ambiguous name match | `Error: '{name}' matches multiple files:` + list, ask user to specify full path |
| AI call fails | Same retry logic as `generate` (exponential backoff). Cache only saved on success. |

## Console Output

**Normal mode:**
```
Updating documentation for OrderService (Apex Class)...
✓ Documentation updated: classes/OrderService.md
✓ Index regenerated
```

**Verbose mode** additionally shows:
```
Resolved target: force-app/main/default/classes/OrderService.cls
Metadata type: Apex Class
Source hash: a1b2c3...
Provider: gemini (gemini-2.5-flash)
Format: markdown (auto-detected)
Cache loaded: 47 entries
Sending to AI provider...
Response received (1.2s)
Cache saved
```

**Exit codes:** `0` success, `1` error.

## Approach

**Thin wrapper reusing `generate` internals.** Extract pipeline steps from `main.rs` into reusable functions, then have `update` call the same functions scoped to a single file. This keeps behavior identical between commands, avoids duplication, and sets the stage for Phase 34 (watch mode) which also needs single-file pipeline invocation.

## Format Auto-Detection

When `--format` is not specified:
1. Check output directory for `index.html` → use HTML format
2. Check output directory for `index.md` → use Markdown format
3. Neither found → default to Markdown

`--format` flag overrides auto-detection when provided.
