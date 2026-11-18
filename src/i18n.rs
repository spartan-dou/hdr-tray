// Localisation français / anglais : la langue suit celle de l'interface Windows.
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

#[link(name = "kernel32")]
extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

static LANG: AtomicU8 = AtomicU8::new(0); // 0 = pas encore lu, 1 = français, 2 = anglais

fn is_french(lang_id: u16) -> bool {
    lang_id & 0xFF == 0x0C // LANG_FRENCH, quel que soit le sous-dialecte
}

pub fn french() -> bool {
    if LANG.load(Relaxed) == 0 {
        let fr = is_french(unsafe { GetUserDefaultUILanguage() });
        LANG.store(if fr { 1 } else { 2 }, Relaxed);
    }
    LANG.load(Relaxed) == 1
}

/// Renvoie le texte français ou anglais selon la langue de Windows.
pub fn tr(fr: &'static str, en: &'static str) -> &'static str {
    if french() {
        fr
    } else {
        en
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_french_dialects() {
        assert!(is_french(0x040C)); // fr-FR
        assert!(is_french(0x080C)); // fr-BE
        assert!(!is_french(0x0409)); // en-US
    }
}
