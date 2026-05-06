# patchwork

**AST-native sed — find, replace, delete, and insert code by structure, not regex.**

## Installation

```bash
cargo install patchwork-cli
```


## Usage

```bash
# Rename a method across files without false positives
patchwork replace -i -p 'getOldData($a)' -r 'getData($a)' src/**/*.java

# Replace a logging framework
patchwork delete -i -p 'logger.debug($msg)' src/*.py
patchwork insert-before -p 'logger.debug($msg)' --code 'tracing.debug($msg)' src/*.py

# Match by structure, not regex
patchwork find -p 'return null;' src/
```

More examples are available in the [examples](examples/README.md) directory.

## The problem

You want to rename a function, swap an import, or update an API call across a codebase. Your options:

- **`sed`** — the regex might match inside strings or comments, misses multi-line patterns, and breaks on nested brackets. Gets fragile fast.
- **`semgrep`** — a 200MB+ Python install, designed for CI linting, not for piping through `find | xargs`.

**patchwork** is a single 3MB binary that parses both your pattern and source into tree-sitter CSTs, finds structural matches, and applies edits. No models, no config, no Python runtime.

## How it compares

Let's be honest: patchwork is essentially a minimal subset of [ast-grep](https://github.com/ast-grep/ast-grep). Both use tree-sitter to parse code into ASTs, both match patterns structurally, both rewrite in-place. ast-grep came first, supports 25+ languages, has a YAML rule system, VS Code extension, LSP, MCP server, interactive mode, playground, pre-commit hooks, Node/Python bindings, and 174+ releases. patchwork is a few months old, supports 5 languages, and does one thing: CLI find/replace/delete/insert with a sed-like interface.

Does that mean patchwork is pointless?

