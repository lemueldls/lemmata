//! Expands a reader dump and prints it, for reading a definition by hand.
//!
//! The port is a file at a time: read the `.ptl`, then read what its macros
//! mean. `patel-c --dump-ast` gives the reader's output, and this prints the
//! expansion with one argument per line, which is the form a port follows.

use std::path::Path;

use lemmata_ptl_front::{Sexpr, expand, parse};

/// One form per line, nested by two spaces.
#[must_use]
pub fn render(node: &Sexpr) -> String {
    let mut out = String::new();
    write(node, 0, &mut out);

    out
}

fn write(node: &Sexpr, depth: usize, out: &mut String) {
    let flat = node.to_string();
    let short = flat.len() + depth * 2 <= 76;
    let Sexpr::List(items) = node else {
        out.push_str(&flat);
        return;
    };
    if short || items.is_empty() {
        out.push_str(&flat);
        return;
    }
    // A computed call has no name, and its callee is the first item.
    let (head, rest) = match node.form() {
        Some(head) => (head, &items[1..]),
        None => ("<call>", &items[..]),
    };
    out.push('[');
    out.push_str(head);
    for item in rest {
        out.push('\n');
        out.push_str(&"  ".repeat(depth + 1));
        write(item, depth + 1, out);
    }
    out.push(']');
}

pub fn run(path: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(Path::new(path))
        .map_err(|error| format!("reading {path}: {error}"))?;
    let program = parse(&text).map_err(|error| format!("{path}: {error}"))?;
    let expanded = expand(&program).map_err(|error| format!("{path}: {error}"))?;
    println!("{}", render(&expanded));

    Ok(())
}
