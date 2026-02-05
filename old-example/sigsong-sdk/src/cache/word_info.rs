use crate::result::{SDKError, SDKResult};
use crate::server;
use crate::server::proto::word_info::{WordInfo, WordMeaning, WordSense};
use crate::storage::word_info::WordInfoStorage;
use sqlx::SqlitePool;

#[derive(Clone, uniffi::Record)]
pub struct CLWordInfo {
    pub id: String,
    pub word: String,
    pub pronounce: String,
    pub tone: Vec<i32>,
    pub normalized_form: String,
    pub form_desc: String,
    pub senses: Vec<CLWordSense>,
}

#[derive(Clone, uniffi::Record)]
pub struct CLWordSense {
    pub part: String,
    pub meanings: Vec<CLWordMeaning>,
}

#[derive(Clone, uniffi::Record)]
pub struct CLWordMeaning {
    pub definition: String,
    pub examples: Vec<CLWordExample>,
}

#[derive(Clone, uniffi::Record)]
pub struct CLWordExample {
    pub sentence: String,
    pub translation: String,
}

impl WordInfo {
    fn into_cl(self) -> CLWordInfo {
        let senses = self.cn_meaning.into_iter().map(WordSense::into_cl).collect();

        CLWordInfo {
            id: self.id.to_string(),
            word: self.word,
            pronounce: self.pronounce,
            tone: self.tone,
            normalized_form: self.normalized_form,
            form_desc: self.form_desc,
            senses,
        }
    }
}

impl WordSense {
    fn into_cl(self) -> CLWordSense {
        let meanings = self.meanings.into_iter().map(WordMeaning::into_cl).collect();
        CLWordSense { part: self.part, meanings }
    }
}

impl WordMeaning {
    fn into_cl(self) -> CLWordMeaning {
        let examples = pair_examples(self.egs, self.egts)
            .into_iter()
            .map(|(sentence, translation)| CLWordExample { sentence, translation })
            .collect();

        CLWordMeaning { definition: self.meaning, examples }
    }
}

fn pair_examples(egs: Vec<String>, egts: Vec<String>) -> Vec<(String, String)> {
    let max_len = egs.len().max(egts.len());
    (0..max_len)
        .map(|idx| {
            let sentence = egs.get(idx).cloned().unwrap_or_default();
            let translation = egts.get(idx).cloned().unwrap_or_default();
            (sentence, translation)
        })
        .collect()
}

pub async fn get_word_info(pool: &SqlitePool, word: &str) -> SDKResult<Vec<CLWordInfo>> {
    let storage = WordInfoStorage::new(pool);

    if let Some(items) = storage.fetch(word).await? {
        return Ok(items.into_iter().map(WordInfo::into_cl).collect());
    }

    let response = server::api::word_info::get_word_info(word.to_string()).await?;
    storage.upsert(word, &response.word_infos).await?;

    let items = storage.fetch(word).await?.ok_or(SDKError::EmptyResponse)?;

    Ok(items.into_iter().map(WordInfo::into_cl).collect())
}

#[derive(Clone, uniffi::Enum)]
pub enum WordType {
    Verb,
    AuxiliaryVerb,
    Noun,
    Particle,
    Adnominal,
    Pronoun,
    Adverb,
    AdjectivalNoun,
    Adjective,
    Interjection,
    Suffix,
    Unknown,
}
