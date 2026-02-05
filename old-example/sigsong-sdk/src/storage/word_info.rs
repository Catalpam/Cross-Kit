use crate::result::{SDKError, SDKResult};
use crate::server::proto::word_info::{WordInfo, WordMeaning, WordSense};
use sqlx::{Row, SqlitePool};

pub struct WordInfoStorage<'a> {
    pool: &'a SqlitePool,
}

impl<'a> WordInfoStorage<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn fetch(&self, word: &str) -> SDKResult<Option<Vec<WordInfo>>> {
        let entry_rows = sqlx::query(
            "SELECT id, word_info_id, word, pronounce, tone, normalized_form, form_desc \
             FROM word_entries WHERE search_key = ? ORDER BY word_info_id",
        )
        .bind(word)
        .fetch_all(self.pool)
        .await
        .map_err(|err| SDKError::database(format!("load cached word infos for {word}"), err))?;

        if entry_rows.is_empty() {
            return Ok(None);
        }

        let mut results = Vec::with_capacity(entry_rows.len());

        for entry_row in entry_rows {
            let entry_id: i64 = entry_row
                .try_get("id")
                .map_err(|err| SDKError::database("read word entry id", err))?;
            let word_info_id: i64 = entry_row
                .try_get("word_info_id")
                .map_err(|err| SDKError::database("read word info id", err))?;
            let word_text: String = entry_row
                .try_get("word")
                .map_err(|err| SDKError::database("read word text", err))?;
            let pronounce: String = entry_row
                .try_get("pronounce")
                .map_err(|err| SDKError::database("read pronounce", err))?;
            let tone_raw: String =
                entry_row.try_get("tone").map_err(|err| SDKError::database("read tone", err))?;
            let normalized: Option<String> = entry_row
                .try_get("normalized_form")
                .map_err(|err| SDKError::database("read normalized form", err))?;
            let form_desc: Option<String> = entry_row
                .try_get("form_desc")
                .map_err(|err| SDKError::database("read form desc", err))?;

            let tone: Vec<i32> =
                serde_json::from_str(&tone_raw).map_err(|err| SDKError::Other {
                    message: format!("failed to deserialize tone for {word_text}: {err}"),
                })?;

            let sense_rows = sqlx::query(
                "SELECT id, part FROM word_senses WHERE entry_id = ? ORDER BY position",
            )
            .bind(entry_id)
            .fetch_all(self.pool)
            .await
            .map_err(|err| SDKError::database("load word senses", err))?;

            let mut senses = Vec::with_capacity(sense_rows.len());
            for sense_row in sense_rows {
                let sense_id: i64 = sense_row
                    .try_get("id")
                    .map_err(|err| SDKError::database("read sense id", err))?;
                let part: String = sense_row
                    .try_get("part")
                    .map_err(|err| SDKError::database("read sense part", err))?;

                let meaning_rows = sqlx::query(
                    "SELECT id, meaning FROM word_meanings WHERE sense_id = ? ORDER BY position",
                )
                .bind(sense_id)
                .fetch_all(self.pool)
                .await
                .map_err(|err| SDKError::database("load word meanings", err))?;

                let mut meanings = Vec::with_capacity(meaning_rows.len());
                for meaning_row in meaning_rows {
                    let meaning_id: i64 = meaning_row
                        .try_get("id")
                        .map_err(|err| SDKError::database("read meaning id", err))?;
                    let meaning_text: String = meaning_row
                        .try_get("meaning")
                        .map_err(|err| SDKError::database("read meaning text", err))?;

                    let example_rows = sqlx::query(
                        "SELECT example, translation FROM word_examples WHERE meaning_id = ? ORDER BY position",
                    )
                    .bind(meaning_id)
                    .fetch_all(self.pool)
                    .await
                    .map_err(|err| SDKError::database("load word examples", err))?;

                    let mut egs = Vec::with_capacity(example_rows.len());
                    let mut egts = Vec::with_capacity(example_rows.len());
                    for example_row in example_rows {
                        let example: Option<String> = example_row
                            .try_get("example")
                            .map_err(|err| SDKError::database("read example", err))?;
                        let translation: Option<String> = example_row
                            .try_get("translation")
                            .map_err(|err| SDKError::database("read example translation", err))?;
                        egs.push(example.unwrap_or_default());
                        egts.push(translation.unwrap_or_default());
                    }

                    meanings.push(WordMeaning { meaning: meaning_text, egs, egts });
                }

                senses.push(WordSense { part, meanings });
            }

            results.push(WordInfo {
                id: word_info_id as i32,
                word: word_text,
                pronounce,
                tone,
                cn_meaning: senses,
                normalized_form: normalized.unwrap_or_default(),
                form_desc: form_desc.unwrap_or_default(),
            });
        }

        Ok(Some(results))
    }

    pub async fn upsert(&self, word: &str, infos: &[WordInfo]) -> SDKResult<()> {
        let mut tx =
            self.pool.begin().await.map_err(|err| SDKError::database("begin transaction", err))?;

        sqlx::query("DELETE FROM word_entries WHERE search_key = ?")
            .bind(word)
            .execute(&mut *tx)
            .await
            .map_err(|err| SDKError::database("clear previous word entries", err))?;

        for info in infos {
            let tone_json = serde_json::to_string(&info.tone).map_err(|err| SDKError::Other {
                message: format!("failed to serialize tone for {}: {err}", info.word),
            })?;

            let entry_row = sqlx::query(
                "INSERT INTO word_entries (search_key, word_info_id, word, pronounce, tone, normalized_form, form_desc) \
                 VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
            )
            .bind(word)
            .bind(info.id as i64)
            .bind(&info.word)
            .bind(&info.pronounce)
            .bind(&tone_json)
            .bind(
                if info.normalized_form.is_empty() {
                    None::<String>
                } else {
                    Some(info.normalized_form.clone())
                },
            )
            .bind(
                if info.form_desc.is_empty() {
                    None::<String>
                } else {
                    Some(info.form_desc.clone())
                },
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| SDKError::database("insert word entry", err))?;

            let entry_id: i64 = entry_row
                .try_get("id")
                .map_err(|err| SDKError::database("read inserted entry id", err))?;

            for (sense_index, sense) in info.cn_meaning.iter().enumerate() {
                let sense_row = sqlx::query(
                    "INSERT INTO word_senses (entry_id, position, part) VALUES (?, ?, ?) RETURNING id",
                )
                .bind(entry_id)
                .bind(sense_index as i64)
                .bind(&sense.part)
                .fetch_one(&mut *tx)
                .await
                .map_err(|err| SDKError::database("insert word sense", err))?;

                let sense_id: i64 = sense_row
                    .try_get("id")
                    .map_err(|err| SDKError::database("read inserted sense id", err))?;

                for (meaning_index, meaning) in sense.meanings.iter().enumerate() {
                    let meaning_row = sqlx::query(
                        "INSERT INTO word_meanings (sense_id, position, meaning) VALUES (?, ?, ?) RETURNING id",
                    )
                    .bind(sense_id)
                    .bind(meaning_index as i64)
                    .bind(&meaning.meaning)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|err| SDKError::database("insert word meaning", err))?;

                    let meaning_id: i64 = meaning_row
                        .try_get("id")
                        .map_err(|err| SDKError::database("read inserted meaning id", err))?;

                    let examples_len = meaning.egs.len();
                    for idx in 0..examples_len {
                        let example = meaning.egs.get(idx).cloned().unwrap_or_default();
                        let translation = meaning.egts.get(idx).cloned().unwrap_or_default();

                        sqlx::query(
                            "INSERT INTO word_examples (meaning_id, position, example, translation) VALUES (?, ?, ?, ?)",
                        )
                        .bind(meaning_id)
                        .bind(idx as i64)
                        .bind(if example.is_empty() { None::<String> } else { Some(example) })
                        .bind(if translation.is_empty() { None::<String> } else { Some(translation) })
                        .execute(&mut *tx)
                        .await
                        .map_err(|err| SDKError::database("insert word example", err))?;
                    }
                }
            }
        }

        tx.commit().await.map_err(|err| SDKError::database("commit word info transaction", err))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_word_info() -> WordInfo {
        WordInfo {
            id: 42,
            word: "描いた".to_string(),
            pronounce: "えがいた".to_string(),
            tone: vec![1, 0],
            cn_meaning: vec![WordSense {
                part: "動詞".to_string(),
                meanings: vec![WordMeaning {
                    meaning: "描绘".to_string(),
                    egs: vec!["夢を描いた".to_string()],
                    egts: vec!["描绘了梦想".to_string()],
                }],
            }],
            normalized_form: "描く".to_string(),
            form_desc: "连体形".to_string(),
        }
    }

    #[tokio::test]
    async fn structured_word_info_roundtrip() {
        let pool = SqlitePool::connect("sqlite::memory:").await.expect("connect in-memory sqlite");

        sqlx::migrate!().run(&pool).await.expect("run migrations");

        let storage = WordInfoStorage::new(&pool);
        let entry = sample_word_info();

        storage.upsert("描いた", &[entry.clone()]).await.expect("upsert word info");

        let fetched = storage
            .fetch("描いた")
            .await
            .expect("fetch word info")
            .expect("word info should exist");

        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].word, entry.word);
        assert_eq!(fetched[0].cn_meaning.len(), 1);
        assert_eq!(fetched[0].cn_meaning[0].meanings.len(), 1);
        assert_eq!(
            fetched[0].cn_meaning[0].meanings[0].egs.first().map(String::as_str),
            Some("夢を描いた")
        );
        assert_eq!(
            fetched[0].cn_meaning[0].meanings[0].egts.first().map(String::as_str),
            Some("描绘了梦想")
        );
    }
}
