use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// didx chunk length
const DIDX_ENTRY_BYTES: u32 = 12;

#[derive(Debug)]
pub struct SoundBank {
    #[allow(dead_code)]
    version: u32,
    pub(crate) didx_entries: Vec<WemDescriptor>,
    data_section_start: u64,
    source_path: PathBuf,
    bkhd_header: [u8; 8],
    bkhd_data: Vec<u8>,
    hirc_section: Option<Vec<u8>>,
    other_sections: Vec<Vec<u8>>,
    data_section_data: Vec<u8>,
    section_order: Vec<SectionKind>,
}

#[derive(Debug)]
pub struct WemDescriptor {
    pub(crate) id: u32,
    pub(crate) offset: u32,
    pub(crate) size: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum SectionKind {
    Bkhd,
    Didx,
    Data,
    Hirc,
    Other(usize),
}

impl SoundBank {
    // parse bnk file into its sections
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let source_path = path.as_ref().to_path_buf();
        let mut file = File::open(&source_path)?;
        let mut version = 0;
        let mut didx_entries = Vec::new();
        let mut data_section_start = 0u64;
        let mut bkhd_header = [0u8; 8];
        let mut bkhd_data = Vec::new();
        let mut hirc_section = None;
        let mut other_sections = Vec::new();
        let mut data_section_data = Vec::new();
        let mut section_order = Vec::new();
        loop {
            let mut header = [0u8; 8];
            match file.read_exact(&mut header) {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let section_id = &header[0..4];
            let section_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
            
            // section order matters, write_to_path basically repeats this so output preserves original bnk structure
            match section_id {
                b"BKHD" => {
                    let mut data = vec![0u8; section_size as usize];
                    file.read_exact(&mut data)?;
                    if data.len() >= 4 {
                        version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    }
                    bkhd_header = header;
                    bkhd_data = data;
                    section_order.push(SectionKind::Bkhd);
                }
                b"DIDX" => {
                    let mut didx_data = vec![0u8; section_size as usize];
                    file.read_exact(&mut didx_data)?;
                    let mut cursor = std::io::Cursor::new(didx_data);
                    let entry_count = section_size / DIDX_ENTRY_BYTES;
                    for _ in 0..entry_count {
                        let id = cursor.read_u32::<LittleEndian>()?;
                        let offset = cursor.read_u32::<LittleEndian>()?;
                        let size = cursor.read_u32::<LittleEndian>()?;
                        didx_entries.push(WemDescriptor { id, offset, size });
                    }
                    section_order.push(SectionKind::Didx);
                }
                b"DATA" => {
                    data_section_start = file.stream_position()?;
                    let mut data = vec![0u8; section_size as usize];
                    file.read_exact(&mut data)?;
                    data_section_data = data;
                    section_order.push(SectionKind::Data);
                }
                _ => {
                    let mut section_body = vec![0u8; section_size as usize];
                    file.read_exact(&mut section_body)?;
                    let mut full_section = Vec::with_capacity(8 + section_size as usize);
                    full_section.extend_from_slice(&header);
                    full_section.extend_from_slice(&section_body);
                    if section_id == b"HIRC" {
                        hirc_section = Some(full_section);
                        section_order.push(SectionKind::Hirc);
                    } else {
                        let idx = other_sections.len();
                        other_sections.push(full_section);
                        section_order.push(SectionKind::Other(idx));
                    }
                }
            }
        }
        let has_bkhd = section_order.iter().any(|s| matches!(s, SectionKind::Bkhd));
        let has_didx = section_order.iter().any(|s| matches!(s, SectionKind::Didx));
        let has_data = section_order.iter().any(|s| matches!(s, SectionKind::Data));
        if !(has_bkhd && has_didx && has_data) {
            return Err(anyhow::anyhow!(
                "BNK {:?} is missing a required section (BKHD={}, DIDX={}, DATA={})",
                source_path, has_bkhd, has_didx, has_data
            ));
        }
        Ok(SoundBank {
            version,
            didx_entries,
            data_section_start,
            source_path,
            bkhd_header,
            bkhd_data,
            hirc_section,
            other_sections,
            data_section_data,
            section_order,
        })
    }
    // writes every wem in bnk
    pub(crate) fn extract_wems<P: AsRef<Path>>(&self, output_dir: P) -> anyhow::Result<()> {
        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;
        let mut file = File::open(&self.source_path)?;
        for desc in &self.didx_entries {
            let absolute_offset = self.data_section_start + desc.offset as u64;
            file.seek(SeekFrom::Start(absolute_offset))?;
            let mut buffer = vec![0u8; desc.size as usize];
            file.read_exact(&mut buffer)?;
            let output_file = output_dir.join(format!("{}.wem", desc.id));
            std::fs::write(&output_file, buffer)?;
        }
        Ok(())
    }
    // swaps payload for one wem id
    pub(crate) fn replace_wem(&mut self, target_id: u32, new_wem_data: &[u8]) -> Result<(), anyhow::Error> {
        self.didx_entries.iter_mut()
            .find(|d| d.id == target_id)
            .ok_or_else(|| anyhow::anyhow!("WEM Id {} not found", target_id))?;
        self.rebuild_data_section(new_wem_data, target_id)?;
        Ok(())
    }
    // reassemble data section with updated wem and recalculates offsets and sizes
    fn rebuild_data_section(&mut self, new_wem_data: &[u8], target_id: u32) -> Result<(), anyhow::Error> {
        let raw_data = self.data_section_data.clone();
        let mut new_data = Vec::new();
        let mut current_offset = 0u32;
        for desc in &mut self.didx_entries {
            let start = desc.offset as usize;
            let end = start + desc.size as usize;
            let wem_data = if desc.id == target_id {
                new_wem_data.to_vec()
            } else {
                raw_data[start..end].to_vec()
            };
            desc.offset = current_offset;
            desc.size = wem_data.len() as u32;
            new_data.extend_from_slice(&wem_data);
            current_offset += desc.size;
        }
        self.data_section_data = new_data;
        Ok(())
    }
    // serializes bnk back to disk
    pub(crate) fn write_to_path(&self, output_path: &Path) -> Result<(), anyhow::Error> {
        let mut file = File::create(output_path)?;
        for kind in &self.section_order {
            match kind {
                SectionKind::Bkhd => {
                    file.write_all(&self.bkhd_header)?;
                    file.write_all(&self.bkhd_data)?;
                }
                SectionKind::Didx => {
                    let mut didx_data = Vec::new();
                    for desc in &self.didx_entries {
                        didx_data.write_u32::<LittleEndian>(desc.id)?;
                        didx_data.write_u32::<LittleEndian>(desc.offset)?;
                        didx_data.write_u32::<LittleEndian>(desc.size)?;
                    }
                    file.write_all(b"DIDX")?;
                    file.write_u32::<LittleEndian>(didx_data.len() as u32)?;
                    file.write_all(&didx_data)?;
                }
                SectionKind::Data => {
                    file.write_all(b"DATA")?;
                    file.write_u32::<LittleEndian>(self.data_section_data.len() as u32)?;
                    file.write_all(&self.data_section_data)?;
                }
                SectionKind::Hirc => {
                    if let Some(hirc) = &self.hirc_section {
                        file.write_all(hirc)?;
                    }
                }
                SectionKind::Other(i) => {
                    if let Some(section) = self.other_sections.get(*i) {
                        file.write_all(section)?;
                    }
                }
            }
        }
        Ok(())
    }
    // returns raw bytes of one wem from data
    pub(crate) fn get_wem_bytes(&self, id: u32) -> Option<&[u8]> {
        let desc = self.didx_entries.iter().find(|d| d.id == id)?;
        let start = desc.offset as usize;
        let end = start + desc.size as usize;
        self.data_section_data.get(start..end)
    }
}