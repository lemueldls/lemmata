//! What expansion owes the lower-case `a`.
//!
//! The file is the first translation target: it exercises the block forms, the
//! glyph procs, the saving forms and the anchors, and it has a hand port to
//! compare a translation against. These checks pin the parts of its expansion
//! that are easy to get subtly wrong: the imports a block keeps, the shape of a
//! saved glyph, and the tree an infix run resolves to.
//!
//! Skips without `LEMMATA_PTL_DIR`.

use lemmata_ptl_front::{Sexpr, expand, parse};

fn lower_a() -> Option<Sexpr> {
    let directory = std::env::var_os("LEMMATA_PTL_DIR")?;
    let entry = std::fs::read_dir(&directory)
        .expect("read the dump directory")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("letter_latin_lower-a")
        })?;
    let text = std::fs::read_to_string(entry.path()).expect("read the dump");
    let program = parse(&text).expect("parse the dump");

    Some(expand(&program).expect("expand"))
}

fn block(program: &Sexpr) -> &Sexpr {
    program
        .lists()
        .into_iter()
        .find(|list| list.form() == Some("$block"))
        .expect("the file holds a block")
}

fn imports(body: &Sexpr, name: &str) -> Vec<String> {
    body.lists()
        .into_iter()
        .filter(|list| {
            list.form() == Some("$import") && (*list).atom_at(1).is_some_and(|block| block == name)
        })
        .flat_map(|list| {
            let Sexpr::List(items) = list else {
                return Vec::new();
            };
            let Some(Sexpr::List(names)) = items.get(2) else {
                return Vec::new();
            };
            names
                .iter()
                .filter_map(Sexpr::atom)
                .map(str::to_string)
                .collect()
        })
        .collect()
}

trait AtomAt {
    fn atom_at(&self, index: usize) -> Option<&str>;
}

impl AtomAt for Sexpr {
    fn atom_at(&self, index: usize) -> Option<&str> {
        let Sexpr::List(items) = self else {
            return None;
        };
        items.get(index)?.atom()
    }
}

#[test]
fn the_block_imports_only_the_shapes_it_mentions() {
    let Some(program) = lower_a() else {
        eprintln!("skipping: set LEMMATA_PTL_DIR to a directory of reader dumps");
        return;
    };
    let body = block(&program);
    let shapes = imports(body, "CommonShapes");
    for used in ["no-shape", "Rect", "VBar", "Arch", "flatside"] {
        assert!(shapes.iter().any(|name| name == used), "{used} is missing");
    }
    for unused in ["Circle", "Ellipse", "UShape", "HCrossBar"] {
        assert!(
            !shapes.iter().any(|name| name == unused),
            "{unused} is unused but imported"
        );
    }
}

#[test]
fn a_saved_glyph_carries_its_target_code_and_proc() {
    let Some(program) = lower_a() else {
        eprintln!("skipping: set LEMMATA_PTL_DIR to a directory of reader dumps");
        return;
    };
    let body = block(&program);
    let saves: Vec<&Sexpr> = body
        .lists()
        .into_iter()
        .filter(|list| list.form() == Some("$save"))
        .collect();
    assert!(!saves.is_empty(), "the block saves no glyph");
    for save in &saves {
        let Sexpr::List(items) = save else {
            unreachable!()
        };
        assert_eq!(items.len(), 4, "a save is a target, a code, and a body");
    }
    // `a.crossing` is a plain glyph and its body is a proc.
    let crossing = saves
        .iter()
        .find(|save| save.to_string().contains("a.crossing"))
        .expect("a.crossing is saved");
    let Sexpr::List(items) = crossing else {
        unreachable!()
    };
    assert_eq!(items[3].form(), Some("$proc"));
}

#[test]
fn an_infix_run_resolves_by_precedence() {
    let Some(program) = lower_a() else {
        eprintln!("skipping: set LEMMATA_PTL_DIR to a directory of reader dumps");
        return;
    };
    // `[mix 0 XH DesignParameters.aBarPos] + sw * DesignParameters.aBarStrokeAdj`
    // multiplies before it adds.
    let text = program.to_string();
    assert!(
        text.contains("[+ [mix [.quote 0] XH [. DesignParameters [.quote aBarPos]]] [* sw [. DesignParameters [.quote aBarStrokeAdj]]]]"),
        "the bar's position does not multiply before it adds"
    );
}

#[test]
fn a_named_pair_is_not_subtraction() {
    let Some(program) = lower_a() else {
        eprintln!("skipping: set LEMMATA_PTL_DIR to a directory of reader dumps");
        return;
    };
    let text = program.to_string();
    assert!(
        text.contains("[$pair [.quote swSerif] [. df [.quote mvs]]]"),
        "a `name -- value` pair did not become a parameter pair"
    );
}
