use wana_kana::{ConvertJapanese, IsJapaneseStr};

pub(crate) mod diff;

#[derive(Debug, Clone, uniffi::Record)]
pub struct PronouncedString {
    pub original: String,
    pub pronunciation: Option<String>,
}

impl PronouncedString {
    #[allow(unused)]
    fn new() -> Self {
        Self { original: "".to_string(), pronunciation: None }
    }
}

pub fn spot(origin: &str, pronounce: &str) -> Vec<PronouncedString> {
    if origin.is_empty() && pronounce.is_empty() {
        return vec![];
    }

    // 只给汉字注音
    if !origin.contains_kanji() {
        return vec![PronouncedString { original: origin.to_string(), pronunciation: None }];
    }

    let pronounce = if pronounce.is_hiragana() { pronounce } else { &*pronounce.to_hiragana() };
    #[derive(PartialEq)]
    enum Type {
        Init,
        Same,
        Different,
    }

    let mut current_type = Type::Init;
    let mut result: Vec<PronouncedString> = vec![];

    for diff in diff::chars(origin, pronounce) {
        match diff {
            diff::Result::Left(l) => {
                if current_type != Type::Different {
                    result.push(PronouncedString::new());
                    current_type = Type::Different;
                }
                result.last_mut().unwrap().original.push(l);
            },
            diff::Result::Both(l, _) => {
                if current_type != Type::Same {
                    result.push(PronouncedString::new());
                    current_type = Type::Same;
                }
                result.last_mut().unwrap().original.push(l);
            },
            diff::Result::Right(r) => {
                if current_type != Type::Different {
                    result.push(PronouncedString::new());
                    current_type = Type::Different;
                }
                if let Some(pronunciation) = &mut result.last_mut().unwrap().pronunciation {
                    pronunciation.push(r);
                } else {
                    result.last_mut().unwrap().pronunciation = Some(r.to_string());
                }
            },
        }
    }
    result
}

// let res = kakasi::convert("hello描いた地図は引き裂いた!アラサ");
// println!("🚀rust_log: {}", res.hiragana);
