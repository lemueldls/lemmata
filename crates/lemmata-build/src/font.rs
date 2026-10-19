//! Assembling a TrueType font from glyph outlines.
//!
//! The builder takes finished contours, not geometry. Glyphs arrive already
//! reduced to quadratic contours, so this module only lays out tables and
//! writes them with `write-fonts`.
//!
//! r[impl build.name]
//! r[impl build.valid]

#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use kurbo::{BezPath, Point};
use lemmata_geom::{QuadifyKind, QuadifyPoint};
use write_fonts::FontBuilder;
use write_fonts::tables::cmap::Cmap;
use write_fonts::tables::glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph};
use write_fonts::tables::gsub::Gsub;
use write_fonts::tables::head::{Flags, Head, MacStyle};
use write_fonts::tables::hhea::Hhea;
use write_fonts::tables::hmtx::{Hmtx, LongMetric};
use write_fonts::tables::maxp::Maxp;
use write_fonts::tables::name::{Name, NameRecord};
use write_fonts::tables::os2::{Os2, SelectionFlags};
use write_fonts::tables::post::Post;
use write_fonts::types::GlyphId16;
use write_fonts::types::{
    FWord, Fixed, GlyphId, LongDateTime, NameId, Tag, UfWord, Version16Dot16,
};

/// Font-wide measurements, in font units.
#[derive(Clone, Debug)]
pub struct Metrics {
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub typo_ascender: i16,
    pub typo_descender: i16,
    pub typo_line_gap: i16,
    pub win_ascent: u16,
    pub win_descent: u16,
    pub x_height: i16,
    pub cap_height: i16,
    pub underline_position: i16,
    pub underline_thickness: i16,
    pub strikeout_position: i16,
    pub strikeout_thickness: i16,
    pub weight_class: u16,
    pub width_class: u16,
    pub italic_angle: f64,
}

impl Default for Metrics {
    fn default() -> Self {
        // Iosevka's vertical metrics at 1000 units per em.
        Self {
            units_per_em: 1000,
            ascender: 965,
            descender: -215,
            line_gap: 70,
            typo_ascender: 965,
            typo_descender: -285,
            typo_line_gap: 0,
            win_ascent: 965,
            win_descent: 285,
            x_height: 520,
            cap_height: 735,
            underline_position: -100,
            underline_thickness: 50,
            strikeout_position: 250,
            strikeout_thickness: 50,
            weight_class: 400,
            width_class: 5,
            italic_angle: 0.0,
        }
    }
}

/// Naming for the output font.
#[derive(Clone, Debug)]
pub struct Names {
    pub family: String,
    pub subfamily: String,
    pub version: String,
    pub vendor: Tag,
}

impl Default for Names {
    fn default() -> Self {
        Self {
            family: "Lemmata".to_string(),
            subfamily: "Regular".to_string(),
            version: "Version 1.000".to_string(),
            vendor: Tag::new(b"LEMM"),
        }
    }
}

/// One glyph, ready to write.
#[derive(Clone, Debug)]
pub struct GlyphData {
    pub name: String,
    pub unicodes: Vec<u32>,
    pub advance: u16,
    pub contours: Vec<Vec<QuadifyPoint>>,
}

impl GlyphData {
    /// The glyph's bounding box, or `None` when it has no points.
    #[must_use]
    pub fn bounding_box(&self) -> Option<(i16, i16, i16, i16)> {
        ink(&self.contours)
    }
}

/// The bounding box of a set of contours, or `None` when they hold no points.
#[must_use]
pub fn ink(contours: &[Vec<QuadifyPoint>]) -> Option<(i16, i16, i16, i16)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for point in contours.iter().flatten() {
        let entry = bounds.get_or_insert((point.x, point.y, point.x, point.y));
        entry.0 = entry.0.min(point.x);
        entry.1 = entry.1.min(point.y);
        entry.2 = entry.2.max(point.x);
        entry.3 = entry.3.max(point.y);
    }

    bounds.map(|(x_min, y_min, x_max, y_max)| {
        (
            x_min.floor() as i16,
            y_min.floor() as i16,
            x_max.ceil() as i16,
            y_max.ceil() as i16,
        )
    })
}

