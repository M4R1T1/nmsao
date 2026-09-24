use crate::logging::log_message;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use ww2ogg::{CodebookLibrary, WwiseRiffVorbis, validate};

// predetermined proper wwise encoded 1ms of silence
const SILENCE_WEM_DATA: &[u8] = &[
    0x52, 0x49, 0x46, 0x46, 0x27, 0x01, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6D, 0x74, 0x20,
    0x42, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x01, 0x00, 0x44, 0xAC, 0x00, 0x00, 0x43, 0x32, 0x03, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x00, 0x2C, 0x00, 0x00, 0x00,
    0xCB, 0x00, 0x00, 0x00, 0xD1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00,
    0xCB, 0x00, 0x00, 0x00, 0x01, 0x00, 0x14, 0x02, 0x04, 0x47, 0x00, 0x00, 0xCC, 0x48, 0x00, 0x00,
    0x48, 0xA4, 0xDE, 0xB3, 0x08, 0x0B, 0x64, 0x61, 0x74, 0x61, 0xD1, 0x00, 0x00, 0x00, 0xC9, 0x00,
    0x26, 0x26, 0x9C, 0x80, 0x42, 0x0A, 0x2A, 0xAC, 0xC0, 0x42, 0x0B, 0x2E, 0xBC, 0x00, 0x43, 0x0C,
    0x3E, 0xFC, 0x00, 0x44, 0x10, 0x42, 0x0C, 0x41, 0x44, 0x11, 0x46, 0x1C, 0x81, 0x44, 0x12, 0x4A,
    0x2C, 0xC1, 0x04, 0x38, 0xD6, 0x5C, 0x83, 0x4D, 0x36, 0xDA, 0x6C, 0xC3, 0x4D, 0x37, 0xDE, 0x7C,
    0x53, 0x4D, 0x40, 0x20, 0x64, 0x02, 0x81, 0x02, 0x28, 0x30, 0x90, 0x01, 0x00, 0x07, 0x08, 0x09,
    0x52, 0x00, 0x40, 0x61, 0x81, 0xA1, 0x43, 0x84, 0x08, 0x10, 0xA3, 0xC0, 0xC0, 0xB8, 0xB8, 0xB4,
    0x08, 0x42, 0x64, 0x86, 0x48, 0x44, 0x2C, 0x06, 0x89, 0x09, 0xD5, 0x40, 0x51, 0x31, 0x1D, 0x00,
    0x2C, 0x2E, 0x30, 0xE4, 0x03, 0x40, 0x86, 0xC6, 0x46, 0xDA, 0xC5, 0x05, 0x74, 0x19, 0xE0, 0x82,
    0x2E, 0xEE, 0x3A, 0x10, 0x42, 0x10, 0x82, 0x10, 0xC4, 0xE2, 0x00, 0x0A, 0x48, 0xC0, 0xC1, 0x09,
    0x37, 0x3C, 0xF1, 0x86, 0x27, 0xDC, 0xE0, 0x04, 0x9D, 0xA2, 0x52, 0x07, 0x01, 0x00, 0x00, 0xC0,
    0x01, 0x00, 0x3C, 0x00, 0x00, 0x1C, 0x1B, 0x40, 0x44, 0x44, 0x73, 0x1C, 0x1D, 0x1E, 0x1F, 0x20,
    0x21, 0x22, 0x23, 0x24, 0x25, 0x01, 0x00, 0x00, 0x80, 0x0D, 0x00, 0x7C, 0x00, 0x00, 0x1C, 0x26,
    0x40, 0x44, 0x44, 0x73, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x40, 0x40, 0x40, 0x00, 0x60, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01,
];

// part of wem header
const WEM_AUDIO_FORMAT: u16 = 0xFFFE;

