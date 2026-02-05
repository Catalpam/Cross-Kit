use crate::logic::error::{SDKError, SDKResult};
use std::fs;
use std::path::PathBuf;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

pub fn get_dict<'a>(source_dir: PathBuf) -> SDKResult<JapaneseDictionary> {
    if !fs::metadata(source_dir.join("system.dict")).is_ok() {
        println!("🚀rust_log: 字典不存在");
        return Err(SDKError::DictionaryNotFound);
    }
    if !fs::metadata(source_dir.join("char.def")).is_ok() {
        println!("🚀rust_log: 字典不存在");
        return Err(SDKError::DictionaryNotFound);
    }
    if !fs::metadata(source_dir.join("rewrite.def")).is_ok() {
        println!("🚀rust_log: 字典不存在");
        return Err(SDKError::DictionaryNotFound);
    }

    println!("🚀rust_log: 开始构造分析器");
    let config = Config::sigsong_default(
        PathBuf::from(source_dir.clone()).join("system.dict"),
        PathBuf::from(source_dir.clone()),
    )
    .map_err(|error| {
        println!("A:{}", error);
        SDKError::Unknown
    })?;
    let dict = JapaneseDictionary::from_cfg(&config).map_err(|error| {
        println!("B:{}", error);
        SDKError::Unknown
    })?;
    println!("🚀rust_log: 词典导入成功");
    Ok(dict)
}
