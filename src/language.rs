use anyhow::Result;
use lingua::{LanguageDetector, LanguageDetectorBuilder};
use std::sync::{Arc, OnceLock};

/// Language detection service for English.
///
/// Thread-safe singleton via `OnceLock`. Expensive detector built once on first use.
pub struct LanguageService {
    detector: Arc<LanguageDetector>,
}

static DETECTOR: OnceLock<Arc<LanguageDetector>> = OnceLock::new();

impl Default for LanguageService {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageService {
    /// Create new language service. Shares singleton detector across all instances.
    pub fn new() -> Self {
        let detector = DETECTOR.get_or_init(|| {
            use lingua::Language::{English, French, German, Polish, Spanish};
            let d = LanguageDetectorBuilder::from_languages(&[
                English, French, German, Polish, Spanish,
            ])
            .with_minimum_relative_distance(0.4)
            .with_preloaded_language_models()
            .build();
            Arc::new(d)
        });
        Self {
            detector: Arc::clone(detector),
        }
    }

    /// Detect if single text is English.
    ///
    /// Returns `true` if detected language is English with confidence > 0.5.
    pub async fn detect(&self, text: &str) -> Result<bool> {
        let detector = self.detector.clone();
        let text = text.to_string();
        tokio::task::spawn_blocking(move || detect_one(&detector, &text))
            .await
            .map_err(|e| anyhow::anyhow!("language detection task panicked: {e}"))
    }
}

fn detect_one(detector: &LanguageDetector, text: &str) -> bool {
    let detected = detector.detect_language_of(text);
    let confidence = detected.map_or(0.0, |lang| detector.compute_language_confidence(text, lang));
    detected.is_some_and(|l| l.iso_code_639_1().to_string() == "en" && confidence > 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_polish_detection() {
        let svc = LanguageService::new();
        let text = r"Mini. 5+ lat doświadczenia w obszarze cloud / architektury IT
    Doświadczenie w pracy z Microsoft Azure, w szczególności: projektowanie struktury Management Groups, Subscriptions i Resource Groups, znajomość Azure RBAC oraz Azure Policy
    Doświadczenie w projektowaniu i wdrażaniu architektury chmurowej w środowisku enterprise";
        assert!(
            !svc.detect(text)
                .await
                .expect("language detection should succeed"),
            "Should NOT detect Polish job ad as English"
        );
    }

    #[tokio::test]
    async fn test_english_detection() {
        let svc = LanguageService::new();

        let english_cases = vec![
            "the",
            "programming",
            "I love programming",
            "Machine learning is a subset of artificial intelligence that enables computers to learn without being explicitly programmed",
            "This is a comprehensive explanation of machine learning algorithms and their applications in modern software development practices",
        ];

        for text in english_cases {
            assert!(
                svc.detect(text)
                    .await
                    .expect("language detection should succeed"),
                "Should detect as English: \"{text}\""
            );
        }
    }

    #[tokio::test]
    async fn test_non_english_detection() {
        let svc = LanguageService::new();

        let non_english_cases = vec![
            "Bonjour",
            "Guten Tag",
            "Hola",
            "Soy un desarrollador de software y me gusta trabajar con tecnologías modernas",
            "Je suis développeur de logiciels et j'aime travailler avec des technologies modernes",
            "Ich bin Softwareentwickler und arbeite gerne mit modernen Technologien",
            "Ciao",
            "Sono uno sviluppatore di software e mi piace lavorare con tecnologie moderne",
            "Hallo",
            "Ik ben een softwareontwikkelaar en ik werk graag met moderne technologieën",
            "Hej",
            "Jag är en mjukvaruutvecklare och jag gillar att arbeta med moderna teknologier",
            "Привет",
            "Я разработчик программного обеспечения и люблю работать с современными технологиями",
            "Cześć",
            "Jestem programistą i lubię pracować z nowoczesnymi technologiami",
            "こんにちは",
            "私はソフトウェア開発者で、最新のテクノロジーで働くのが好きです",
            "안녕하세요",
            "저는 소프트웨어 개발자이고 현대 기술로 일하는 것을 좋아합니다",
            "नमस्ते",
            "मैं एक सॉफ्टवेयर डेवलपर हूं और आधुनिक तकनीकों के साथ काम करना पसंद करता हूं",
            "สวัสดี",
            "ฉันเป็นนักพัฒนาซอฟต์แวร์และชอบทำงานกับเทคโนโลยีสมัยใหม่",
            "مرحبا",
            "أنا مطور برمجيات وأحب العمل مع التقنيات الحديثة",
            "שלום",
            "אני מפתח תוכנה ואני אוהב לעבוד עם טכנולוגיות מודרניות",
            "merhaba",
            "Yazılım geliştiricisiyim ve modern teknolojilerle çalışmayı seviyorum",
            "sawubona",
            "Ngiyisiphakeli se-software futhi ngiyathanda ukusebenza nobuchwepheshe besimanje",
            "hello",
            "hi",
            "TIL",
            "AITA",
        ];

        for text in non_english_cases {
            assert!(
                !svc.detect(text)
                    .await
                    .expect("language detection should succeed"),
                "Should NOT detect as English: \"{text}\""
            );
        }
    }

    #[tokio::test]
    async fn test_singleton_reuse() {
        let svc1 = LanguageService::new();
        let svc2 = LanguageService::new();

        assert!(
            svc1.detect("the")
                .await
                .expect("language detection should succeed")
        );
        assert!(
            svc2.detect("programming")
                .await
                .expect("language detection should succeed")
        );

        let ptr1 = Arc::as_ptr(&svc1.detector);
        let ptr2 = Arc::as_ptr(&svc2.detector);
        assert_eq!(ptr1, ptr2, "Same detector instance");
    }
}