/// The glyphs derived from the captured set: the size variants of the stretchy
/// glyphs, then the pieces their assemblies stack.
///
/// The writer appends these and a caller reporting the glyph count reads the
/// same function, so a derived glyph cannot be written without being counted.
/// The pieces take the private-use codes after the variants, so the two blocks
/// do not collide.
#[must_use]
pub fn derived(glyphs: &[GlyphData], metrics: &Metrics) -> Vec<GlyphData> {
    let mut derived = crate::variants::derive(glyphs);
    let next_code = |derived: &[GlyphData]| {
        crate::variants::PRIVATE_USE_START + u32::try_from(derived.len()).unwrap_or(0)
    };
    let pieces = crate::assembly::derive(
        glyphs,
        next_code(&derived),
        metrics.underline_thickness.unsigned_abs(),
    );
    derived.extend(pieces);
    let dotless = crate::dotless::derive(glyphs, f64::from(metrics.x_height), next_code(&derived));
    derived.extend(dotless);
    let script = crate::ssty::derive(glyphs, next_code(&derived));
    derived.extend(script);
    let flattened = crate::flac::derive(glyphs, next_code(&derived));
    derived.extend(flattened);
    let healed = crate::texture::derive(glyphs, next_code(&derived));
    derived.extend(healed);

    derived
}

/// The `calt` feature: texture healing's passes, in the order they have to run.
///
/// Each pass names what it changes and what it changes it in front of, and a
/// later pass sees what an earlier one left: the centre pass only fires between
/// two givers that the first two passes have already moved apart.
fn texture(glyphs: &[GlyphData]) -> Option<lemmata_layout::Feature> {
    use crate::texture::{Role, Side, form_name};

    let id = |name: &str| -> Option<GlyphId16> {
        let index = glyphs.iter().position(|glyph| glyph.name == name)?;
        u16::try_from(index + 1).ok().map(GlyphId16::new)
    };
    // One classification for the coverages and the forms both, so the rule and
    // the glyph it names cannot come apart.
    let roles = crate::texture::classify(glyphs);
    let of = |role: Role| -> Vec<usize> {
        roles
            .iter()
            .filter(|(_, recorded)| *recorded == role)
            .map(|(index, _)| *index)
            .collect()
    };
    let givers = of(Role::Gives);
    let takers = of(Role::Takes);
    let stationary = of(Role::Stationary);
    if givers.is_empty() || takers.is_empty() {
        return None;
    }
    let ids = |indices: &[usize]| -> Vec<GlyphId16> {
        indices
            .iter()
            .filter_map(|index| id(&glyphs[*index].name))
            .collect()
    };
    let shifted = |indices: &[usize], side: Side| -> Vec<(GlyphId16, GlyphId16)> {
        indices
            .iter()
            .filter_map(|index| {
                let name = &glyphs[*index].name;
                Some((id(name)?, id(&form_name(name, side))?))
            })
            .collect()
    };

    let givers_left: Vec<GlyphId16> = shifted(&givers, Side::Left)
        .into_iter()
        .map(|(_, to)| to)
        .collect();
    let givers_right: Vec<GlyphId16> = shifted(&givers, Side::Right)
        .into_iter()
        .map(|(_, to)| to)
        .collect();
    let wide = ids(&takers);
    let mut passes = vec![
        // A giver with a taker on each side has room on both, so it takes both
        // at once and widens. Moving it one way would close one gap by exactly
        // what it opened on the other, which is no better than leaving it. This
        // runs first because the passes below look for the base form, and a
        // giver this one widened is no longer it.
        lemmata_layout::Rule {
            backtrack: wide.clone(),
            input: ids(&givers),
            lookahead: wide.clone(),
            apply: shifted(&givers, Side::Both),
        },
        // A giver before a taker cedes toward it.
        lemmata_layout::Rule {
            backtrack: Vec::new(),
            input: ids(&givers),
            lookahead: wide.clone(),
            apply: shifted(&givers, Side::Left),
        },
        // A giver after a taker cedes toward it.
        lemmata_layout::Rule {
            backtrack: wide.clone(),
            input: ids(&givers),
            lookahead: Vec::new(),
            apply: shifted(&givers, Side::Right),
        },
        // A taker between two givers takes both sides. This runs after the two
        // passes above, so what it looks for is the forms they left behind.
        lemmata_layout::Rule {
            backtrack: givers_left.clone(),
            input: wide.clone(),
            lookahead: givers_right.clone(),
            apply: shifted(&takers, Side::Both),
        },
    ];

    // A giver that has already moved left opened a gap on its right, so the
    // taker after it can reach into that gap. The same the other way round.
    // These run after the centre pass, and a taker it turned into `.both` is no
    // longer in the input coverage, so the two cannot fight.
    if !givers_left.is_empty() {
        passes.push(lemmata_layout::Rule {
            backtrack: givers_left,
            input: wide.clone(),
            lookahead: Vec::new(),
            apply: shifted(&takers, Side::Left),
        });
    }
    if !givers_right.is_empty() {
        passes.push(lemmata_layout::Rule {
            backtrack: Vec::new(),
            input: wide.clone(),
            lookahead: givers_right,
            apply: shifted(&takers, Side::Right),
        });
    }

    // A mark beside a taker does not move itself, so the taker moves toward it.
    let marks = ids(&stationary);
    if !marks.is_empty() {
        passes.push(lemmata_layout::Rule {
            backtrack: marks.clone(),
            input: wide.clone(),
            lookahead: Vec::new(),
            apply: shifted(&takers, Side::Left),
        });
        passes.push(lemmata_layout::Rule {
            backtrack: Vec::new(),
            input: wide,
            lookahead: marks,
            apply: shifted(&takers, Side::Right),
        });
    }

    Some(lemmata_layout::Feature {
        tag: Tag::new(b"calt"),
        scripts: vec![Tag::new(b"latn"), Tag::new(b"DFLT")],
        effect: lemmata_layout::Effect::Contextual(passes),
    })
}

