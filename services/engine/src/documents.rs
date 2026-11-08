//! Bounded, text-only Word import. No archive files are written or links followed.
use flate2::read::DeflateDecoder;
use quick_xml::{events::Event, Reader};
use std::io::Read;

const MAX_PART: usize = 16_000_000;
const MAX_TEXT: usize = 8_000_000;

fn word_xml(name: &str) -> bool {
    matches!(name, "word/document.xml" | "word/footnotes.xml" | "word/endnotes.xml")
        || (name.starts_with("word/header") || name.starts_with("word/footer"))
            && name.ends_with(".xml") && !name[5..].contains('/')
}
fn u16_at(data: &[u8], offset: usize) -> Result<usize, String> {
    let bytes: [u8; 2] = data.get(offset..offset + 2).ok_or("Truncated Word archive.")?.try_into().unwrap();
    Ok(u16::from_le_bytes(bytes) as usize)
}
fn u32_at(data: &[u8], offset: usize) -> Result<usize, String> {
    let bytes: [u8; 4] = data.get(offset..offset + 4).ok_or("Truncated Word archive.")?.try_into().unwrap();
    Ok(u32::from_le_bytes(bytes) as usize)
}
fn xml_text(xml: &[u8]) -> Result<String, String> {
    let mut reader = Reader::from_reader(xml);
    let mut text = String::new();
    loop {
        match reader.read_event().map_err(|e| format!("Invalid Word XML: {e}"))? {
            Event::Eof => break,
            Event::DocType(_) => return Err("Word XML document types are not supported.".into()),
            Event::Start(tag) if tag.local_name().as_ref() == "t" => {
                let raw = reader.read_text(tag.name()).map_err(|e| e.to_string())?;
                text.push_str(&quick_xml::escape::unescape(&raw).map_err(|e| e.to_string())?);
            }
            Event::Empty(tag) if matches!(tag.local_name().as_ref(), "br" | "cr") => text.push('\n'),
            Event::Empty(tag) if tag.local_name().as_ref() == "tab" => text.push('\t'),
            Event::End(tag) if matches!(tag.local_name().as_ref(), "p" | "tr") => text.push('\n'),
            Event::End(tag) if tag.local_name().as_ref() == "tc" => text.push('\t'),
            _ => {}
        }
        if text.len() > MAX_TEXT { return Err("Extracted document exceeds the 8 MB text limit. Split it into smaller documents.".into()); }
    }
    Ok(text)
}