| Tool | Language | Parsing | Size | Languages | Maturity |
|---|---|---|---|---|---|
| **patchwork** | Rust | tree-sitter AST | ~3MB | 5 | Alpha — one person, <1yr |
| [ast-grep](https://github.com/ast-grep/ast-grep) | Rust | tree-sitter AST | ~10MB | 25+ | Mature — 174 releases, active community |
| [Comby](https://comby.dev) | OCaml | parser-free | ~8MB | ~all | Mature |
| [Semgrep](https://github.com/semgrep/semgrep) | Python | real parsers | 200MB+ | 20+ | Mature — enterprise security product |

### Honest comparison with ast-grep

**What ast-grep has that patchwork doesn't:**
- 25+ languages vs 5
- YAML rule system (composable `inside`/`has`/`follows`/`precedes`/`not`/`any`/`all` rules)
- `transform` system (regex substitution, case conversion, substring slicing on captured vars)
- `FixConfig` for clean list-item deletion (expand start/end to remove surrounding commas)
- VS Code extension, interactive mode, `ast-grep scan` for project-wide linting
- LSP server (go-to-definition, hover, diagnostics via structural rules)
- MCP server (Claude Code, Cursor, and other AI tools can drive ast-grep directly)
- Node.js and Python bindings
- Pre-commit hook support
- Online playground, Codemod Studio
- Stricter matching control: `strictness` levels (`cst`, `smart`, `ast`, `relaxed`, `signature`)
- Same-name `$VAR` backreferences (`$A == $A` matches `a == a` but not `a == b`)
- `$$VAR` for unnamed/anonymous node capture
- Non-capturing variables (`$_VAR`) for performance

**What patchwork has that ast-grep doesn't:**
- The `$($name)sep*` / `$($name)sep+` / `$($name)sep?` Rust-style repetition syntax (ast-grep uses `$$$NAME` which doesn't capture the separator) — this is genuinely novel
- `$$$name` multi-node repetition at any position (ast-grep compatible)
- Special tokens: `$BODY` (statement-aware block matching), `$STMT` (any statement), `$EXPR` (any expression)
- A dedicated `insert-before` / `insert-after` command — useful for wrapping or logging instrumentation
- Slightly smaller binary (~3MB vs ~10MB)
- No config files, no YAML, no subcommands beyond the 5 operations — just `patchwork find|replace|delete|insert-before|insert-after`
- `$lowercase` placeholder convention (some prefer this; it's a minor stylistic difference from ast-grep's `$UPPERCASE`)

**What both have:**
- tree-sitter-based AST matching
- `$name` single-node wildcards
- Multi-node repetition (patchwork: `$($name)sep*`, ast-grep: `$$$NAME`)
- Tree-sitter query mode (`-q` in patchwork, `--query` in ast-grep)
- `-i` in-place editing
- Pipe/stdin support
- CLI-focused design (both are fast Rust binaries)

### So why does patchwork exist?

Two reasons:

1. **The `$($name)sep*` repetition is genuinely better for function arguments.** `$f($($arg,)*)` captures the separator and handles zero/one/many args with a natural Rust-like syntax. ast-grep's `console.log($$$ARGS)` has no separator awareness — if you delete a single arg, you're left with trailing commas.

2. **Vision: a tool that AI agents reach for first.** patchwork aims to be the simplest possible AST editor — so simple that an LLM can generate precise patchwork commands without thinking about YAML config, rule composition, or scanning modes. Whether it achieves this better than `ast-grep -p '...' -r '...'` is an open question, but the design surface is intentionally tiny.

Realistically: if you need a mature, well-documented tool today, use ast-grep. If you're interested in the repetition syntax experiment or want to influence the design of a simpler alternative, watch this space.

## How it works

Write a code snippet and patchwork finds structurally identical code in your source.

**Names and values match exactly by default** — `return 1;` only matches `return 1;`, not `return 42;`. This means you don't accidentally match code that happens to have the same shape but different identifiers.

```bash
# Only matches exactly this call
patchwork find -p 'old_api(x)' src/

# Use $ to match any identifier
patchwork find -p 'old_api($x)' src/
```

### `$` placeholders

`$name` matches any single AST node — any identifier, literal, or expression. It's like `.*` for code but structure-aware.

```bash
# Match any call to old_api with any single argument
patchwork replace -i -p 'old_api($arg)' -r 'new_api($arg)' src/**/*.java

# Match any two-argument call, reorder args in replacement
patchwork replace -i -p '$f($a, $b)' -r '$f($b, $a)' src/*.py

# Delete all calls to debug regardless of argument
patchwork delete -i -p 'debug($msg)' src/*.py

# Match any return statement
patchwork find -p 'return $val;' src/
```

### Multi-node repetition

Match zero or more consecutive children at any position:

```bash
# Match any method call with any number of arguments
patchwork find -p '$fn($$$args);' src/

# Match all method calls on users (size(), get(id), remove(id))
patchwork find -p 'users.$$$($($arg,)*);' src/
```

`$($name)sep*` matches repetitions at the last child position (Rust macro syntax). `$$$name` matches at any position (ast-grep compatible).

### Special tokens

Pre-defined shortcuts for common patterns:

| Token | Matches | Example |
|-------|---------|---------|
| `$BODY` | Zero or more statements inside a block | `if ($EXPR) { $BODY }` |
| `$STMT` | A single statement of any kind | `$STMT` matches `return 42;`, `if (x) {}`, etc. |
| `$EXPR` | A single expression | `debug($EXPR);` matches `debug(x)`, `debug(f())` |

`$BODY` is statement-aware — it works where `$$$name` doesn't because tree-sitter wraps bare identifiers in `expression_statement` nodes inside blocks. Use it to match arbitrary if/while/for bodies:

```bash
# Find all if statements regardless of body
patchwork find -p 'if ($EXPR) { $BODY }' src/

# Replace all debug calls with log calls
patchwork replace -i -p 'debug($EXPR);' -r 'log($EXPR);' src/**/*.java
```

### Tree-sitter queries

For precise structural patterns with named captures:

```bash
patchwork replace -q '(if_statement condition: (identifier) @matched)' -r 'check(x)' file.ts
```

## Operations

| Command | Effect |
|---------|--------|
| `find` | Print `file:line:col` for each match |
| `replace` | Replace matched code with new code |
| `delete` | Remove matched code |
| `insert-before` | Insert code before each match |
| `insert-after` | Insert code after each match |

All editing commands support `-i` (in-place, like `sed -i`) and stdin/stdout piping.

## Usage

```bash
# Pipe mode (requires --language with stdin)
cat Main.java | patchwork find -l java -p 'return null;'

# File mode (language detected from extension)
patchwork replace -i -p 'println($arg)' -r 'log($arg)' src/**/*.java

# Query mode
patchwork find -q '(method_invocation name: (identifier) @name (#eq? @name "println"))' App.java

# Multi-file
patchwork replace -p 'BufferedReader $r' -r 'Reader $r' src/**/*.java
```

## Installation

```bash
cargo install patchwork
```

Or build from source:

```bash
git clone https://github.com/ThatXliner/patchwork
cd patchwork
cargo build --release
```

## Supported languages

Java, Python, JavaScript, TypeScript, TSX. Adding a language is one crate dependency and a handful of lines.

## Limitations

- **Single-file only** — no cross-file rename tracking or import updates
- **Formatting** — replacement text isn't auto-indented; include your own whitespace
- **`$$$name` in blocks** — `$$$name` doesn't match statements inside blocks due to tree-sitter's `expression_statement` wrappers. Use `$BODY` instead
- **No model** — this is by design. Complex structural changes that need reasoning aren't supported. For those, use an LLM-based tool

## Status

Single 3MB binary, zero dependencies, zero configuration. Works with pipes, files, and shell globs.
