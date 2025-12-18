//! JSON output formatting

use std::io;

use crate::tree::TreeNode;

/// Print tree node as pretty-printed JSON to stdout.
pub fn print_json(node: &TreeNode) -> io::Result<()> {
    let json =
        serde_json::to_string_pretty(node).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!("{}", json);
    Ok(())
}
