use crate::db::types::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Veritabanının kalbi.
/// Tüm veriler bellekte (RAM) bu HashMap içinde duruyor.
pub struct DB {
    pub(crate) items: HashMap<String, Entry>,
    /// Son kayıttan bu yana yapılan değişiklik sayısı
    pub(crate) changes_since_save: Arc<AtomicUsize>,
}

impl DB {
    /// Yeni, tertemiz bir veritabanı oluşturur.
    pub fn new() -> Self {
        DB {
            items: HashMap::new(),
            changes_since_save: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Değişiklik sayacını artırır
    pub fn increment_changes(&self) {
        self.changes_since_save.fetch_add(1, Ordering::Relaxed);
    }

    /// Değişiklik sayacını sıfırlar (kayıt sonrası)
    pub fn reset_changes(&self) {
        self.changes_since_save.store(0, Ordering::Relaxed);
    }

    /// Son kayıttan bu yana yapılan değişiklik sayısını döndürür
    pub fn get_changes(&self) -> usize {
        self.changes_since_save.load(Ordering::Relaxed)
    }

    /// Değişiklik sayacının klonunu döndürür (background task için)
    pub fn get_changes_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.changes_since_save)
    }
}
