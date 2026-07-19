use tree_sitter::Parser;

fn main()
{
    // The tree-sitter parser bridge must not panic on arbitrary bytes, and the
    // produced tree must be fully walkable (`to_sexp` traverses every node).
    afl::fuzz!(|data: &[u8]| {
        let mut parser = Parser::new();
        if parser
            .set_language(&gandr_tree_sitter::language::gandr())
            .is_err()
        {
            return;
        }
        if let Some(tree) = parser.parse(data, None) {
            let _ = tree.root_node().to_sexp();
        }
    });
}
