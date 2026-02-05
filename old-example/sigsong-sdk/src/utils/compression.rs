use bytes::Bytes;
use std::io::{Cursor, Read, Write};
use zstd::Decoder;
use zstd::Encoder;
use zstd::zstd_safe::max_c_level;

// 使用Zstd压缩数据,压缩效果设置为最高
pub fn compress(data: Vec<u8>) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = Encoder::new(Vec::new(), max_c_level())?;
    encoder.write_all(data.as_ref())?;
    encoder.finish()
}

// 使用Zstd解压缩数据
pub fn decompress(data: Vec<u8>) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = Decoder::new(Cursor::new(data))?;
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

pub fn decompress_bytes(data: Bytes) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = Decoder::new(Cursor::new(data.to_vec()))?;
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}