/// The substitutions the font carries.
///
/// Two features: `dtls`, the dotless forms, and `ssty`, the script-size
/// alternates. A pair is found by name, the way the MATH table finds its derived
/// glyphs, so the derivation and the substitution cannot come to name different
/// glyphs.
fn substitutions(glyphs: &[GlyphData]) -> Gsub {
    let id = |name: &str| -> Option<GlyphId16> {
        let index = glyphs.iter().position(|glyph| glyph.name == name)?;
        u16::try_from(index + 1).ok().map(GlyphId16::new)
    };

    let mut pairs = Vec::new();
    for glyph in glyphs {
        let Some(base) = glyph.name.strip_suffix(".dotless") else {
            continue;
        };
        if let (Some(from), Some(to)) = (id(base), id(&glyph.name)) {
            pairs.push((from, to));
        }
    }

    // Each base names its alternates in the order the renderer picks them, so
    // the first is the script size and the second the scriptscript size.
    let mut alternates = Vec::new();
    for glyph in glyphs {
        let forms: Vec<GlyphId16> = (0..crate::ssty::SCALES.len())
            .filter_map(|index| id(&crate::ssty::alternate_name(&glyph.name, index)))
            .collect();
        if forms.is_empty() {
            continue;
        }
        if let Some(base) = id(&glyph.name) {
            alternates.push((base, forms));
        }
    }

    // `flac` flattens an accent for a tall base, so every accent the font
    // carries above the baseline is named.
    let mut flattened = Vec::new();
    for glyph in glyphs {
        let Some(base) = glyph.name.strip_suffix(".mathcap") else {
            continue;
        };
        if let (Some(from), Some(to)) = (id(base), id(&glyph.name)) {
            flattened.push((from, to));
        }
    }

    let mut features = vec![
        lemmata_layout::Feature {
            tag: Tag::new(b"dtls"),
            scripts: vec![Tag::new(b"math")],
            effect: lemmata_layout::Effect::Single(pairs),
        },
        lemmata_layout::Feature {
            tag: Tag::new(b"flac"),
            scripts: vec![Tag::new(b"math")],
            effect: lemmata_layout::Effect::Single(flattened),
        },
        lemmata_layout::Feature {
            tag: Tag::new(b"ssty"),
            scripts: vec![Tag::new(b"math")],
            effect: lemmata_layout::Effect::Alternate(alternates),
        },
    ];
    if let Some(healing) = texture(glyphs) {
        features.push(healing);
    }

    lemmata_layout::gsub(&features)
}

