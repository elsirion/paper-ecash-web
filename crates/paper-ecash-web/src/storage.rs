use gloo_storage::{LocalStorage, Storage};
use tracing::warn;

use crate::models::Issuance;

const ISSUANCES_KEY: &str = "paper-ecash.issuances";

pub fn load_issuances() -> Vec<Issuance> {
    LocalStorage::get::<Vec<Issuance>>(ISSUANCES_KEY).unwrap_or_default()
}

pub fn save_issuances(issuances: &[Issuance]) {
    if let Err(e) = LocalStorage::set(ISSUANCES_KEY, issuances) {
        warn!("Failed to save issuances to localStorage: {e}");
    }
}

pub fn save_issuance(issuance: &Issuance) {
    let mut all = load_issuances();
    if let Some(existing) = all.iter_mut().find(|i| i.id == issuance.id) {
        *existing = issuance.clone();
    } else {
        all.push(issuance.clone());
    }
    save_issuances(&all);
}

pub fn load_issuance(id: &str) -> Option<Issuance> {
    load_issuances().into_iter().find(|i| i.id == id)
}

pub fn delete_issuance(id: &str) {
    let all: Vec<Issuance> = load_issuances()
        .into_iter()
        .filter(|i| i.id != id)
        .collect();
    save_issuances(&all);
}
