//! Offline document preparation; importing into a campaign is a separate review step.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentProgress {
    pub page: u32,
    pub total: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedDocument {
    pub text: String,
    pub method: String,
    pub pages: u32,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPage {
    pub page: u32,
    pub page_count: u32,
    pub data_url: String,
}
#[tauri::command]
pub async fn render_reference_pdf_page(
    app: tauri::AppHandle,
    campaign_id: String,
    id: String,
    page: u32,
) -> Result<DocumentPage, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let (meta, bytes) = app
            .state::<dwa_engine::CampaignStore>()
            .knowledge_original_bytes(&campaign_id, &id)
            .map_err(|e| e.to_string())?;
        if meta.media_type != "application/pdf" {
            return Err("Choose a PDF document for the page viewer.".into());
        }
        render_pdf_page(&bytes, page)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg(not(windows))]
fn render_pdf_page(_bytes: &[u8], _page: u32) -> Result<DocumentPage, String> {
    Err("Original PDF pages are available in the Windows app. You can read the saved text or download the original.".into())
}
#[cfg(windows)]
fn render_pdf_page(bytes: &[u8], number: u32) -> Result<DocumentPage, String> {
    use windows::{
        Data::Pdf::{PdfDocument, PdfPageRenderOptions},
        Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream},
        Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
    };
    let result = (|| -> windows::core::Result<Result<DocumentPage, String>> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _guard = Apartment;
        let input = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&input)?;
        writer.WriteBytes(bytes)?;
        writer.StoreAsync()?.join()?;
        writer.DetachStream()?;
        input.Seek(0)?;
        let document = PdfDocument::LoadFromStreamAsync(&input)?.join()?;
        let count = document.PageCount()?;
        if number == 0 || number > count {
            return Ok(Err("Choose a page in this PDF.".into()));
        }
        let page = document.GetPage(number - 1)?;
        let size = page.Size()?;
        let scale = 2000.0 / size.Width.max(size.Height).max(1.0);
        let options = PdfPageRenderOptions::new()?;
        options.SetDestinationWidth((size.Width * scale).max(1.0) as u32)?;
        options.SetDestinationHeight((size.Height * scale).max(1.0) as u32)?;
        let output = InMemoryRandomAccessStream::new()?;
        page.RenderWithOptionsToStreamAsync(&output, &options)?
            .join()?;
        page.Close()?;
        let length = output.Size()?;
        if length > 16_000_000 {
            return Ok(Err(
                "This PDF page is too large to preview. Download the original instead.".into(),
            ));
        }
        output.Seek(0)?;
        let reader = DataReader::CreateDataReader(&output)?;
        reader.LoadAsync(length as u32)?.join()?;
        let mut image = vec![0; length as usize];
        reader.ReadBytes(&mut image)?;
        Ok(Ok(DocumentPage {
            page: number,
            page_count: count,
            data_url: format!("data:image/png;base64,{}", STANDARD.encode(image)),
        }))
    })();
    result.map_err(|e| format!("Could not render this PDF page: {e}"))?
}

#[tauri::command]
pub async fn prepare_reference_document(
    name: String,
    data_base64: String,
    on_progress: Channel<DocumentProgress>,
) -> Result<PreparedDocument, String> {
    tauri::async_runtime::spawn_blocking(move||{
        if data_base64.len()>100_000_000 {return Err("Choose a document up to 75 MB, or split it into smaller files.".into());}
        let bytes=STANDARD.decode(data_base64).map_err(|_|"The document could not be read. Choose the file again.")?;
        if bytes.len()>75_000_000 {return Err("Choose a document up to 75 MB.".into());}
        let extension=name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match extension.as_str(){
            "docx"=>Ok(PreparedDocument{text:dwa_engine::documents::extract_docx_text(&bytes)?,method:"Word document text".into(),pages:0,warnings:vec!["Illustrations and page layout are not indexed. Review tables and statistics before saving.".into()]}),
            "pdf"=>pdf_text(&bytes,|page,total|{let _=on_progress.send(DocumentProgress{page,total});}),
            _=>Err("Choose PDF or Word DOCX for document conversion. Plain text, Markdown, CSV, HTML and JSON can be read directly.".into())
        }
    }).await.map_err(|e|format!("Document preparation stopped: {e}"))?
}

#[cfg(not(windows))]
fn pdf_text(_bytes: &[u8], _progress: impl FnMut(u32, u32)) -> Result<PreparedDocument, String> {
    Err("PDF OCR is available in the Windows desktop app. Import a TXT, Markdown or DOCX export on this platform.".into())
}
#[cfg(windows)]
fn pdf_text(bytes: &[u8], mut progress: impl FnMut(u32, u32)) -> Result<PreparedDocument, String> {
    if let Ok(text) = pdf_filter_text(bytes) {
        return Ok(PreparedDocument{text,method:"PDF document text".into(),pages:0,warnings:vec!["Review the extracted text before saving. Image-only pages and illustrations may need OCR; PDF reading order can differ from the printed layout.".into()]});
    }
    pdf_ocr(bytes, &mut progress)
}