/// Builds a TrueType font.
///
/// Glyph zero is an empty `.notdef`; the rest follow the input order.
///
/// `variants` is the plan's character choices. They are applied before anything
/// else, because the derived glyphs are derived from what the choices leave: a
/// `dtls` form of the `i` the plan selected, not of the one it replaced.
///
/// # Errors
///
/// Returns a message when a contour cannot be written or a table fails to
/// compile.
pub fn build(
    glyphs: &[GlyphData],
    metrics: &Metrics,
    names: &Names,
    variants: &[(String, String)],
) -> Result<Vec<u8>, String> {
    // The derived size variants join the set first, before anything counts the
    // glyphs or indexes into them. Appending them later leaves them past
    // `maxp.num_glyphs`, where they are ignored until something references one
    // and the reference is out of range.
    let glyphs = crate::character::apply(glyphs, variants);
    let mut with_variants = glyphs.clone();
    with_variants.extend(derived(&glyphs, metrics));
    let glyphs = with_variants.as_slice();

    let mut glyf_builder = GlyfLocaBuilder::new();
    glyf_builder
        .add_glyph(&Glyph::Empty)
        .map_err(|error| format!(".notdef: {error}"))?;

    let mut h_metrics = vec![LongMetric {
        advance: glyphs.first().map_or(0, |glyph| glyph.advance),
        side_bearing: 0,
    }];
    let mut cmap_mappings = Vec::new();
    let mut max_points = 0u16;
    let mut max_contours = 0u16;
    let mut advance_width_max = 0u16;
    let mut min_left_side_bearing = i16::MAX;
    let mut min_right_side_bearing = i16::MAX;
    let mut x_max_extent = i16::MIN;
    let mut font_bounds: Option<(i16, i16, i16, i16)> = None;

    for glyph in glyphs {
        let path = glyph_path(&glyph.contours);
        let simple = SimpleGlyph::from_bezpath(&path)
            .map_err(|error| format!("{}: {error:?}", glyph.name))?;

        let points: u16 = simple
            .contours
            .iter()
            .map(|contour| contour.len() as u16)
            .sum();
        max_points = max_points.max(points);
        max_contours = max_contours.max(simple.contours.len() as u16);

        let (x_min, y_min, x_max, y_max) = glyph.bounding_box().unwrap_or_default();
        let side_bearing = x_min;
        let right_side_bearing = i32::from(glyph.advance)
            - (i32::from(side_bearing) + i32::from(x_max) - i32::from(x_min));
        min_left_side_bearing = min_left_side_bearing.min(side_bearing);
        min_right_side_bearing =
            min_right_side_bearing.min(i16::try_from(right_side_bearing).unwrap_or(i16::MAX));
        x_max_extent = x_max_extent.max(side_bearing + (x_max - x_min));
        advance_width_max = advance_width_max.max(glyph.advance);
        if glyph.bounding_box().is_some() {
            let entry = font_bounds.get_or_insert((x_min, y_min, x_max, y_max));
            entry.0 = entry.0.min(x_min);
            entry.1 = entry.1.min(y_min);
            entry.2 = entry.2.max(x_max);
            entry.3 = entry.3.max(y_max);
        }

        for unicode in &glyph.unicodes {
            if let Some(ch) = char::from_u32(*unicode) {
                cmap_mappings.push((ch, GlyphId::new(h_metrics.len() as u32)));
            }
        }
        h_metrics.push(LongMetric {
            advance: glyph.advance,
            side_bearing,
        });

        glyf_builder
            .add_glyph(&simple)
            .map_err(|error| format!("{}: {error}", glyph.name))?;
    }

    let (glyf, loca, loca_format) = glyf_builder.build();
    let (x_min, y_min, x_max, y_max) = font_bounds.unwrap_or((0, 0, 0, 0));
    let glyph_count = (glyphs.len() + 1) as u16;
    let first_char = cmap_mappings
        .iter()
        .map(|(ch, _)| *ch as u32)
        .min()
        .unwrap_or(0);
    let last_char = cmap_mappings
        .iter()
        .map(|(ch, _)| *ch as u32)
        .max()
        .unwrap_or(0);

    // One source of truth for the style. A renderer picks an italic face by the
    // style bits, so they have to agree with the angle the glyphs were drawn at.
    let italic = metrics.italic_angle != 0.0;

    let head = Head {
        font_revision: Fixed::from_f64(1.0),
        checksum_adjustment: 0,
        magic_number: 0x5F0F_3CF5,
        flags: Flags::BASELINE_AT_Y_0 | Flags::FORCE_INTEGER_PPEM,
        units_per_em: metrics.units_per_em,
        created: LongDateTime::new(0),
        modified: LongDateTime::new(0),
        x_min,
        y_min,
        x_max,
        y_max,
        mac_style: if italic {
            MacStyle::ITALIC
        } else {
            MacStyle::empty()
        },
        lowest_rec_ppem: 8,
        font_direction_hint: 2,
        index_to_loc_format: match loca_format {
            write_fonts::tables::loca::LocaFormat::Short => 0,
            write_fonts::tables::loca::LocaFormat::Long => 1,
        },
    };

    let hhea = Hhea {
        ascender: FWord::new(metrics.ascender),
        descender: FWord::new(metrics.descender),
        line_gap: FWord::new(metrics.line_gap),
        advance_width_max: UfWord::new(advance_width_max),
        min_left_side_bearing: FWord::new(min_left_side_bearing),
        min_right_side_bearing: FWord::new(min_right_side_bearing),
        x_max_extent: FWord::new(x_max_extent),
        caret_slope_rise: 1,
        caret_slope_run: 0,
        caret_offset: 0,
        number_of_h_metrics: glyph_count,
    };

    let maxp = Maxp {
        num_glyphs: glyph_count,
        max_points: Some(max_points),
        max_contours: Some(max_contours),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(2),
        max_twilight_points: Some(0),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(0),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };

    let os2 = Os2 {
        x_avg_char_width: h_metrics
            .first()
            .map_or(0, |metric| i16::try_from(metric.advance).unwrap_or(0)),
        us_weight_class: metrics.weight_class,
        us_width_class: metrics.width_class,
        fs_type: 0,
        y_subscript_x_size: 0,
        y_subscript_y_size: 0,
        y_subscript_x_offset: 0,
        y_subscript_y_offset: 0,
        y_superscript_x_size: 0,
        y_superscript_y_size: 0,
        y_superscript_x_offset: 0,
        y_superscript_y_offset: 0,
        y_strikeout_size: metrics.strikeout_thickness,
        y_strikeout_position: metrics.strikeout_position,
        s_family_class: 0,
        panose_10: [0; 10],
        ul_unicode_range_1: 1,
        ul_unicode_range_2: 0,
        ul_unicode_range_3: 0,
        ul_unicode_range_4: 0,
        ach_vend_id: names.vendor,
        // The specification has REGULAR mean neither bold nor italic, so an
        // italic face sets ITALIC alone.
        fs_selection: if italic {
            SelectionFlags::ITALIC
        } else {
            SelectionFlags::REGULAR
        },
        us_first_char_index: first_char as u16,
        us_last_char_index: last_char as u16,
        s_typo_ascender: metrics.typo_ascender,
        s_typo_descender: metrics.typo_descender,
        s_typo_line_gap: metrics.typo_line_gap,
        us_win_ascent: metrics.win_ascent,
        us_win_descent: metrics.win_descent,
        ul_code_page_range_1: Some(1),
        ul_code_page_range_2: Some(0),
        sx_height: Some(metrics.x_height),
        s_cap_height: Some(metrics.cap_height),
        us_default_char: Some(0),
        us_break_char: Some(32),
        us_max_context: Some(1),
        us_lower_optical_point_size: None,
        us_upper_optical_point_size: None,
    };

    let post = Post {
        version: Version16Dot16::new(3, 0),
        // Counter-clockwise from the vertical, so a face leaning right is
        // negative.
        italic_angle: Fixed::from_f64(-metrics.italic_angle),
        underline_position: FWord::new(metrics.underline_position),
        underline_thickness: FWord::new(metrics.underline_thickness),
        is_fixed_pitch: 1,
        min_mem_type42: 0,
        max_mem_type42: 0,
        min_mem_type1: 0,
        max_mem_type1: 0,
        num_glyphs: None,
        glyph_name_index: None,
        string_data: None,
    };

    let hmtx = Hmtx {
        h_metrics,
        left_side_bearings: Vec::new(),
    };

    let cmap = Cmap::from_mappings(cmap_mappings).map_err(|error| format!("cmap: {error}"))?;
    let name = build_name_table(names, metrics);

    let math = crate::math::table(metrics, &crate::math::MathProfile::default(), glyphs);
    let gsub = substitutions(glyphs);

    let bytes = FontBuilder::new()
        .add_table(&head)
        .map_err(|error| format!("head: {error}"))?
        .add_table(&hhea)
        .map_err(|error| format!("hhea: {error}"))?
        .add_table(&maxp)
        .map_err(|error| format!("maxp: {error}"))?
        .add_table(&os2)
        .map_err(|error| format!("OS/2: {error}"))?
        .add_table(&hmtx)
        .map_err(|error| format!("hmtx: {error}"))?
        .add_table(&cmap)
        .map_err(|error| format!("cmap: {error}"))?
        .add_table(&name)
        .map_err(|error| format!("name: {error}"))?
        .add_table(&post)
        .map_err(|error| format!("post: {error}"))?
        .add_table(&glyf)
        .map_err(|error| format!("glyf: {error}"))?
        .add_table(&loca)
        .map_err(|error| format!("loca: {error}"))?
        .add_table(&math)
        .map_err(|error| format!("MATH: {error}"))?
        .add_table(&gsub)
        .map_err(|error| format!("GSUB: {error}"))?
        .build();

    Ok(bytes)
}

