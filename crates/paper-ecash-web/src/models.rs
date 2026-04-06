use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextConfig {
    pub font_family: String,
    pub font_url: String,
    pub font_size_pt: f64,
    pub color_hex: String,
    pub x_offset_cm: f64,
    pub y_offset_cm: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IssuanceStatus {
    AwaitingDeposit,
    Funded,
    Issued,
    Complete,
}

impl IssuanceStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AwaitingDeposit => "Awaiting Deposit",
            Self::Funded => "Funded",
            Self::Issued => "Notes Issued",
            Self::Complete => "Complete",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssuanceConfig {
    pub federation_invite: String,
    pub design_id: String,
    pub denominations_msat: Vec<u64>,
    pub note_count: u32,
    pub qr_x_offset_cm: f64,
    pub qr_y_offset_cm: f64,
    pub qr_size_cm: f64,
    pub qr_error_correction: QrErrorCorrection,
    #[serde(default)]
    pub amount_text: Option<TextConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Copy)]
pub enum QrErrorCorrection {
    M,
    Q,
    H,
}

impl QrErrorCorrection {
    pub fn to_qrcode_ec(self) -> qrcode::EcLevel {
        match self {
            Self::M => qrcode::EcLevel::M,
            Self::Q => qrcode::EcLevel::Q,
            Self::H => qrcode::EcLevel::H,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Issuance {
    pub id: String,
    pub created_at: f64,
    pub label: String,
    pub config: IssuanceConfig,
    pub status: IssuanceStatus,
    pub mnemonic_words: String,
    pub ecash_notes: Vec<String>,
    pub total_amount_msat: u64,
}

impl Issuance {
    pub fn total_amount_sats(&self) -> u64 {
        self.total_amount_msat / 1000
    }

    pub fn per_note_amount_msat(&self) -> u64 {
        self.config.denominations_msat.iter().sum()
    }

    pub fn per_note_amount_sats(&self) -> u64 {
        self.per_note_amount_msat() / 1000
    }
}
