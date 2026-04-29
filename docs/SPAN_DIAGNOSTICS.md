Span Diagnostics and Propagation

- What we track: spans from tokenizers through AST and into HIR to enable precise error messages and IDE tooling.
- Current state: span fields added to AST nodes (FnDecl, LetDecl, Param, LetStmt, ReturnStmt, Block). LBrace spans propagate to function bodies and inline blocks. Tests cover multiline blocks and nested blocks.
- Future plans: propagate spans deeper into expressions and statements (Block-level spans at every level), and surface spans in all HIR nodes for end-to-end traceability.

Usage notes:
- Run tests to verify spans propagate as code is parsed.
- Use span information in diagnostics to report exact token locations.