// part of wem header
const JUNK_DATA: [u8; 20] = [
    0x06, 0x00, 0x00, 0x00,
    0x01, 0x41, 0x00, 0x00,
    0x4A, 0x55, 0x4E, 0x4B,
    0x04, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

// writes predetermined Silence1ms.wem file to be used as SILENCE dependency
pub(crate) fn create_silence_wem(deps_dir: &Path) -> anyhow::Result<()> {
    let target_path = deps_dir.join("Silence1ms.wem");
    std::fs::write(&target_path, SILENCE_WEM_DATA)?;
    log_message(&format!("Created Silence1ms.wem at {:?}", target_path));
    Ok(())
}

// wem to ogg via ww2ogg-rs
pub(crate) fn convert_wem_to_ogg(input_path: &Path, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let input_data = std::fs::read(input_path)?;
    let ogg_data = match try_with_codebooks(&input_data, false) {
        Ok(data) if validate(&data).is_ok() => data,
        _ => {
            let data = try_with_codebooks(&input_data, true)?;
            if let Err(e) = validate(&data) {
                log_message(&format!(
                    "Warning: {} decoded with imperfect validation: {}",
                    input_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    e
                ));
            }
            data
        }
    };
    // let result = try_with_codebooks(&input_data, false);
    // let ogg_data = match result {
    //     Ok(data) if validate(&data).is_ok() => data,
    //     _ => {
    //         let data = try_with_codebooks(&input_data, true)?;
    //         validate(&data).map_err(|e| format!("validation failed: {}", e))?;
    //         data
    //     }
    // };
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, ogg_data)?;
    Ok(())
}

// runs ww2ogg with either default or AoTuV codebooks
pub(crate) fn try_with_codebooks(input: &[u8], aotuv: bool) -> Result<Vec<u8>, ww2ogg::WemError> {
    let codebooks = if aotuv {
        CodebookLibrary::aotuv_codebooks()?
    } else {
        CodebookLibrary::default_codebooks()?
    };
    let cursor = std::io::Cursor::new(input);
    let mut converter = WwiseRiffVorbis::new(cursor, codebooks)?;
    let mut output = Vec::new();
    converter.generate_ogg(&mut output)?;
    Ok(output)
}

// wraps wav pcm data in wwise wem headers
pub(crate) fn convert_wav_to_wem_bytes(file_data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut cursor = Cursor::new(file_data);
    let mut riff_id = [0u8; 4];
    cursor.read_exact(&mut riff_id)?;
    let _riff_size = cursor.read_u32::<LittleEndian>()?;
    let mut wave_id = [0u8; 4];
    cursor.read_exact(&mut wave_id)?;
    if &riff_id != b"RIFF" || &wave_id != b"WAVE" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Input file is not a valid WAV",
        ));
    }
    let mut fmt_chunk_data = Vec::new();
    let mut data_chunk_data = Vec::new();
    let mut data_chunk_size = 0u32;
    loop {
        let mut chunk_id = [0u8; 4];
        if cursor.read_exact(&mut chunk_id).is_err() {
            break;
        }
        let chunk_size = cursor.read_u32::<LittleEndian>()?;
        let _chunk_start = cursor.position();
        if &chunk_id == b"fmt " {
            let mut buf = vec![0u8; chunk_size as usize];
            cursor.read_exact(&mut buf)?;
            fmt_chunk_data = buf;
        } else if &chunk_id == b"data" {
            data_chunk_size = chunk_size;
            let mut buf = vec![0u8; chunk_size as usize];
            cursor.read_exact(&mut buf)?;
            data_chunk_data = buf;
            break;
        } else {
            cursor.seek(SeekFrom::Current(chunk_size as i64))?;
        }
    }
    if fmt_chunk_data.is_empty() || data_chunk_data.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Missing required 'fmt ' or 'data' chunk",
        ));
    }
    let mut fmt_cursor = Cursor::new(&fmt_chunk_data);
    let _audio_format = fmt_cursor.read_u16::<LittleEndian>()?;
    let channels = fmt_cursor.read_u16::<LittleEndian>()?;
    let sample_rate = fmt_cursor.read_u32::<LittleEndian>()?;
    let byte_rate = fmt_cursor.read_u32::<LittleEndian>()?;
    let block_align = fmt_cursor.read_u16::<LittleEndian>()?;
    let bits_per_sample = fmt_cursor.read_u16::<LittleEndian>()?;
    let mut fmt_data = Vec::new();
    fmt_data.write_u16::<LittleEndian>(WEM_AUDIO_FORMAT)?;
    fmt_data.write_u16::<LittleEndian>(channels)?;
    fmt_data.write_u32::<LittleEndian>(sample_rate)?;
    fmt_data.write_u32::<LittleEndian>(byte_rate)?;
    fmt_data.write_u16::<LittleEndian>(block_align)?;
    fmt_data.write_u16::<LittleEndian>(bits_per_sample)?;
    fmt_data.extend_from_slice(&JUNK_DATA);

    // 16 byte waveformatex + 8 byte cbsize + extension header
    const FAKE_FMT_SIZE: u32 = 24;
    let mut output = Vec::new();
    let total_wem_size = 4 + 8 + 36 + 8 + data_chunk_size;
    output.extend_from_slice(b"RIFF");
    output.write_u32::<LittleEndian>(total_wem_size)?;
    output.extend_from_slice(b"WAVE");
    output.extend_from_slice(b"fmt ");
    output.write_u32::<LittleEndian>(FAKE_FMT_SIZE)?;
    output.extend_from_slice(&fmt_data);
    output.extend_from_slice(b"data");
    output.write_u32::<LittleEndian>(data_chunk_size)?;
    output.extend_from_slice(&data_chunk_data);
    Ok(output)
}

