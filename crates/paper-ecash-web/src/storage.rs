use gloo_storage::{LocalStorage, Storage};
use tracing::warn;

use crate::designs::{Design, DesignSource};
use crate::models::Issuance;

const ISSUANCES_KEY: &str = "paper-ecash.issuances";
const DESIGN_SOURCES_KEY: &str = "paper-ecash.design-sources";
const LOCAL_DESIGNS_KEY: &str = "paper-ecash.local-designs";

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

pub fn load_local_designs() -> Vec<Design> {
    LocalStorage::get::<Vec<Design>>(LOCAL_DESIGNS_KEY).unwrap_or_default()
}

pub fn save_local_designs(designs: &[Design]) {
    if let Err(e) = LocalStorage::set(LOCAL_DESIGNS_KEY, designs) {
        warn!("Failed to save local designs to localStorage: {e}");
    }
}

pub fn save_local_design(design: &Design) {
    let mut all = load_local_designs();
    if let Some(existing) = all.iter_mut().find(|d| d.id == design.id) {
        *existing = design.clone();
    } else {
        all.push(design.clone());
    }
    save_local_designs(&all);
}

pub fn delete_local_design(id: &str) {
    let all: Vec<Design> = load_local_designs()
        .into_iter()
        .filter(|d| d.id != id)
        .collect();
    save_local_designs(&all);
}

pub fn load_design_sources() -> Vec<DesignSource> {
    LocalStorage::get::<Vec<DesignSource>>(DESIGN_SOURCES_KEY).unwrap_or_default()
}

pub fn save_design_sources(sources: &[DesignSource]) {
    if let Err(e) = LocalStorage::set(DESIGN_SOURCES_KEY, sources) {
        warn!("Failed to save design sources to localStorage: {e}");
    }
}

pub fn delete_issuance(id: &str) {
    let all: Vec<Issuance> = load_issuances()
        .into_iter()
        .filter(|i| i.id != id)
        .collect();
    save_issuances(&all);
}
