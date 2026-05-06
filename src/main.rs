use tree_sitter::{InputEdit, Language, Node, Parser, Point, Tree};
fn nodes_are_approximately_equal(node1: Node, node2: Node) -> bool {
    // TODO: implement the approximately part of this function
    println!(
        "Comparing node kind '{}' with '{}'",
        node1.kind(),
        node2.kind()
    );
    for child in node1.children(&mut node1.walk()) {
        if child.kind() == node2.kind() {
            if node2.child_count() == 0 {
                return true;
            }
            if nodes_are_approximately_equal(child, node2.child(0).unwrap()) {
                return true;
            }
        }
    }
    node1.kind() == node2.kind()
}
fn match_node(root_node1: Node, root_node2: Node, tree1: &Tree) -> Option<(usize, usize)> {
    // TODO: making multiple cursors like this is very inefficent
    // when we could instead by using cursor.node(), cursor.goto_first_child(),
    // cursor.goto_next_sibling(), etc.
    for child1 in root_node1.children(&mut tree1.walk()) {
        if nodes_are_approximately_equal(child1, root_node2) {
            //     "Matched node kind '{}' at position {:?} in root_node1 with position {:?} in root_node2",
            //     child1.kind(),
            //     child1.range(),
            //    root_node2.range()
            // );
            // println!(
            //     "It ends at the {:?} row {:?} column",
            //     child1.end_position().row,
            //     child1.end_position().column
            // );
            // return;
            return Some((child1.end_position().row, child1.end_position().column));
        }
        if let Some(output) = match_node(child1, root_node2, tree1) {
            return Some(output);
        }
    }
    return None;
}
fn patch(source: &str, context: &str, insertion: &str) -> Result<String, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .expect("Error loading Java grammar");
    let tree = parser.parse(source, None).unwrap();
    let root_node = tree.root_node();
    println!("Root node: {:?}", root_node.to_sexp());
    let secondary_tree = parser.parse(context, None).unwrap();
    let secondary_root_node = secondary_tree.root_node().child(0).unwrap();
    println!("To match: {:?}", secondary_root_node.to_sexp());
    // For modifying existing code, see https://github.com/tree-sitter/tree-sitter/discussions/2553#discussioncomment-9976343
    if let Some((line, column)) = match_node(root_node, secondary_root_node, &tree) {
        println!("Found the context at line {} column {}", line, column);
        let mut lines: Vec<String> = source.lines().map(|x| x.to_owned()).collect();
        if line < lines.len() {
            let target_line = &lines[line];
            {
                let (prefix, suffix) = target_line.split_at(column - 1);
                let new_line = format!("{}{}{}", prefix, insertion, suffix);
                lines[line] = new_line;
            }
        }
        let patched_source = lines.join("\n");
        return Ok(patched_source);
    }
    return Err("Could not find the context in the source".to_string());
}
fn main() {
    println!(
        "{}",
        patch(
            r#"public static final class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}"#,
            r#"System.out.println("Hello, World!");"#,
            "\nSystem.out.println(\"Bye, World!\");"
        )
        .unwrap()
    );
}