// file path wrapper for convert_wav_to_wem_bytes
pub(crate) fn convert_wav_to_wem(input_path: &str, output_path: &str) -> std::io::Result<()> {
    let file_data = std::fs::read(input_path)?;
    let bytes = convert_wav_to_wem_bytes(&file_data)?;
    std::fs::write(output_path, bytes)?;
    Ok(())
}

// reads sample rate from wem file fmt chunk
pub(crate) fn wem_sample_rate(path: &Path) -> Option<u32> {
    let data = std::fs::read(path).ok()?;
    let mut cursor = Cursor::new(&data);
    let mut riff = [0u8; 4];
    cursor.read_exact(&mut riff).ok()?;
    let is_le = &riff == b"RIFF";

    // rifx check for future proofing
    if &riff != b"RIFF" && &riff != b"RIFX" { return None; }
    let _ = cursor.read_u32::<LittleEndian>().ok()?; // size, unused
    let mut wave = [0u8; 4];
    cursor.read_exact(&mut wave).ok()?;
    if &wave != b"WAVE" { return None; }
    loop {
        let mut id = [0u8; 4];
        if cursor.read_exact(&mut id).is_err() { return None; }
        let size = cursor.read_u32::<LittleEndian>().ok()?;
        if &id == b"fmt " {
            let mut fmt = vec![0u8; size as usize];
            cursor.read_exact(&mut fmt).ok()?;
            if fmt.len() < 8 { return None; }
            let sr = if is_le {
                u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]])
            } else {
                u32::from_be_bytes([fmt[4], fmt[5], fmt[6], fmt[7]])
            };
            return if sr > 0 { Some(sr) } else { None };
        }
        cursor.seek(SeekFrom::Current(size as i64)).ok()?;
    }
}

// returns granulepos of ogg stream EOS page (total samples per channel)
pub(crate) fn ogg_total_samples(ogg_bytes: &[u8]) -> Option<u64> {
    let mut last_granule: Option<u64> = None;
    let mut i = 0usize;
    while i + 27 <= ogg_bytes.len() {
        if &ogg_bytes[i..i + 4] != b"OggS" {
            i += 1;
            continue;
        }
        let header_type = ogg_bytes[i + 5];
        let granule = u64::from_le_bytes([
            ogg_bytes[i + 6],  ogg_bytes[i + 7],
            ogg_bytes[i + 8],  ogg_bytes[i + 9],
            ogg_bytes[i + 10], ogg_bytes[i + 11],
            ogg_bytes[i + 12], ogg_bytes[i + 13],
        ]);
        if granule != u64::MAX {
            last_granule = Some(granule);
            if header_type & 0x04 != 0 {
                return Some(granule);
            }
        }
        let num_segments = ogg_bytes[i + 26] as usize;
        if i + 27 + num_segments > ogg_bytes.len() { break; }
        let mut body_size = 0usize;
        for s in 0..num_segments {
            body_size += ogg_bytes[i + 27 + s] as usize;
        }
        i += 27 + num_segments + body_size;
    }
    last_granule
}