//! Macro expansion, from the reader's calls to the forms a translator can read.
//!
//! `meta/macros.ptl` defines twenty macros over thirteen builtin forms. This
//! module applies them, so its output names no macro and holds no reader
//! scaffolding.
//!
//! Two rules keep the output honest. Literal forms (`.quote` and `.list`) hold
//! symbols rather than expressions, so expansion does not enter them, except
//! that `Just` is identity and is rewritten anywhere it appears. A `define-macro`
//! or `define-operator` form defines a macro, so its body is data and is left
//! alone.
//!
//! The forms that remain are the builtins plus the runtime vocabulary below. The
//! runtime forms carry a `$` so they cannot collide with a name a definition
//! uses, and each one maps to a single constructor in `lemmata-ptl`.

use std::{collections::BTreeSet, fmt};

use crate::Sexpr;

/// The runtime forms expansion introduces, besides the builtins.
pub const RUNTIME: &[&str] = &[
    "$module",
    "$run-module",
    "$block",
    "$import",
    "$export",
    "$proc",
    "$save",
    "$fork",
    "$glyph-is-needed",
    "$width-kinds",
    "$defaults",
];

/// Why a form could not be expanded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// A macro received a shape it does not define.
    Shape {
        form: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape { form, expected } => write!(f, "{form} expects {expected}"),
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

fn shape(form: &'static str, expected: &'static str) -> Error {
    Error::Shape { form, expected }
}

fn list(items: Vec<Sexpr>) -> Sexpr {
    Sexpr::List(items)
}

fn atom(text: &str) -> Sexpr {
    Sexpr::Atom(text.to_string())
}

fn quote(text: &str) -> Sexpr {
    list(vec![atom(".quote"), atom(text)])
}

fn begin(items: Vec<Sexpr>) -> Sexpr {
    let mut all = Vec::with_capacity(items.len() + 1);
    all.push(atom("begin"));
    all.extend(items);

    list(all)
}

/// `object.name(args...)`, the shape the reader gives a method call.
fn method(object: &str, name: &str, args: Vec<Sexpr>) -> Sexpr {
    let mut call = Vec::with_capacity(args.len() + 1);
    call.push(list(vec![atom("."), atom(object), quote(name)]));
    call.extend(args);

    list(call)
}

/// Expands a reader program.
///
/// # Errors
///
/// Returns the shape error of the first macro that does not fit its patterns.
pub fn expand(program: &Sexpr) -> Result<Sexpr> {
    Expander::default().expand(program)
}

#[derive(Default)]
struct Expander {
    /// The names the enclosing glyph block mentions. An unfiltered
    /// `glyph-block-import` keeps only the exports in this set.
    used: Option<BTreeSet<String>>,
}

impl Expander {
    fn expand(&mut self, node: &Sexpr) -> Result<Sexpr> {
        let Sexpr::List(items) = node else {
            return Ok(node.clone());
        };
        let Some(head) = items.first().and_then(Sexpr::atom) else {
            return self.call(items);
        };
        match head {
            // The reader's program wrapper.
            ".begin" => {
                let mut out = Vec::with_capacity(items.len());
                for item in &items[1..] {
                    // `$$include` names `meta/macros.ptl`, whose macros this
                    // module implements, so the statement has no output.
                    if item.form() == Some("$$include") {
                        continue;
                    }
                    out.push(self.expand(item)?);
                }
                Ok(begin(out))
            }
            "$$include" => Ok(Sexpr::Null),
            // A macro definition describes a macro rather than code to expand.
            "define-macro" | "define-operator" => Ok(node.clone()),
            // Literal forms hold symbols, not expressions.
            ".quote" | ".list" => Ok(node.clone()),
            // The reader leaves an infix run flat and the operator table turns
            // it into a tree. Resolving it here means the translator only ever
            // sees a binary form.
            ".operatorPiece" => {
                let tree = operator_tree(&items[1..])?;
                let tree = resolve_pairs(tree)?;
                self.expand(&tree)
            }
            // A prefix operator renames to the binary form, so `!x` is `["!", x]`.
            ".prefix.!" | ".prefix.-" | ".prefix.+" => {
                let [_, operand] = items.as_slice() else {
                    return Err(shape(".prefix", "one operand"));
                };
                let operator = &head[8..];
                Ok(list(vec![atom(operator), self.expand(operand)?]))
            }

            "Just" => {
                let [_, inner] = items.as_slice() else {
                    return Err(shape("Just", "one form"));
                };
                self.expand(inner)
            }
            // The `lift-` form binds a temporary per `@name` it finds. No
            // definition uses it.
            "lift-" => Err(shape("lift-", "an expander, and no definition uses it")),

            "glyph-proc" => {
                let body = begin(self.each(&items[1..])?);
                Ok(list(vec![
                    atom("$proc"),
                    list(vec![atom("currentGlyph")]),
                    body,
                ]))
            }
            "composite-proc" => {
                let steps = self.each(&items[1..])?;
                let mut body = Vec::with_capacity(steps.len());
                for (index, step) in steps.into_iter().enumerate() {
                    let args = if index == 0 {
                        vec![step]
                    } else {
                        vec![step, Sexpr::Bool(true), Sexpr::Bool(true)]
                    };
                    body.push(method("currentGlyph", "include", args));
                }
                Ok(list(vec![
                    atom("$proc"),
                    list(vec![atom("currentGlyph")]),
                    begin(body),
                ]))
            }

            "include" => Ok(method("currentGlyph", "include", self.each(&items[1..])?)),
            "set-width" => Ok(method("currentGlyph", "setWidth", self.each(&items[1..])?)),
            "set-mark-anchor" => {
                Ok(method(
                    "currentGlyph",
                    "setMarkAnchor",
                    self.each(&items[1..])?,
                ))
            }
            "set-base-anchor" => {
                Ok(method(
                    "currentGlyph",
                    "setBaseAnchor",
                    self.each(&items[1..])?,
                ))
            }
            "eject-contour" => {
                Ok(method(
                    "currentGlyph",
                    "ejectTagged",
                    self.each(&items[1..])?,
                ))
            }

            "new-glyph" => {
                let [_, body] = items.as_slice() else {
                    return Err(shape("new-glyph", "one body"));
                };
                Ok(save(Sexpr::Null, Sexpr::Null, self.expand(body)?))
            }
            "create-glyph" => {
                let rest = &items[1..];
                let (target, code, body) = match rest {
                    [body] => (Sexpr::Null, Sexpr::Null, body),
                    [target, body] => (self.expand(target)?, Sexpr::Null, body),
                    [target, code, body] => (self.expand(target)?, self.expand(code)?, body),
                    _ => return Err(shape("create-glyph", "one to three arguments")),
                };
                Ok(save(target, code, self.expand(body)?))
            }
            "create-aliased-glyph" => {
                let rest = &items[1..];
                let (target, code) = match rest {
                    [target] => (self.expand(target)?, Sexpr::Null),
                    [target, code] => (self.expand(target)?, self.expand(code)?),
                    _ => return Err(shape("create-aliased-glyph", "one or two arguments")),
                };
                Ok(save(
                    target,
                    code,
                    list(vec![atom("$fork"), atom("currentGlyph"), Sexpr::Null]),
                ))
            }
            "create-forked-glyph" => {
                let rest = &items[1..];
                let (target, code, body) = match rest {
                    [body] => (Sexpr::Null, Sexpr::Null, body),
                    [target, body] => (self.expand(target)?, Sexpr::Null, body),
                    [target, code, body] => (self.expand(target)?, self.expand(code)?, body),
                    _ => return Err(shape("create-forked-glyph", "one to three arguments")),
                };
                Ok(save(
                    target,
                    code,
                    list(vec![
                        atom("$fork"),
                        atom("currentGlyph"),
                        self.expand(body)?,
                    ]),
                ))
            }
            "glyph-is-needed" => {
                let [_, name] = items.as_slice() else {
                    return Err(shape("glyph-is-needed", "one name"));
                };
                Ok(list(vec![atom("$glyph-is-needed"), self.expand(name)?]))
            }

            "glyph-module" => Ok(list(vec![atom("$module")])),
            "run-glyph-module" => {
                let rest = &items[1..];
                let [path] = rest else {
                    return Err(shape("run-glyph-module", "one path"));
                };
                Ok(list(vec![atom("$run-module"), self.expand(path)?]))
            }
            "glyph-block" => {
                let [_, name, body] = items.as_slice() else {
                    return Err(shape("glyph-block", "a name and a body"));
                };
                let mut used = BTreeSet::new();
                trace(body, &mut used);
                let previous = self.used.replace(used);
                let body = self.expand(body);
                self.used = previous;
                Ok(list(vec![atom("$block"), name.clone(), body?]))
            }
            "glyph-block-import" => self.import(items),
            "glyph-block-export" => {
                let mut out = vec![atom("$export")];
                out.extend(self.each(&items[1..])?);
                Ok(list(out))
            }
            "end-glyph-block" => Ok(begin(Vec::new())),

            "for-width-kinds" => {
                let [_, desired, body @ ..] = items.as_slice() else {
                    return Err(shape("for-width-kinds", "a width and a body"));
                };
                Ok(list(vec![
                    atom("$width-kinds"),
                    self.expand(desired)?,
                    begin(self.each(body)?),
                ]))
            }

            "with-params" => {
                let [_, pairs, body] = items.as_slice() else {
                    return Err(shape("with-params", "a parameter list and a body"));
                };
                let Sexpr::List(pairs) = pairs else {
                    return Err(shape("with-params", "a parameter list"));
                };
                let pairs = pairs
                    .iter()
                    .map(|pair| self.parameter(pair))
                    .collect::<Result<Vec<_>>>()?;
                Ok(list(vec![
                    atom("$defaults"),
                    list(pairs),
                    self.expand(body)?,
                ]))
            }
            // `flex-params` appears only inside `meta/macros.ptl`, where it
            // defines the macro of the same name.
            "flex-params" => Err(shape("flex-params", "a definition body")),

            // `define` accepts a definer macro between itself and the target.
            // Only `flex-params` is used that way, and it turns the body into a
            // lambda whose parameters are the body's `local-parameter`s.
            "define" if items.get(1).and_then(Sexpr::atom) == Some("flex-params") => {
                self.define_flex(&items[1..])
            }

            // `where` binds functions after the statements that call them. A
            // definition is hoisted to the top of the block, so it lowers to one,
            // or to a block of them when the clause holds several.
            "where" => {
                let clauses = &items[1..];
                if clauses.is_empty() {
                    return Err(shape("where", "a binding and a body"));
                }
                let mut definitions = Vec::with_capacity(clauses.len());
                for clause in clauses {
                    let Sexpr::List(pair) = clause else {
                        return Err(shape("where", "a binding and a body"));
                    };
                    let [target, body] = pair.as_slice() else {
                        return Err(shape("where", "a binding and a body"));
                    };
                    definitions.push(list(vec![
                        atom("define"),
                        target.clone(),
                        self.expand(body)?,
                    ]));
                }
                match definitions.len() {
                    1 => Ok(definitions.remove(0)),
                    _ => Ok(begin(definitions)),
                }
            }

            "match" => {
                let [_, subject, arms @ ..] = items.as_slice() else {
                    return Err(shape("match", "a subject and arms"));
                };
                let mut out = vec![atom("match"), self.expand(subject)?];
                for arm in arms {
                    let Sexpr::List(pair) = arm else {
                        return Err(shape("match", "arms of a pattern and a body"));
                    };
                    let [pattern, body] = pair.as_slice() else {
                        return Err(shape("match", "arms of a pattern and a body"));
                    };
                    out.push(list(vec![self.expand(pattern)?, self.expand(body)?]));
                }
                Ok(list(out))
            }

            _ => self.call(items),
        }
    }

    /// A pair of `with-params`: a name, or a name and its default.
    ///
    /// A default written `name -- value` resolves to `["--", name, value]` once
    /// the infix run is a tree, so both spellings arrive as a pair.
    fn parameter(&mut self, pair: &Sexpr) -> Result<Sexpr> {
        let pair = self.expand(pair)?;
        match &pair {
            Sexpr::Atom(name) => Ok(list(vec![atom(name), Sexpr::Null])),
            Sexpr::List(items) => {
                match items.as_slice() {
                    [op, name, default] if op.atom() == Some("$pair") => {
                        Ok(list(vec![pair_key(name)?, default.clone()]))
                    }
                    [name, default] => Ok(list(vec![name.clone(), default.clone()])),
                    _ => Err(shape("with-params", "a name, or a name and a default")),
                }
            }
            _ => Err(shape("with-params", "a name, or a name and a default")),
        }
    }

    /// `define flex-params [name] body`.
    ///
    /// The definer turns the body into a lambda, collects the body's
    /// `local-parameter` statements into its parameter list, and then defines
    /// `name`. A parameter without `--` has no default. The body keeps its
    /// leader, whether that is a `begin` or a `glyph-proc`.
    fn define_flex(&mut self, rest: &[Sexpr]) -> Result<Sexpr> {
        let [_, target, body] = rest else {
            return Err(shape("define", "a definer, a target and a body"));
        };
        let Sexpr::List(target) = target else {
            return Err(shape("define", "a target list after flex-params"));
        };
        let Sexpr::List(statements) = body else {
            return Err(shape("define", "a body after flex-params"));
        };
        let Some(leader) = statements.first() else {
            return Err(shape("define", "a body after flex-params"));
        };

        let mut params: Vec<Sexpr> = Vec::new();
        let mut kept = vec![leader.clone()];
        for statement in &statements[1..] {
            let local = match statement {
                Sexpr::List(items) => {
                    match items.as_slice() {
                        [head, argument] if head.atom() == Some("local-parameter") => {
                            Some(argument)
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            match local {
                Some(argument) => params.push(self.parameter_argument(argument)?),
                None => kept.push(statement.clone()),
            }
        }

        let body = self.expand(&list(kept))?;
        let mut out = vec![atom("define")];
        // `target` holds the name, and the collected parameters follow it.
        out.extend(target.iter().cloned());
        out.push(list(vec![atom("$defaults"), list(params), body]));

        Ok(list(out))
    }

    /// One `local-parameter` argument: a name, or an infix pair with a default.
    fn parameter_argument(&mut self, argument: &Sexpr) -> Result<Sexpr> {
        let argument = self.expand(argument)?;
        match &argument {
            Sexpr::Atom(name) => Ok(list(vec![atom(name), Sexpr::Null])),
            Sexpr::List(items) => {
                match items.as_slice() {
                    [name] => Ok(list(vec![name.clone(), Sexpr::Null])),
                    [op, name, default] if op.atom() == Some("$pair") => {
                        Ok(list(vec![pair_key(name)?, default.clone()]))
                    }
                    _ => Err(shape("local-parameter", "a name, or a name and a default")),
                }
            }
            _ => Err(shape("local-parameter", "a name")),
        }
    }

    fn import(&mut self, items: &[Sexpr]) -> Result<Sexpr> {
        let rest = &items[1..];
        let (block, names) = match rest {
            [block] => (block, None),
            [block, names] => (block, Some(names)),
            _ => return Err(shape("glyph-block-import", "a block and optional names")),
        };
        let Sexpr::Atom(block_name) = block else {
            return Err(shape("glyph-block-import", "a literal block name"));
        };

        let names = match names {
            Some(names) => {
                let names = names_list(names)?;
                self.each(&names)?
            }
            None => {
                match exports_of(block_name) {
                    Some(exports) => {
                        let used = self.used.as_ref();
                        exports
                            .iter()
                            .filter(|name| used.is_none_or(|used| used.contains(**name)))
                            .map(|name| atom(name))
                            .collect()
                    }
                    // No table names the exports, so import everything the block
                    // defines.
                    None => Vec::new(),
                }
            }
        };

        Ok(list(vec![atom("$import"), atom(block_name), list(names)]))
    }

    fn each(&mut self, nodes: &[Sexpr]) -> Result<Vec<Sexpr>> {
        nodes.iter().map(|node| self.expand(node)).collect()
    }

    fn call(&mut self, items: &[Sexpr]) -> Result<Sexpr> {
        self.each(items).map(list)
    }
}

fn names_list(node: &Sexpr) -> Result<Vec<Sexpr>> {
    match node {
        Sexpr::List(items) => Ok(items.clone()),
        // A single name needs no brackets.
        Sexpr::Atom(_) => Ok(vec![node.clone()]),
        _ => Err(shape("glyph-block-import", "a list of names")),
    }
}

/// The name a `$pair` carries, which `--` quotes on its left.
fn pair_key(node: &Sexpr) -> Result<Sexpr> {
    match node {
        Sexpr::List(items) => {
            match items.as_slice() {
                [quote, name] if quote.atom() == Some(".quote") => Ok(name.clone()),
                _ => Err(shape("--", "a quoted name")),
            }
        }
        _ => Err(shape("--", "a quoted name")),
    }
}

fn save(target: Sexpr, code: Sexpr, body: Sexpr) -> Sexpr {
    list(vec![atom("$save"), target, code, body])
}

/// Renders one node of the operator tree, deepest first.
fn render_operator(nodes: &[OperatorNode], index: usize) -> Sexpr {
    let node = &nodes[index];
    match &node.operator {
        Some(operator) => {
            let left = node
                .left
                .map_or(Sexpr::Null, |left| render_operator(nodes, left));
            let right = node
                .right
                .map_or(Sexpr::Null, |right| render_operator(nodes, right));
            list(vec![atom(operator), left, right])
        }
        None => node.operand.clone().unwrap_or(Sexpr::Null),
    }
}

/// Rewrites `name -- value` into a named parameter pair.
///
/// `--` is not arithmetic. `macros.ptl` defines it as the operator that builds a
/// `NamedParameterPair`, so it is the one operator the tree does not leave as a
/// call.
fn resolve_pairs(node: Sexpr) -> Result<Sexpr> {
    let Sexpr::List(items) = node else {
        return Ok(node);
    };
    if items.len() == 3 && items[0].atom() == Some("--") {
        let mut items = items;
        let right = resolve_pairs(items.pop().expect("three items"))?;
        let left = resolve_pairs(items.pop().expect("three items"))?;
        let name = match &left {
            Sexpr::Atom(name) => quote(name),
            Sexpr::List(_) => left,
            _ => return Err(shape("--", "a name on its left")),
        };
        return Ok(list(vec![atom("$pair"), name, right]));
    }

    items
        .into_iter()
        .map(resolve_pairs)
        .collect::<Result<Vec<_>>>()
        .map(list)
}

/// How an infix operator is grouped.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Assoc {
    Left,
    Right,
    Never,
}

/// The priority and grouping the reader gives an operator.
///
/// The defaults are `patel/src/ex.js`'s table, keyed on the first character.
/// `meta/macros.ptl` adds four operators, and only those four differ.
fn operator_info(operator: &str) -> (i32, Assoc) {
    match operator {
        "--" => (890, Assoc::Right),
        "~>" => (880, Assoc::Right),
        "**" => (150, Assoc::Right),
        "" | "=" => (1000, Assoc::Right),
        _ => {
            let first = operator.as_bytes()[0];
            let priority = match first {
                b'?' | b'^' => 100,
                b'*' | b'/' | b'%' => 200,
                b'+' | b'-' => 300,
                b'=' | b'!' => 400,
                b'<' | b'>' => 500,
                b'&' => 600,
                b'|' => 700,
                _ => 1000,
            };
            let assoc = if operator.len() > 1 && operator.ends_with('>') {
                Assoc::Right
            } else {
                match first {
                    b'!' | b'=' | b'<' | b'>' => Assoc::Never,
                    _ => Assoc::Left,
                }
            };

            (priority, assoc)
        }
    }
}

/// One node of the tree the operator table builds.
struct OperatorNode {
    /// The operator, or `None` for an operand.
    operator: Option<String>,
    left: Option<usize>,
    right: Option<usize>,
    /// The operand itself, when this node is a leaf.
    operand: Option<Sexpr>,
    binding: i32,
}

/// Resolves a flat `operand operator operand ...` run into a tree.
///
/// The reader's codegen does this before evaluating, so the tree is the form
/// the program means rather than a left-to-right reading of the tokens.
fn operator_tree(items: &[Sexpr]) -> Result<Sexpr> {
    if items.len() < 3 || items.len().is_multiple_of(2) {
        return Err(shape(".operatorPiece", "operands and operators"));
    }

    // Index 0 is the sentinel the insertion walks, index 1 is the first operand.
    let mut nodes = vec![
        OperatorNode {
            operator: None,
            left: None,
            right: Some(1),
            operand: None,
            binding: 65534,
        },
        OperatorNode {
            operator: None,
            left: None,
            right: None,
            operand: Some(items[0].clone()),
            binding: i32::MIN,
        },
    ];

    let mut at = 1;
    while at + 1 < items.len() {
        let operator = items[at]
            .atom()
            .ok_or_else(|| shape(".operatorPiece", "an operator name"))?
            .to_string();
        let operand = items[at + 1].clone();
        at += 2;

        let (binding, assoc) = operator_info(&operator);
        let binding = binding.min(65534);
        let operand_index = nodes.len();
        nodes.push(OperatorNode {
            operator: None,
            left: None,
            right: None,
            operand: Some(operand),
            binding: i32::MIN,
        });
        let node_index = nodes.len();
        nodes.push(OperatorNode {
            operator: Some(operator),
            left: None,
            right: Some(operand_index),
            operand: None,
            binding,
        });

        // Walk the right spine to the last node this operator binds past.
        let mut cursor = 0usize;
        while let Some(right) = nodes[cursor].right {
            let past = match assoc {
                Assoc::Left | Assoc::Never => nodes[right].binding > binding,
                Assoc::Right => nodes[right].binding >= binding,
            };
            if !past {
                break;
            }
            cursor = right;
        }
        nodes[node_index].left = nodes[cursor].right;
        nodes[cursor].right = Some(node_index);
    }

    let root = nodes[0]
        .right
        .ok_or_else(|| shape(".operatorPiece", "an operand"))?;

    Ok(render_operator(&nodes, root))
}

/// Every name a glyph block mentions, so an unfiltered import can keep only the
/// exports it uses.
fn trace(node: &Sexpr, used: &mut BTreeSet<String>) {
    match node {
        Sexpr::Atom(text) => {
            used.insert(text.clone());
        }
        Sexpr::List(items) => {
            // A quoted property name is not a reference to the name, but
            // upstream traces it anyway and the table bounds the result.
            for item in items {
                trace(item, used);
            }
        }
        Sexpr::Number(_) | Sexpr::Bool(_) | Sexpr::Null => {}
    }
}

/// The exports of the blocks `macros.ptl` keeps a table for.
///
/// Every other block is imported by name, so its full export list never needs
/// to be known here.
fn exports_of(block: &str) -> Option<&'static [&'static str]> {
    const COMMON_DERIVATIVES: &[&str] = &[
        "query-glyph",
        "refer-glyph",
        "select-variant",
        "select-multi-dimensional-variant",
        "conditional-follow",
        "link-reduced-variant",
        "alias-reduced-variant",
        "DeriveMeshT",
        "derive-glyphs",
        "derive-multi-part-glyphs",
        "derive-composites",
        "alias",
        "orthographic-italic",
        "orthographic-slanted",
        "turned",
        "HCombine",
        "HDual",
        "VCombine",
        "VDual",
        "link-gr",
        "HalfAdvance",
        "NameUni",
    ];
    const COMMON_SHAPES: &[&str] = &[
        "no-shape",
        "tagged",
        "Rect",
        "SquareAt",
        "MaskAbove",
        "MaskBelow",
        "MaskLeft",
        "MaskRight",
        "MaskAboveLine",
        "MaskBelowLine",
        "MaskLeftLine",
        "MaskRightLine",
        "HalfRectTriangle",
        "Ring",
        "RingAt",
        "DotAt",
        "RingStroke",
        "RingStrokeAt",
        "DotStrokeAt",
        "Circle",
        "Ellipse",
        "OShapeT",
        "OShape",
        "OShapeOutline",
        "OShapeFlatTB",
        "UShapeT",
        "UShape",
        "UShapeOutline",
        "HSerif",
        "VSerif",
        "NeedSlab",
        "NeedNotItalic",
        "DiagCor",
        "DiagCorDs",
        "HBar",
        "HOverlayBar",
        "HOverlayObliqueBar",
        "HCrossBar",
        "FlatSlashShape",
        "VBar",
        "HookStart",
        "HookEnd",
        "flatside",
        "Arch",
        "Ungizmo",
        "Regizmo",
        "FlipAround",
        "ScaleAround",
        "Realign",
        "ForceUpright",
        "PointingTo",
        "with-transform",
        "with-outlined",
        "remove-holes",
        "radicalize",
        "clear-geometry",
        "clear-anchors",
        "ExtLineCenter",
        "ExtLineLhs",
        "ExtLineLhsToRhs",
        "ExtLineRhs",
        "ExtLineRhsToLhs",
        "DotVariants",
        "WithDotVariants",
        "TangentToNormal",
        "VectorDot",
        "sharp-corner",
    ];

    match block {
        "Common-Derivatives" => Some(COMMON_DERIVATIVES),
        "CommonShapes" => Some(COMMON_SHAPES),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, expand};
    use crate::{Sexpr, parse};

    fn expand_str(text: &str) -> Sexpr {
        expand(&parse(text).expect("parse")).expect("expand")
    }

    fn heads(node: &Sexpr) -> Vec<String> {
        node.lists()
            .into_iter()
            .filter_map(|list| list.form().map(str::to_string))
            .collect()
    }

    #[test]
    fn a_glyph_proc_becomes_a_proc_of_the_current_glyph() {
        let out = expand_str(r#"["glyph-proc",["include","shape"]]"#);
        assert_eq!(
            out.to_string(),
            "[$proc [currentGlyph] [begin [[. currentGlyph [.quote include]] shape]]]"
        );
    }

    #[test]
    fn include_keeps_its_arguments() {
        let out = expand_str(r#"["include",["a","b"],"c",true]"#);
        assert_eq!(
            out.to_string(),
            "[[. currentGlyph [.quote include]] [a b] c true]"
        );
    }

    #[test]
    fn just_is_identity() {
        let out = expand_str(r#"["Just","HOOK-HALF"]"#);
        assert_eq!(out.to_string(), "HOOK-HALF");
    }

    #[test]
    fn a_create_glyph_names_its_target() {
        let out = expand_str(r#"["create-glyph",["+",[".quote","a."],"suffix"],["make","df"]]"#);
        assert_eq!(
            out.to_string(),
            "[$save [+ [.quote a.] suffix] null [make df]]"
        );
    }

    #[test]
    fn a_create_glyph_without_a_code_keeps_the_slot() {
        let out = expand_str(r#"["create-glyph","name",["make"]]"#);
        assert_eq!(out.to_string(), "[$save name null [make]]");
    }

    #[test]
    fn a_reader_include_is_dropped() {
        let out = expand_str(r#"["$$include",[".quote","../../meta/macros.ptl"]]"#);
        assert_eq!(out.to_string(), "null");
    }

    #[test]
    fn a_literal_list_is_not_expanded() {
        let out = expand_str(r#"[".list",["corner","x","y"]]"#);
        assert_eq!(out.to_string(), "[.list [corner x y]]");
    }

    #[test]
    fn an_unfiltered_import_keeps_the_names_the_block_mentions() {
        let out = expand_str(
            r#"["glyph-block","B",["begin",["glyph-block-import","CommonShapes"],["tagged","x"],["Rect","x"]]]"#,
        );
        assert_eq!(
            out.to_string(),
            "[$block B [begin [$import CommonShapes [tagged Rect]] [tagged x] [Rect x]]]"
        );
    }

    #[test]
    fn a_match_pattern_keeps_its_arm_pair() {
        let out = expand_str(r#"["match","kind",[["Just","A"],["pick","a"]],["__",["pick","b"]]]"#);
        assert_eq!(out.to_string(), "[match kind [A [pick a]] [__ [pick b]]]");
    }

    #[test]
    fn a_where_binding_becomes_a_definition() {
        let out =
            expand_str(r#"["where",[["make","df","_hookStyle"],["glyph-proc",["include","x"]]]]"#);
        assert_eq!(
            out.to_string(),
            "[define [make df _hookStyle] [$proc [currentGlyph] [begin [[. currentGlyph [.quote include]] x]]]]"
        );
    }

    #[test]
    fn an_expanded_program_holds_no_macro_head() {
        let out = expand_str(
            r#"["begin",["glyph-proc",["include","x"]],["create-glyph","a",["union","x","y"]]]"#,
        );
        assert!(
            !heads(&out)
                .iter()
                .any(|head| crate::MACROS.contains(&head.as_str()))
        );
    }

    #[test]
    fn a_flex_parameter_collects_its_locals() {
        let out = expand_str(
            r#"["define","flex-params",["n"],["begin",["local-parameter",["a"]],["local-parameter",[".operatorPiece","b","--",[".quote",1]]],["return","a"]]]"#,
        );
        assert_eq!(
            out.to_string(),
            "[define n [$defaults [[a null] [b [.quote 1]]] [begin [return a]]]]"
        );
    }

    #[test]
    fn a_bad_shape_names_the_macro() {
        let node = parse(r#"["with-params",[]]"#).expect("parse");
        assert_eq!(
            expand(&node),
            Err(Error::Shape {
                form: "with-params",
                expected: "a parameter list and a body"
            })
        );
    }
}
