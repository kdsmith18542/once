Consolidated Patch: Task 6-12 (Span depth, ImportResolver patterns, nested-span tests, compliance matrix, DX docs)

What this patch covers
- Task 6: Span depth polish
- Task 7: ImportResolver patterns (broader patterns) with tests
- Task 8: Deeper nested-span tests
- Task 9: Compliance matrix (per-spec tracking)
- Task 9 (DX): Span diagnostics docs expansion
- Task 10-12: Lays groundwork for future patches (span depth continuation, broader import patterns, deeper nested-span tests, compliance matrix, DX docs) with suggested follow-ups

Summary of changes by area
- Parser/AST (once-parse):
  - Block now has span: Option<Span>
  - FnDecl, LetDecl, Param, LetStmt, ReturnStmt all gain span fields
  - Inline blocks inside expressions carry span; function bodies capture LBrace span
  - Added tests to cover span depth under multiline and nested blocks

- HIR wiring (once-hir):
  - HirBlock now has span: Option<(usize, usize)>
  - HirFnDecl, HirLetDecl, HirLetStmt, HirReturnStmt gain span
  - ImportResolver integration remains a no-op in behavior but now includes deeper path handling and normalization for imports; tests cover noop, basic, relative, and named imports
  - Tests added for named imports (test_import_resolver_named_imports)

- ImportResolver (enhanced, local):
  - Normalize relative paths, Windows separators, and collapse simple relative segments
  - Default empty imports to wildcard, add prelude for std/core with wildcard
  - Added test coverage for named imports and relative path handling

- DX and docs: span diagnostics
  - docs/SPAN_DIAGNOSTICS.md added with current approach and future plans
  - NEXT_STEPS.md updated with Compliance Snapshot and Week 1-2 focus

How to verify locally (commands)
- Ensure you’re in the local workspace:
  - cd G:\BACKUP\once
- Run tests:
  - cargo test -p once-parse
  - cargo test -p once-hir
- Review new tests:
  - test_import_resolver_named_imports (in Hir tests)
  - Deep-nested span tests in parse tests

Notes and next steps
- Next patch sequence (Task 10–12) will continue the plan:
  - Task 10: Span depth continuation (further normalization and propagation)
  - Task 11: Broaden ImportResolver patterns (more import shapes, more tests)
  - Task 12: Compliance matrix refinement and DX expansion

Owner note: This consolidation is for review in your local environment. No remote pushes were performed.
