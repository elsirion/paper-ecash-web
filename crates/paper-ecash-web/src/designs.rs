use crate::models::QrErrorCorrection;

#[derive(Clone, Debug)]
pub struct Design {
    pub id: &'static str,
    pub name: &'static str,
    pub front_url: &'static str,
    pub back_url: &'static str,
    pub qr_x_offset_cm: f64,
    pub qr_y_offset_cm: f64,
    pub qr_size_cm: f64,
    pub qr_error_correction: QrErrorCorrection,
    pub qr_overlay_url: Option<&'static str>,
}

pub const DESIGNS: &[Design] = &[
    Design {
        id: "fedi",
        name: "Fedi",
        front_url: "designs/fedi/front.png",
        back_url: "designs/fedi/back.png",
        qr_x_offset_cm: 0.0,
        qr_y_offset_cm: 0.0,
        qr_size_cm: 7.0,
        qr_error_correction: QrErrorCorrection::M,
        qr_overlay_url: None,
    },
    Design {
        id: "fedimint_v2",
        name: "Fedimint v2",
        front_url: "designs/fedimint_v2/front.png",
        back_url: "designs/fedimint_v2/back.png",
        qr_x_offset_cm: 0.0,
        qr_y_offset_cm: 0.0,
        qr_size_cm: 7.0,
        qr_error_correction: QrErrorCorrection::M,
        qr_overlay_url: None,
    },
    Design {
        id: "39c3",
        name: "39C3",
        front_url: "designs/39c3/front.png",
        back_url: "designs/39c3/back.png",
        qr_x_offset_cm: 0.9,
        qr_y_offset_cm: 0.87,
        qr_size_cm: 4.2,
        qr_error_correction: QrErrorCorrection::Q,
        qr_overlay_url: None,
    },
    Design {
        id: "dark_prague_25",
        name: "Dark Prague 2025",
        front_url: "designs/dark_prague_25/front.png",
        back_url: "designs/dark_prague_25/back.png",
        qr_x_offset_cm: 0.0,
        qr_y_offset_cm: 0.0,
        qr_size_cm: 7.0,
        qr_error_correction: QrErrorCorrection::M,
        qr_overlay_url: None,
    },
    Design {
        id: "historic",
        name: "Historic",
        front_url: "designs/historic/front.png",
        back_url: "designs/historic/back.png",
        qr_x_offset_cm: 1.65,
        qr_y_offset_cm: 1.13,
        qr_size_cm: 4.8,
        qr_error_correction: QrErrorCorrection::Q,
        qr_overlay_url: Some("designs/historic/qr_overlay.png"),
    },
];

pub fn get_design(id: &str) -> Option<&'static Design> {
    DESIGNS.iter().find(|d| d.id == id)
}