/// Supports ordinary ZIP-based DOCX files with stored or deflated XML parts.
/// Central directory lengths, local headers, checksums and output limits are checked.
pub fn extract_docx_text(data: &[u8]) -> Result<String, String> {
    let eocd = (data.len().saturating_sub(65_557)..data.len().saturating_sub(21)).rev()
        .find(|&p| data.get(p..p+4) == Some(b"PK\x05\x06")
            && u16_at(data,p+20).is_ok_and(|n| p+22+n==data.len()))
        .ok_or("This is not a readable DOCX archive. Re-save it as .docx in your word processor.")?;
    let count = u16_at(data,eocd+10)?;
    if u16_at(data,eocd+4)? != 0 || u16_at(data,eocd+6)? != 0
        || u16_at(data,eocd+8)? != count || count == 0 || count > 8192 {
        return Err("Split, empty, ZIP64, or unusually large Word archives are not supported.".into());
    }
    let central_size = u32_at(data,eocd+12)?;
    let central_start = u32_at(data,eocd+16)?;
    let central_end = central_start.checked_add(central_size).filter(|n| *n <= eocd).ok_or("Invalid Word archive directory.")?;
    let mut position = central_start;
    let mut parts = Vec::new();
    let mut expanded = 0usize;
    for _ in 0..count {
        if data.get(position..position+4) != Some(b"PK\x01\x02") || position+46 > central_end { return Err("Invalid Word archive entry.".into()); }
        let flags=u16_at(data,position+8)?;
        let method=u16_at(data,position+10)?;
        let crc=u32_at(data,position+16)? as u32;
        let compressed=u32_at(data,position+20)?;
        let length=u32_at(data,position+24)?;
        let name_length=u16_at(data,position+28)?;
        let next=position.checked_add(46+name_length+u16_at(data,position+30)?+u16_at(data,position+32)?).filter(|n| *n <= central_end).ok_or("Invalid Word archive entry size.")?;
        let name=std::str::from_utf8(&data[position+46..position+46+name_length]).map_err(|_| "Invalid Word archive filename.")?;
        if word_xml(name) {
            if flags & 1 != 0 || !matches!(method,0|8) { return Err("Encrypted or unsupported Word compression. Re-save an unlocked DOCX copy.".into()); }
            expanded=expanded.checked_add(length).filter(|n| *n<=32_000_000).ok_or("Word document expands beyond the import limit.")?;
            if length>MAX_PART { return Err("Word document is too large. Split it into smaller documents.".into()); }
            let local=u32_at(data,position+42)?;
            if data.get(local..local+4)!=Some(b"PK\x03\x04") || u16_at(data,local+8)? != method { return Err("Invalid Word archive local header.".into()); }
            let local_name=u16_at(data,local+26)?;
            if data.get(local+30..local+30+local_name)!=Some(name.as_bytes()) { return Err("Word archive entry names do not match.".into()); }
            let start=local.checked_add(30+local_name+u16_at(data,local+28)?).ok_or("Invalid Word archive offset.")?;
            let end=start.checked_add(compressed).filter(|n| *n<=central_start).ok_or("Truncated Word archive data.")?;
            let packed=data.get(start..end).ok_or("Truncated Word archive data.")?;
            let xml=if method==0 { packed.to_vec() } else {
                let mut output=Vec::new();
                DeflateDecoder::new(packed).take(MAX_PART as u64+1).read_to_end(&mut output).map_err(|_| "Could not decompress Word document.")?;
                output
            };
            if xml.len()!=length || xml.len()>MAX_PART || crc32fast::hash(&xml)!=crc { return Err("Word document data is damaged or exceeds its size limit.".into()); }
            parts.push((name.to_owned(),xml_text(&xml)?));
        }
        position=next;
    }
    if position!=central_end || !parts.iter().any(|(name,_)|name=="word/document.xml") { return Err("The file does not contain a Word document.".into()); }
    parts.sort_by_key(|(name,_)| (name!="word/document.xml",name.clone()));
    let text=parts.into_iter().map(|(_,text)|text).collect::<Vec<_>>().join("\n");
    if text.trim().is_empty() { return Err("No document text was found. Image-only Word files need OCR or a text export.".into()); }
    if text.len()>MAX_TEXT { return Err("Extracted document exceeds the 8 MB text limit. Split it into smaller documents.".into()); }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::DeflateEncoder,Compression};
    use std::io::Write;
    fn docx(xml:&str,compressed:bool)->Vec<u8>{
        let name=b"word/document.xml";let crc=crc32fast::hash(xml.as_bytes());
        let packed=if compressed {let mut e=DeflateEncoder::new(Vec::new(),Compression::default());e.write_all(xml.as_bytes()).unwrap();e.finish().unwrap()}else{xml.as_bytes().to_vec()};
        let method=if compressed {8u16}else{0};let mut out=Vec::new();
        out.extend(b"PK\x03\x04");for n in [20u16,0,method,0,0]{out.extend(n.to_le_bytes());}out.extend(crc.to_le_bytes());out.extend((packed.len() as u32).to_le_bytes());out.extend((xml.len() as u32).to_le_bytes());out.extend((name.len() as u16).to_le_bytes());out.extend(0u16.to_le_bytes());out.extend(name);out.extend(&packed);
        let central=out.len();out.extend(b"PK\x01\x02");for n in [20u16,20,0,method,0,0]{out.extend(n.to_le_bytes());}out.extend(crc.to_le_bytes());out.extend((packed.len() as u32).to_le_bytes());out.extend((xml.len() as u32).to_le_bytes());for n in [name.len() as u16,0,0,0,0]{out.extend(n.to_le_bytes());}out.extend(0u32.to_le_bytes());out.extend(0u32.to_le_bytes());out.extend(name);
        let central_len=out.len()-central;out.extend(b"PK\x05\x06");for n in [0u16,0,1,1]{out.extend(n.to_le_bytes());}out.extend((central_len as u32).to_le_bytes());out.extend((central as u32).to_le_bytes());out.extend(0u16.to_le_bytes());out
    }
    #[test]
    fn stored_and_deflated_word_preserve_unicode_paragraphs_and_tables(){
        let xml=r#"<w:document xmlns:w="word"><w:p><w:r><w:t>Enki &amp; Ilmater — Aerathis</w:t></w:r></w:p><w:tr><w:tc><w:p><w:r><w:t>AC 14</w:t></w:r></w:p></w:tc></w:tr></w:document>"#;
        for compressed in [false,true]{let text=extract_docx_text(&docx(xml,compressed)).unwrap();assert!(text.contains("Enki & Ilmater — Aerathis\n"));assert!(text.contains("AC 14"));}
    }
    #[test]
    fn damaged_unsupported_and_empty_documents_fail_without_extracting_files(){
        assert!(extract_docx_text(b"not a docx").is_err());
        assert!(extract_docx_text(&docx("<w:document xmlns:w=\"word\"/>",false)).is_err());
        let mut corrupt=docx("<w:document><w:t>Test</w:t></w:document>",true);corrupt[35]^=4;assert!(extract_docx_text(&corrupt).is_err());
        assert!(extract_docx_text(&docx("<!DOCTYPE x [<!ENTITY a SYSTEM 'file:///secret'>]><w:t>&a;</w:t>",false)).is_err());
    }
}
