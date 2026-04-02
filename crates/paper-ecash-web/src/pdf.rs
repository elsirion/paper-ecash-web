use printpdf::*;

/// A4 dimensions in mm
const A4_WIDTH_MM: f32 = 210.0;
const A4_HEIGHT_MM: f32 = 297.0;

/// Note image dimensions in mm (matching LaTeX: 14cm x 7cm)
const NOTE_WIDTH_MM: f32 = 140.0;
const NOTE_HEIGHT_MM: f32 = 70.0;

/// Notes per page
const NOTES_PER_PAGE: usize = 4;

/// Generate a PDF with paper ecash notes.
pub fn generate_pdf(
    qr_pngs: &[Vec<u8>],
    front_png: &[u8],
    back_png: &[u8],
    qr_x_offset_cm: f64,
    qr_y_offset_cm: f64,
    qr_size_cm: f64,
    cutting_lines: bool,
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
    let front_img_w = front_raw.width as f32;
    let front_img_h = front_raw.height as f32;
    doc.resources
        .xobjects
        .map
        .insert(front_id.clone(), XObject::Image(front_raw));

    let back_raw =
        RawImage::decode_from_bytes(back_png, &mut warnings).map_err(|e| anyhow::anyhow!(e))?;
    let back_id = XObjectId::new();
    let back_img_w = back_raw.width as f32;
    let back_img_h = back_raw.height as f32;
    doc.resources
        .xobjects
        .map
        .insert(back_id.clone(), XObject::Image(back_raw));

    // Decode QR images and register
    let mut qr_ids = Vec::with_capacity(qr_pngs.len());
    let mut qr_dims = Vec::with_capacity(qr_pngs.len());
    for qr_png in qr_pngs {
        let qr_raw =
            RawImage::decode_from_bytes(qr_png, &mut warnings).map_err(|e| anyhow::anyhow!(e))?;
        let qr_w = qr_raw.width as f32;
        let qr_h = qr_raw.height as f32;
        let qr_id = XObjectId::new();
        doc.resources
            .xobjects
            .map
            .insert(qr_id.clone(), XObject::Image(qr_raw));
        qr_ids.push(qr_id);
        qr_dims.push((qr_w, qr_h));
    }

    let num_pages = (qr_pngs.len() + NOTES_PER_PAGE - 1) / NOTES_PER_PAGE;

    for page_idx in 0..num_pages {
        let start = page_idx * NOTES_PER_PAGE;
        let end = (start + NOTES_PER_PAGE).min(qr_pngs.len());
        let notes_on_page = end - start;

        // === FRONT PAGE ===
        let mut front_ops = Vec::new();

        for i in 0..notes_on_page {
            let note_idx = start + i;
            // Position: top-down, left-aligned. PDF origin = bottom-left.
            let y_bottom_pt = page_h_pt - Pt((i + 1) as f32 * note_h_pt.0);

            // Front image scaled to note size
            let sx = note_w_pt.0 / front_img_w;
            let sy = note_h_pt.0 / front_img_h;
            front_ops.push(Op::UseXobject {
                id: front_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(0.0)),
                    translate_y: Some(y_bottom_pt),
                    scale_x: Some(sx),
                    scale_y: Some(sy),
                    ..Default::default()
                },
            });

            // QR code overlay
            let (qr_w, qr_h) = qr_dims[note_idx];
            let qr_sx = qr_size_pt.0 / qr_w;
            let qr_sy = qr_size_pt.0 / qr_h;
            front_ops.push(Op::UseXobject {
                id: qr_ids[note_idx].clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(qr_x_pt.0)),
                    translate_y: Some(Pt(y_bottom_pt.0 + qr_y_pt.0)),
                    scale_x: Some(qr_sx),
                    scale_y: Some(qr_sy),
                    ..Default::default()
                },
            });

            // Cutting lines
            if cutting_lines {
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
                            LinePoint { p: Point { x: Pt(0.0), y: y_bottom_pt }, bezier: false },
                            LinePoint { p: Point { x: note_w_pt, y: y_bottom_pt }, bezier: false },
                        ],
                        is_closed: false,
                    },
                });

                // Top line (only for first note)
                if i == 0 {
                    let y_top_pt = Pt(y_bottom_pt.0 + note_h_pt.0);
                    front_ops.push(Op::DrawLine {
                        line: Line {
                            points: vec![
                                LinePoint { p: Point { x: Pt(0.0), y: y_top_pt }, bezier: false },
                                LinePoint { p: Point { x: note_w_pt, y: y_top_pt }, bezier: false },
                            ],
                            is_closed: false,
                        },
                    });
                }

                // Right line
                let y_top_pt = Pt(y_bottom_pt.0 + note_h_pt.0);
                front_ops.push(Op::DrawLine {
                    line: Line {
                        points: vec![
                            LinePoint { p: Point { x: note_w_pt, y: y_bottom_pt }, bezier: false },
                            LinePoint { p: Point { x: note_w_pt, y: y_top_pt }, bezier: false },
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
        let sx = note_w_pt.0 / back_img_w;
        let sy = note_h_pt.0 / back_img_h;

        for i in 0..notes_on_page {
            let y_bottom_pt = page_h_pt - Pt((i + 1) as f32 * note_h_pt.0);

            back_ops.push(Op::UseXobject {
                id: back_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(x_right_pt),
                    translate_y: Some(y_bottom_pt),
                    scale_x: Some(sx),
                    scale_y: Some(sy),
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

    let pdf_bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    Ok(pdf_bytes)
}
