# patchwork

**Structural code editing with tree-sitter — like `sed` but it understands syntax.**

```bash
# Match by structure, not regex
patchwork find -p 'return null;' src/

# Replace structurally equivalent code
patchwork replace -i -p 'old_func(a, b)' -r 'new_func(b, a)' src/*.java

# Delete matched expressions
patchwork delete -i -p 'debug(message)' app.py

# Insert relative to structural matches
patchwork insert-after -p 'System.out.println("done");' --code '\nlogger.info("complete");' App.java

# Or use tree-sitter queries for precise matching
patchwork find -q '(function_definition name: (identifier) @name)' src/*.py
```

## Why

- **`sed`/`grep`** are line-based and break on multi-line statements, nested brackets, or string literals containing your pattern.
- **`semgrep`** is a 200MB+ Python dependency designed for CI scans, not ad-hoc CLI piping.
- **`fastedit`** is built for AI agents as an MCP server — it uses a 1.7B model for non-trivial edits and operates at the symbol level, not arbitrary nodes.

**patchwork** is a single deterministic binary. It parses both your pattern and source code into tree-sitter CSTs (concrete syntax trees), then finds structural matches and applies edits. No model calls, no configuration files, no magic.

## How it works

There are two matching modes:

### Snippet matching (`-p`)

Write the code you're looking for. It gets parsed into an AST subtree and matched structurally against the source — same tree shape, same node kinds, but leaf values (identifiers, literals) match by type only. `return 1;` matches `return 42;` because the structure (`return_statement → integer_literal`) is identical.

```bash
# Matches any return with an integer literal
patchwork find -p 'return 1;' file.java
```

For name-aware matching, use tree-sitter queries. Snippet matching is purely structural — `old_func(a, b)` matches any two-argument call at that position.

### Tree-sitter queries (`-q`)

[Tree-sitter S-expression queries](https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax) for when you need precise structural patterns with named captures.

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

## Language support

Java, Python, JavaScript, TypeScript, and TSX — covered by `tree-sitter-java`, `tree-sitter-python`, `tree-sitter-javascript`, and `tree-sitter-typescript`. Adding a language means adding one crate dependency and one enum arm.

## Usage

```bash
# Pipe mode (auto-detection requires --language)
cat Main.java | patchwork find -l java -p 'return null;'

# File mode (language detected from extension)
patchwork replace -i -p 'catch (Exception e)' -r 'catch (Exception e) { log(e); throw; }' src/*.java

# Query mode
patchwork find -q '(method_invocation name: (identifier) @name (#eq? @name "println"))' App.java

# Multi-file with filename headers
patchwork replace -p 'TODO' -r 'FIXME' src/*.rs
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

## How is this different from fastedit?

Fastedit is an MCP server designed for AI coding agents. Its deterministic mode operates at the **symbol level** (find function `foo`, then line-splice within its body). Complex edits fall back to a 1.7B model. It's not designed for CLI composability — you can't pipe to it or use it in `find | xargs` workflows.

Patchwork is a CLI tool designed for **you**, not an agent. It matches arbitrary AST nodes (not just named symbols), supports stdin/stdout pipelines, and is fully deterministic. There is no model.

## Status

Early but functional. 14 tests, one binary, zero config.
