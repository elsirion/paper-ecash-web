use printpdf::*;

pub struct NoteTextConfig {
    pub font_bytes: Vec<u8>,
    pub font_size_pt: f32,
    pub color_rgb: (f32, f32, f32),
    pub x_offset_cm: f32,
    pub y_offset_cm: f32,
    pub width_cm: f32,
    pub height_cm: f32,
}

pub fn parse_hex_color(hex: &str) -> (f32, f32, f32) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    (r, g, b)
}

/// A4 dimensions in mm
const A4_WIDTH_MM: f32 = 210.0;
const A4_HEIGHT_MM: f32 = 297.0;

/// Note image dimensions in mm (matching LaTeX: 14cm x 7cm)
const NOTE_WIDTH_MM: f32 = 140.0;
const NOTE_HEIGHT_MM: f32 = 70.0;

/// Notes per page
const NOTES_PER_PAGE: usize = 4;

/// DPI used by printpdf for image scaling (default)
const DPI: f32 = 300.0;

/// Convert pixel dimension to its natural point size at DPI.
fn natural_pt(pixels: usize) -> f32 {
    Px(pixels).into_pt(DPI).0
}

/// Generate a PDF with paper ecash notes.
pub fn generate_pdf(
    qr_pngs: &[Vec<u8>],
    front_png: &[u8],
    back_png: &[u8],
    qr_x_offset_cm: f64,
    qr_y_offset_cm: f64,
    qr_size_cm: f64,
    cutting_lines: bool,
    amount_text: Option<(&NoteTextConfig, &[String])>,
) -> anyhow::Result<Vec<u8>> {
    let mut doc = PdfDocument::new("Paper eCash");
    let mut warnings = Vec::new();

    let qr_x_pt = Mm(qr_x_offset_cm as f32 * 10.0).into_pt();
    let qr_y_pt = Mm(qr_y_offset_cm as f32 * 10.0).into_pt();
    let qr_size_pt = Mm(qr_size_cm as f32 * 10.0).into_pt();
    let note_w_pt = Mm(NOTE_WIDTH_MM).into_pt();
    let note_h_pt = Mm(NOTE_HEIGHT_MM).into_pt();
    let page_h_pt = Mm(A4_HEIGHT_MM).into_pt();

    // Decode front and back images, register as XObjects
    let front_raw =
        RawImage::decode_from_bytes(front_png, &mut warnings).map_err(|e| anyhow::anyhow!(e))?;
    let front_id = XObjectId::new();
    let front_nat_w = natural_pt(front_raw.width);
    let front_nat_h = natural_pt(front_raw.height);
    doc.resources
        .xobjects
        .map
        .insert(front_id.clone(), XObject::Image(front_raw));

    let back_raw =
        RawImage::decode_from_bytes(back_png, &mut warnings).map_err(|e| anyhow::anyhow!(e))?;
    let back_id = XObjectId::new();
    let back_nat_w = natural_pt(back_raw.width);
    let back_nat_h = natural_pt(back_raw.height);
    doc.resources
        .xobjects
        .map
        .insert(back_id.clone(), XObject::Image(back_raw));

    // Decode QR images and register
    let mut qr_ids = Vec::with_capacity(qr_pngs.len());
    let mut qr_nat_sizes = Vec::with_capacity(qr_pngs.len());
    for qr_png in qr_pngs {
        let qr_raw =
            RawImage::decode_from_bytes(qr_png, &mut warnings).map_err(|e| anyhow::anyhow!(e))?;
        let nat_w = natural_pt(qr_raw.width);
        let nat_h = natural_pt(qr_raw.height);
        let qr_id = XObjectId::new();
        doc.resources
            .xobjects
            .map
            .insert(qr_id.clone(), XObject::Image(qr_raw));
        qr_ids.push(qr_id);
        qr_nat_sizes.push((nat_w, nat_h));
    }

    // Parse and register font if amount text is configured
    let (font_id, parsed_font) = if let Some((text_cfg, _)) = &amount_text {
        let parsed = ParsedFont::from_bytes(&text_cfg.font_bytes, 0, &mut warnings)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse font"))?;
        let id = doc.add_font(&parsed);
        (Some(id), Some(parsed))
    } else {
        (None, None)
    };

    let num_pages = (qr_pngs.len() + NOTES_PER_PAGE - 1) / NOTES_PER_PAGE;

    // Scale factors for front/back images to fill note dimensions
    let front_sx = note_w_pt.0 / front_nat_w;
    let front_sy = note_h_pt.0 / front_nat_h;
    let back_sx = note_w_pt.0 / back_nat_w;
    let back_sy = note_h_pt.0 / back_nat_h;

    for page_idx in 0..num_pages {
        let start = page_idx * NOTES_PER_PAGE;
        let end = (start + NOTES_PER_PAGE).min(qr_pngs.len());
        let notes_on_page = end - start;

        // === FRONT PAGE ===
        let mut front_ops = Vec::new();

        for i in 0..notes_on_page {
            let note_idx = start + i;
            // Position: top-down, left-aligned. PDF origin = bottom-left.
            let y_bottom = Pt(page_h_pt.0 - (i + 1) as f32 * note_h_pt.0);

            // Front image scaled to note size
            front_ops.push(Op::UseXobject {
                id: front_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(0.0)),
                    translate_y: Some(y_bottom),
                    scale_x: Some(front_sx),
                    scale_y: Some(front_sy),
                    dpi: Some(DPI),
                    ..Default::default()
                },
            });

            // QR code overlay
            let (qr_nat_w, qr_nat_h) = qr_nat_sizes[note_idx];
            let qr_sx = qr_size_pt.0 / qr_nat_w;
            let qr_sy = qr_size_pt.0 / qr_nat_h;
            front_ops.push(Op::UseXobject {
                id: qr_ids[note_idx].clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(qr_x_pt.0)),
                    translate_y: Some(Pt(y_bottom.0 + note_h_pt.0 - qr_y_pt.0 - qr_size_pt.0)),
                    scale_x: Some(qr_sx),
                    scale_y: Some(qr_sy),
                    dpi: Some(DPI),
                    ..Default::default()
                },
            });

            // Amount text
            if let (Some(fid), Some(parsed), Some((text_cfg, texts))) =
                (&font_id, &parsed_font, &amount_text)
            {
                if let Some(text) = texts.get(note_idx) {
                    let (r, g, b) = text_cfg.color_rgb;
                    // Box top-left (PDF coords: y grows upward from bottom)
                    let box_x_pt = Mm(text_cfg.x_offset_cm * 10.0).into_pt().0;
                    let box_top_pt = y_bottom.0 + note_h_pt.0
                        - Mm(text_cfg.y_offset_cm * 10.0).into_pt().0;
                    let box_w_pt = Mm(text_cfg.width_cm * 10.0).into_pt().0;
                    let box_h_pt = Mm(text_cfg.height_cm * 10.0).into_pt().0;

                    // Compute text width in pt using glyph advances
                    let upm = parsed.font_metrics.units_per_em as f32;
                    let mut text_width_units: f32 = 0.0;
                    for ch in text.chars() {
                        if let Some(gid) = parsed.lookup_glyph_index(ch as u32) {
                            text_width_units += parsed.get_horizontal_advance(gid) as f32;
                        } else if let Some(sw) = parsed.get_space_width() {
                            text_width_units += sw as f32;
                        }
                    }
                    let text_width_pt = text_width_units * text_cfg.font_size_pt / upm;

                    // Horizontal centering within the box
                    let cursor_x = box_x_pt + (box_w_pt - text_width_pt) / 2.0;
                    // Vertical centering: baseline = box_center_y - (ascent+descent)/2 + descent
                    // Simpler: baseline sits at box_bottom + (box_h - font_size) / 2 + descent_offset
                    // Approximation: baseline = box_top - box_h/2 - font_size_pt * 0.35
                    let cursor_y = box_top_pt - box_h_pt / 2.0 - text_cfg.font_size_pt * 0.35;

                    front_ops.push(Op::StartTextSection);
                    front_ops.push(Op::SetFillColor {
                        col: Color::Rgb(Rgb {
                            r,
                            g,
                            b,
                            icc_profile: None,
                        }),
                    });
                    front_ops.push(Op::SetTextCursor {
                        pos: Point { x: Pt(cursor_x), y: Pt(cursor_y) },
                    });
                    front_ops.push(Op::SetFontSize {
                        font: fid.clone(),
                        size: Pt(text_cfg.font_size_pt),
                    });
                    front_ops.push(Op::WriteText {
                        items: vec![TextItem::Text(text.clone())],
                        font: fid.clone(),
                    });
                    front_ops.push(Op::EndTextSection);
                }
            }

            // Cutting lines
            if cutting_lines {
                let y_top = Pt(y_bottom.0 + note_h_pt.0);
                front_ops.push(Op::SaveGraphicsState);
                front_ops.push(Op::SetOutlineColor {
                    col: Color::Rgb(Rgb {
                        r: 0.75,
                        g: 0.75,
                        b: 0.75,
                        icc_profile: None,
                    }),
                });
                front_ops.push(Op::SetLineDashPattern {
                    dash: LineDashPattern {
                        dash_1: Some(3),
                        gap_1: Some(3),
                        ..Default::default()
                    },
                });
                front_ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });

                // Bottom line
                front_ops.push(Op::DrawLine {
                    line: Line {
                        points: vec![
                            LinePoint { p: Point { x: Pt(0.0), y: y_bottom }, bezier: false },
                            LinePoint { p: Point { x: note_w_pt, y: y_bottom }, bezier: false },
                        ],
                        is_closed: false,
                    },
                });

                // Top line (only for first note on page)
                if i == 0 {
                    front_ops.push(Op::DrawLine {
                        line: Line {
                            points: vec![
                                LinePoint { p: Point { x: Pt(0.0), y: y_top }, bezier: false },
                                LinePoint { p: Point { x: note_w_pt, y: y_top }, bezier: false },
                            ],
                            is_closed: false,
                        },
                    });
                }

                // Right line
                front_ops.push(Op::DrawLine {
                    line: Line {
                        points: vec![
                            LinePoint { p: Point { x: note_w_pt, y: y_bottom }, bezier: false },
                            LinePoint { p: Point { x: note_w_pt, y: y_top }, bezier: false },
                        ],
                        is_closed: false,
                    },
                });
                front_ops.push(Op::RestoreGraphicsState);
            }
        }

        doc.pages.push(PdfPage::new(
            Mm(A4_WIDTH_MM),
            Mm(A4_HEIGHT_MM),
            front_ops,
        ));

        // === BACK PAGE ===
        let mut back_ops = Vec::new();
        let x_right_pt = Mm(A4_WIDTH_MM - NOTE_WIDTH_MM).into_pt();

        for i in 0..notes_on_page {
            let y_bottom = Pt(page_h_pt.0 - (i + 1) as f32 * note_h_pt.0);

            back_ops.push(Op::UseXobject {
                id: back_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(x_right_pt),
                    translate_y: Some(y_bottom),
                    scale_x: Some(back_sx),
                    scale_y: Some(back_sy),
                    dpi: Some(DPI),
                    ..Default::default()
                },
            });
        }

        doc.pages.push(PdfPage::new(
            Mm(A4_WIDTH_MM),
            Mm(A4_HEIGHT_MM),
            back_ops,
        ));
    }

    let save_opts = PdfSaveOptions {
        subset_fonts: true,
        ..Default::default()
    };
    let pdf_bytes = doc.save(&save_opts, &mut warnings);
    Ok(pdf_bytes)
}