/// Turns quadratic contours into a path. A quadratic point is a control point
/// whose partner is the next corner; two corners make a line.
pub(crate) fn glyph_path(contours: &[Vec<QuadifyPoint>]) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours {
        if contour.len() < 2 {
            continue;
        }
        // TrueType wants an on-curve start point.
        let start = contour
            .iter()
            .position(|point| point.kind == QuadifyKind::Corner)
            .unwrap_or(0);
        let rotated: Vec<&QuadifyPoint> = contour[start..]
            .iter()
            .chain(contour[..start].iter())
            .collect();
        path.move_to(Point::new(rotated[0].x, rotated[0].y));
        let mut index = 1;
        while index < rotated.len() {
            let point = rotated[index];
            if point.kind == QuadifyKind::Quadratic && index + 1 < rotated.len() {
                let end = rotated[index + 1];
                path.quad_to(Point::new(point.x, point.y), Point::new(end.x, end.y));
                index += 2;
            } else {
                path.line_to(Point::new(point.x, point.y));
                index += 1;
            }
        }
        path.close_path();
    }

    path
}

/// Keeps the glyphs that carry a codepoint inside one of `ranges`.
///
/// This runs on the input rather than on the built font, which is what keeps
/// the MATH table honest. Its glyph ids are assigned while the font is built, so
/// restricting a built font would have to renumber them, and the subsetter does
/// not close over MATH. Building from the smaller set has the same effect on a
/// generator, which has nothing to prune that it did not just write.
pub fn retain_ranges(glyphs: &mut Vec<GlyphData>, ranges: &[(u32, u32)]) {
    glyphs.retain(|glyph| {
        glyph
            .unicodes
            .iter()
            .any(|code| ranges.iter().any(|(low, high)| low <= code && code <= high))
    });
}