#[cfg(windows)]
fn pdf_filter_text(bytes: &[u8]) -> Result<String, String> {
    use std::{
        fs::OpenOptions,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };
    use windows::{
        core::{IUnknown, Interface, HRESULT, PCWSTR, PWSTR},
        Win32::{
            Storage::IndexServer::*,
            System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
        },
    };
    let path = std::env::temp_dir().join(format!(
        "dwa-reference-{}-{}.pdf",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    struct Temporary(std::path::PathBuf);
    impl Drop for Temporary {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _temporary = Temporary(path.clone());
    file.write_all(bytes).map_err(|e| e.to_string())?;
    drop(file);
    let result = (|| -> windows::core::Result<String> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _apartment = Apartment;
        let wide: Vec<u16> = path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut raw = std::ptr::null_mut();
        unsafe {
            LoadIFilter(PCWSTR(wide.as_ptr()), None, &mut raw)?;
        }
        if raw.is_null() {
            return Err(windows::core::Error::from_hresult(HRESULT(
                0x80004005_u32 as i32,
            )));
        }
        let unknown = unsafe { IUnknown::from_raw(raw) };
        let filter: IFilter = unknown.cast()?;
        let mut flags = 0;
        unsafe {
            HRESULT(filter.Init(
                (IFILTER_INIT_CANON_PARAGRAPHS.0
                    | IFILTER_INIT_HARD_LINE_BREAKS.0
                    | IFILTER_INIT_DISABLE_EMBEDDED.0) as u32,
                &[],
                &mut flags,
            ))
            .ok()?;
        }
        let mut text = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let mut complete = false;
        for _ in 0..100_000 {
            if std::time::Instant::now() >= deadline {
                break;
            }
            let mut chunk = STAT_CHUNK::default();
            let status = HRESULT(unsafe { filter.GetChunk(&mut chunk) });
            if status == FILTER_E_END_OF_CHUNKS {
                complete = true;
                break;
            }
            status.ok()?;
            if chunk.flags.0 & CHUNK_TEXT.0 == 0 {
                continue;
            }
            for _ in 0..100_000 {
                if std::time::Instant::now() >= deadline {
                    return Err(windows::core::Error::from_hresult(HRESULT(
                        0x800705B4_u32 as i32,
                    )));
                }
                let mut buffer = [0u16; 4096];
                let mut length = buffer.len() as u32;
                let status =
                    HRESULT(unsafe { filter.GetText(&mut length, PWSTR(buffer.as_mut_ptr())) });
                if status == FILTER_E_NO_MORE_TEXT || status == FILTER_E_NO_TEXT {
                    break;
                }
                status.ok()?;
                text.push_str(
                    &String::from_utf16_lossy(&buffer[..(length as usize).min(buffer.len())])
                        .replace('\0', ""),
                );
                if text.len() > 8_000_000 {
                    return Err(windows::core::Error::from_hresult(HRESULT(
                        0x800700DF_u32 as i32,
                    )));
                }
                if status == FILTER_S_LAST_TEXT || length == 0 {
                    break;
                }
            }
            text.push('\n');
        }
        if !complete {
            return Err(windows::core::Error::from_hresult(HRESULT(
                0x800705B4_u32 as i32,
            )));
        }
        Ok(text)
    })();
    let text = result.map_err(|e| e.to_string())?;
    if text.trim().is_empty() {
        Err("PDF has no searchable text.".into())
    } else {
        Ok(text)
    }
}

#[cfg(windows)]
fn pdf_ocr(bytes: &[u8], mut progress: impl FnMut(u32, u32)) -> Result<PreparedDocument, String> {
    use windows::{
        Data::Pdf::{PdfDocument, PdfPageRenderOptions},
        Graphics::Imaging::BitmapDecoder,
        Media::Ocr::OcrEngine,
        Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
        Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
    };
    let result = (|| -> windows::core::Result<_> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _apartment = Apartment;
        let input = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&input)?;
        writer.WriteBytes(bytes)?;
        writer.StoreAsync()?.join()?;
        writer.DetachStream()?;
        input.Seek(0)?;
        let document = PdfDocument::LoadFromStreamAsync(&input)?.join()?;
        let count = document.PageCount()?;
        if count == 0 || count > 1500 {
            return Ok(Err(
                "Choose a PDF with 1-1500 pages, or split it into smaller documents.".to_owned(),
            ));
        }
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        let maximum = OcrEngine::MaxImageDimension()?.min(2400) as f32;
        let mut text = String::new();
        let mut empty = Vec::new();
        for index in 0..count {
            progress(index + 1, count);
            let page = document.GetPage(index)?;
            let page_result = (|| -> windows::core::Result<String> {
                let size = page.Size()?;
                let scale = maximum / size.Width.max(size.Height).max(1.0);
                let options = PdfPageRenderOptions::new()?;
                options.SetDestinationWidth((size.Width * scale).max(1.0) as u32)?;
                options.SetDestinationHeight((size.Height * scale).max(1.0) as u32)?;
                let stream = InMemoryRandomAccessStream::new()?;
                page.RenderWithOptionsToStreamAsync(&stream, &options)?
                    .join()?;
                stream.Seek(0)?;
                let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
                let bitmap = decoder.GetSoftwareBitmapAsync()?.join()?;
                let recognized = engine.RecognizeAsync(&bitmap)?.join()?.Text()?.to_string();
                bitmap.Close()?;
                Ok(recognized)
            })();
            let _ = page.Close();
            let page_text = page_result?;
            if page_text.trim().is_empty() {
                empty.push(index + 1);
            } else {
                text.push_str(&format!("\n[Page {}]\n{}\n", index + 1, page_text));
            }
            if text.len() > 8_000_000 {
                return Ok(Err(
                    "Extracted PDF exceeds the 8 MB text limit. Split it into smaller PDFs."
                        .to_owned(),
                ));
            }
        }
        if text.trim().is_empty() {
            return Ok(Err(
                "No readable text was found in this PDF. Try a clearer scan or a text export."
                    .to_owned(),
            ));
        }
        let mut warnings=vec!["PDF text was read with offline Windows OCR. Review names, numbers, tables and columns before saving; illustrations are not interpreted.".into()];
        if !empty.is_empty() {
            warnings.push(format!(
                "{} page(s) had no recognized text: {}{}.",
                empty.len(),
                empty
                    .iter()
                    .take(30)
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                if empty.len() > 30 { " …" } else { "" }
            ));
        }
        Ok(Ok(PreparedDocument {
            text,
            method: "Offline PDF OCR".into(),
            pages: count,
            warnings,
        }))
    })();
    result.map_err(|error|format!("Windows could not read this PDF ({error}). It may be locked, damaged, or the installed OCR language may be unavailable. Import a TXT/DOCX export if needed."))?
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn fixture_pdf() -> String {
        let stream = "BT /F1 28 Tf 48 700 Td (Aerathis Sunshard AC 14) Tj ET";
        let objects=["<< /Type /Catalog /Pages 2 0 R >>".to_owned(),"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".into(),"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),format!("<< /Length {} >>\nstream\n{stream}\nendstream",stream.len())];
        let mut pdf = "%PDF-1.4\n".to_owned();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", index + 1, object));
        }
        let start = pdf.len();
        pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
        ));
        pdf
    }
    #[test]
    fn native_pdf_reads_a_real_page_with_offline_ocr() {
        let pdf = fixture_pdf();
        let mut pages = Vec::new();
        let result = pdf_ocr(pdf.as_bytes(), |page, total| pages.push((page, total)))
            .expect("Windows PDF rendering and installed OCR must read the fixture");
        assert_eq!(result.pages, 1);
        assert_eq!(pages, vec![(1, 1)]);
        assert!(
            result.text.to_lowercase().contains("sunshard"),
            "{}",
            result.text
        );
        assert!(result.text.contains("14"));
        assert!(result.method.contains("OCR"));
    }
    #[test]
    fn native_pdf_import_uses_text_or_available_ocr_without_a_cloud_service() {
        let result = pdf_text(fixture_pdf().as_bytes(), |_, _| {}).unwrap();
        assert!(result.text.to_lowercase().contains("sunshard"));
        assert!(result.text.contains("14"));
        assert!(result.method.contains("PDF"));
        assert!(!result.warnings.is_empty());
        println!("Qualified PDF path: {}", result.method);
    }
    #[test]
    fn original_pdf_page_preview_preserves_layout_as_a_local_png() {
        let pdf = fixture_pdf();
        let page = render_pdf_page(pdf.as_bytes(), 1).unwrap();
        assert_eq!(page.page, 1);
        assert_eq!(page.page_count, 1);
        assert!(page
            .data_url
            .starts_with("data:image/png;base64,iVBORw0KGgo"));
        assert!(render_pdf_page(pdf.as_bytes(), 0).is_err());
        assert!(render_pdf_page(pdf.as_bytes(), 2).is_err());
    }
}
