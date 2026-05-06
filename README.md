# patchwork

**AST-native sed — find, replace, delete, and insert code by structure, not regex.**

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

| Tool | Language | Parsing | Size | Languages | Best for |
|---|---|---|---|---|---|
| **patchwork** | Rust | tree-sitter AST | ~3MB | 5 | Simple CLI refactors, pipes, `$()*` repetition |
| [ast-grep](https://github.com/ast-grep/ast-grep) | Rust | tree-sitter AST | ~10MB | 25+ | General refactoring, YAML rules, LSP/VS Code/MCP |
| [Comby](https://comby.dev) | OCaml | parser-free | ~8MB | ~all | Quick cross-language/text-format replacements |
| [Semgrep](https://github.com/semgrep/semgrep) | Python | real parsers | 200MB+ | 20+ | Security auditing with taint tracking & dataflow |

**patchwork vs ast-grep** (the closest alternative):
- patchwork is purely CLI — no YAML rules, no interactive mode, no LSP. A drop-in for `sed` in shell scripts.
- patchwork's `$($name)sep*` / `$($name)sep+` / `$($name)sep?` Rust-style repetition is unique here.
- patchwork uses `$lowercase` placeholders (not `$CAPITAL`), which some find more natural.
- ast-grep has a richer ecosystem — VS Code extension, MCP server, Python/Node bindings — if you need those.

**patchwork vs Comby**: patchwork uses real tree-sitter parsers so it understands language-specific AST structure. Comby is parser-free and works on any language, but with less structural precision.

**patchwork vs Semgrep**: Semgrep is heavy but delivers deep semantic analysis (taint tracking, dataflow). patchwork is for the 80% case: quick, correct structural edits that finish before your coffee gets cold.

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
- **No model** — this is by design. Complex structural changes that need reasoning aren't supported. For those, use an LLM-based tool

## Status

Single 3MB binary, zero dependencies, zero configuration. Works with pipes, files, and shell globs.
