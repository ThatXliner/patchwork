# patchwork

**AST-aware code editing with tree-sitter — like `sed` but it understands syntax.**

```bash
# Match by structure, not regex
patchwork find -p 'return null;' src/

# Replace matched code
patchwork replace -i -p 'old_func($a, $b)' -r 'new_func($b, $a)' src/*.java

# Delete matched expressions
patchwork delete -i -p 'debug($msg)' app.py

# Insert relative to structural matches
patchwork insert-after -p 'System.out.println("done");' --code '\nlogger.info("complete");' App.java

# Or use tree-sitter queries for precise matching
patchwork find -q '(function_definition name: (identifier) @name)' src/*.py
```

## Why

- **`sed`/`grep`** are line-based and break on multi-line statements, nested brackets, or string literals containing your pattern.
- **`semgrep`** is a 200MB+ Python dependency designed for CI scans, not ad-hoc CLI piping.
- **`fastedit`** is built for AI agents as an MCP server — it uses a 1.7B model for non-trivial edits.

**patchwork** is a single deterministic binary. It parses both your pattern and source code into tree-sitter CSTs, then finds structural matches and applies edits. No model calls, no configuration files, no magic.

## How it works

Write the code as a snippet (`-p`). It gets parsed into an AST subtree and matched against the source.

**Names and values match exactly by default.** `return 1;` only matches `return 1;`, not `return 42;`. Use `$` prefix (like `$x`) to match any value at that position:

```bash
# Match specific return value
patchwork find -p 'return null;' src/

# Match any return value
patchwork find -p 'return $val;' src/
```

`$name` acts as a placeholder that matches any single AST node of any type — identifiers, literals, expressions.

```bash
# Match any call to any method with value 42
patchwork replace -i -p 'println(42)' -r 'log(42)' src/*.java

# Match any call to println with any argument
patchwork replace -i -p 'println($arg)' -r 'log($arg)' src/*.java

# Match any method call with any args
patchwork delete -i -p '$method($args)' App.java
```

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
# Pipe mode (requires --language with stdin)
cat Main.java | patchwork find -l java -p 'return null;'

# File mode (language detected from extension)
patchwork replace -i -p 'catch (Exception $e)' -r 'catch (Exception $e) { log($e); throw; }' src/*.java

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

Early but functional. 63 tests, one binary, zero config.