/// Compresses a built font to WOFF2.
///
/// The same font in the container a page serves, and the one a subsetter reads.
/// The quality is the crate's default, which is its best.
///
/// # Errors
///
/// Returns a message when the container cannot be built.
pub fn woff2(ttf: &[u8]) -> Result<Vec<u8>, String> {
    ttf2woff2::encode(ttf, ttf2woff2::BrotliQuality::default())
        .map_err(|error| format!("woff2: {error}"))
}

/// The copyright the license requires. The glyphs derive from upstream, so the
/// notice is upstream's.
const COPYRIGHT: &str =
    "Copyright (c) 2015-2026, Renzhi Li (aka. Belleve Invis, belleve@typeof.net)";

/// What nameID 13 carries: the statement the OFL asks a font to make.
const LICENSE_DESCRIPTION: &str =
    "This Font Software is licensed under the SIL Open Font License, Version 1.1.";

/// Where the license lives, for nameID 14.
const LICENSE_URL: &str = "https://openfontlicense.org";

/// The upstream release the glyphs come from, for nameID 10.
///
/// The tag is what a person reads and the commit is what a build can check. The
/// structure suite holds the commit against `golden/parity.json`, so the record
/// in the font cannot drift from the build it describes.
const UPSTREAM: &str =
    "Glyph source: Iosevka 34.9.0, commit 2487637bb71f41b2a66134afd9faa6c316e5a438";

