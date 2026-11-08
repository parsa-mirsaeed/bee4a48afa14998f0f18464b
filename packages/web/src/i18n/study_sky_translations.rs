use super::Locale;

pub(crate) fn study_sky_translation(key: &'static str, locale: Locale) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "student.sky.eyebrow") => Some("Study Sky"),
        (Locale::Fa, "student.sky.eyebrow") => Some("آسمان مطالعه"),
        (Locale::En, "student.sky.lit_caption") => Some("stars lit"),
        (Locale::Fa, "student.sky.lit_caption") => Some("ستاره روشن"),
        (Locale::En, "student.sky.legend_label") => Some("Star key"),
        (Locale::Fa, "student.sky.legend_label") => Some("راهنمای ستاره‌ها"),
        (Locale::En, "student.sky.up_next") => Some("Up next"),
        (Locale::Fa, "student.sky.up_next") => Some("کار بعدی"),
        (Locale::En, "student.sky.empty") => Some("Your sky is waiting for its first star."),
        (Locale::Fa, "student.sky.empty") => Some("آسمان شما منتظر اولین ستاره است."),
        (Locale::En, "student.sky.more_hidden") => Some("more assignments not drawn"),
        (Locale::Fa, "student.sky.more_hidden") => Some("تکلیف دیگر نمایش داده نشده"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKY_KEYS: &[&str] = &[
        "student.sky.eyebrow",
        "student.sky.lit_caption",
        "student.sky.legend_label",
        "student.sky.up_next",
        "student.sky.empty",
        "student.sky.more_hidden",
    ];

    #[test]
    fn study_sky_copy_has_english_and_farsi_parity() {
        for key in SKY_KEYS {
            for locale in [Locale::En, Locale::Fa] {
                let translated = study_sky_translation(key, locale);
                assert!(
                    translated.is_some(),
                    "missing {locale:?} translation for {key}"
                );
                assert_ne!(translated, Some(*key), "raw key fallback for {key}");
            }
        }
    }
}
