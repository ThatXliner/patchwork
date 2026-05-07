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

## Motivation

Most code transformation tools sit at extremes. Regex-based tools (`sed`) are fragile across multi-line patterns and nested syntax. Full-featured linters (`semgrep`, `ast-grep`) require config files, YAML rules, or heavy runtimes — overkill for the common case of "find this pattern, change it."

**patchwork** is for the middle ground: structural search and replace that runs as a single `find | xargs` pipeline. Zero config, zero setup, one 3MB binary.

## How it compares

[ast-grep](https://github.com/ast-grep/ast-grep) is more mature, supports more languages, has YAML rules, LSP, VS Code extension, MCP server, playground, pre-commit hooks, language bindings — the works. patchwork doesn't try to catch up on that axis. They go by different design philosophies.

| Tool | Language | Parsing | Size | Languages | Maturity |
|---|---|---|---|---|---|
| **patchwork** | Rust | tree-sitter AST | ~3MB | 5 | Alpha — one person, <1yr |
| [ast-grep](https://github.com/ast-grep/ast-grep) | Rust | tree-sitter AST | ~10MB | 25+ | Mature — 174 releases, active community |
| [Comby](https://comby.dev) | OCaml | parser-free | ~8MB | ~all | Mature |
| [Semgrep](https://github.com/semgrep/semgrep) | Python | real parsers | 200MB+ | 20+ | Mature — enterprise security product |

**patchwork is built for scripts.** Default output is intentionally boring (just `file:line:col`) because it's trivial to `cut`, `xargs`, or pipe into other tools. No config files, no YAML, no rule system — five flat commands with sed-like flags. If the only thing between your `find` and your edit is a tool that needs a config file, you reach for something else.

**patchwork aims for more expressive matching.** The `$($name)sep*` repetition syntax (Rust-style, separator-aware) is the first step. We have `$BODY`/`$STMT`/`$EXPR` special tokens, a dedicated `insert-before`/`insert-after` command, and more matching logic planned — things that go beyond what a YAML rule system enables.

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
- **Formatting** — replacement text isn't auto-indented; include your own whitespace. This also means we don't really support Python
- **`$$$name` in blocks** — `$$$name` doesn't match statements inside blocks due to tree-sitter's `expression_statement` wrappers. Use `$BODY` instead
- **No model** — this is by design. Complex structural changes that need reasoning aren't supported. For those, use an LLM-based tool

## Status

Single 3MB binary, zero dependencies, zero configuration. Works with pipes, files, and shell globs.
