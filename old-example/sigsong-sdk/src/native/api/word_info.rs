use crate::cache;
use crate::cache::word_info::CLWordInfo;
use crate::client::invoke::InvokeManager;
use crate::result::SDKResult;
use crate::server::proto::word_info::WordInfo;

#[uniffi::export]
impl InvokeManager {
    pub fn get_word_info(&self, word: String) -> SDKResult<Vec<CLWordInfo>> {
        self.rt.block_on(cache::word_info::get_word_info(&self.pool, &word))
    }
}
