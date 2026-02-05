use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::analysis::Mode;
use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::MorphemeList;
use sudachi::sentence_splitter::{SentenceSplitter, SplitSentences};

use crate::logic::error::{SDKError, SDKResult};

pub struct AnalyzedWord {
    pub word: String,
    pub word_type_desc: String,
    pub normalized_form: String,
    pub dictionary_form: String,
    pub reading_form: String,
}

pub trait Analysis {
    fn analyze(&mut self, input: &str) -> SDKResult<Vec<AnalyzedWord>>;
    fn set_subset(&mut self, subset: InfoSubset);
}

pub struct AnalyzeNonSplitted<D: DictionaryAccess> {
    analyzer: StatefulTokenizer<D>,
    morphemes: MorphemeList<D>,
}

impl<D: DictionaryAccess + Clone> AnalyzeNonSplitted<D> {
    pub fn new(dict: D) -> Self {
        Self {
            morphemes: MorphemeList::empty(dict.clone()),
            analyzer: StatefulTokenizer::create(dict, false, Mode::B),
        }
    }
}

impl<D: DictionaryAccess> Analysis for AnalyzeNonSplitted<D> {
    fn analyze(&mut self, input: &str) -> SDKResult<Vec<AnalyzedWord>> {
        self.analyzer.reset().push_str(input);
        // Tokenize分词
        if let Err(e) = self.analyzer.do_tokenize() {
            return Err(SDKError::split_error(
                format!("tokenization failed, input: {}", input).as_str(),
                e,
            ));
        }
        // 收集字典解析结果
        if let Err(e) = self.morphemes.collect_results(&mut self.analyzer) {
            return Err(SDKError::split_error("result collection failed", e));
        }

        let words = self
            .morphemes
            .iter()
            .map(|m| {
                let word_type_desc = m.part_of_speech()[0].clone();
                AnalyzedWord {
                    word: m.surface().to_string(),
                    word_type_desc,
                    normalized_form: m.normalized_form().to_string(),
                    dictionary_form: m.dictionary_form().to_string(),
                    reading_form: m.reading_form().to_string(),
                }
            })
            .collect();
        Ok(words)
    }

    fn set_subset(&mut self, subset: InfoSubset) {
        self.analyzer.set_subset(subset);
    }
}

pub struct AnalyzeSplitted<'a, D: DictionaryAccess + 'a> {
    splitter: SentenceSplitter<'a>,
    inner: AnalyzeNonSplitted<&'a D>,
}

impl<'a, D: DictionaryAccess + 'a> AnalyzeSplitted<'a, D> {
    pub fn new(dict: &'a D) -> Self {
        Self {
            inner: AnalyzeNonSplitted::new(dict),
            splitter: SentenceSplitter::new().with_checker(dict.lexicon()),
        }
    }
}

impl<'a, D: DictionaryAccess + 'a> Analysis for AnalyzeSplitted<'a, D> {
    fn analyze(&mut self, input: &str) -> SDKResult<Vec<AnalyzedWord>> {
        let mut word_results = vec![];
        for (_, sent) in self.splitter.split(input) {
            word_results.extend(self.inner.analyze(sent)?);
        }
        Ok(word_results)
    }

    fn set_subset(&mut self, subset: InfoSubset) {
        self.inner.set_subset(subset)
    }
}