fn build_name_table(names: &Names, metrics: &Metrics) -> Name {
    let full_name = format!("{} {}", names.family, names.subfamily);
    let postscript_name = format!("{}-{}", names.family, names.subfamily);
    let unique_id = format!("{full_name}; Lemmata; {}\n", names.version);
    let records = [
        (NameId::FAMILY_NAME, names.family.clone()),
        (NameId::SUBFAMILY_NAME, names.subfamily.clone()),
        (NameId::UNIQUE_ID, unique_id),
        (NameId::FULL_NAME, full_name),
        (NameId::VERSION_STRING, names.version.clone()),
        (NameId::POSTSCRIPT_NAME, postscript_name),
        // The typographic pair is what lets a renderer find the Italic face of
        // the family rather than treating it as a family of its own. Without
        // it a math renderer sets variables upright, because it never pairs
        // the two.
        (NameId::TYPOGRAPHIC_FAMILY_NAME, names.family.clone()),
        (NameId::TYPOGRAPHIC_SUBFAMILY_NAME, names.subfamily.clone()),
        // The license asks for a copyright notice and a statement of the
        // license, and a font that derives from OFL Font Software has to carry
        // both wherever it goes.
        (NameId::COPYRIGHT_NOTICE, COPYRIGHT.to_string()),
        (NameId::LICENSE_DESCRIPTION, LICENSE_DESCRIPTION.to_string()),
        (NameId::LICENSE_URL, LICENSE_URL.to_string()),
        // The glyph source, so a font in the wild says what it was built from
        // rather than only what it is called.
        (NameId::DESCRIPTION, UPSTREAM.to_string()),
    ];
    let mut name_record = Vec::new();
    for (name_id, value) in records {
        for (platform_id, encoding_id, language_id) in [(3u16, 1u16, 0x0409u16), (1, 0, 0)] {
            name_record.push(NameRecord {
                platform_id,
                encoding_id,
                language_id,
                name_id,
                string: value.clone().into(),
            });
        }
    }
    name_record.sort_by_key(|record| {
        (
            record.platform_id,
            record.encoding_id,
            record.language_id,
            record.name_id.to_u16(),
        )
    });
    let _ = metrics;

    Name {
        name_record,
        lang_tag_record: None,
    }
}
