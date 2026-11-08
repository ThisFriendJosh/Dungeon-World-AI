//! Clipboard access happens only in response to an explicit edit gesture.
#[cfg(windows)]
mod native {
    use windows_sys::Win32::{Foundation::{HANDLE,HWND},System::{DataExchange::*,Memory::*}};
    const TEXT: u32 = 13; // CF_UNICODETEXT
    const LIMIT: usize = 1_000_000;
    struct Clipboard;
    impl Clipboard {
        fn open(owner:HWND)->Result<Self,String>{
            for _ in 0..4 {
                if unsafe{OpenClipboard(owner)}!=0 {return Ok(Self);}
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(format!("The clipboard could not be opened: {}. Try copying the text again.",std::io::Error::last_os_error()))
        }
    }
    impl Drop for Clipboard {fn drop(&mut self){unsafe{CloseClipboard();}}}
    pub fn read(owner:HWND)->Result<String,String>{
        let _clipboard=Clipboard::open(owner)?;
        if unsafe{IsClipboardFormatAvailable(TEXT)}==0 {return Err("The clipboard contains no plain text. Copy text, then paste again.".into());}
        let data=unsafe{GetClipboardData(TEXT)};
        if data.is_null(){return Err("The clipboard text is unavailable.".into());}
        let size=unsafe{GlobalSize(data)};
        if size==0 || size>LIMIT*2+2 || size%2!=0{return Err("Clipboard text exceeds the import limit. Add a document file instead.".into());}
        let ptr=unsafe{GlobalLock(data)} as *const u16;
        if ptr.is_null(){return Err("The clipboard text could not be read.".into());}
        let result={let values=unsafe{std::slice::from_raw_parts(ptr,size/2)};let end=values.iter().position(|v|*v==0).unwrap_or(values.len());String::from_utf16_lossy(&values[..end])};
        unsafe{GlobalUnlock(data);}
        Ok(result)
    }
    pub fn write(owner:HWND,text:&str)->Result<(),String>{
        if text.len()>LIMIT {return Err("Select less than 1 MB of text to copy.".into());}
        let wide:Vec<u16>=text.encode_utf16().chain(Some(0)).collect();
        let memory=unsafe{GlobalAlloc(GMEM_MOVEABLE|GMEM_ZEROINIT,wide.len()*2)};
        if memory.is_null(){return Err("Could not allocate clipboard text.".into());}
        let result=(||{
            let ptr=unsafe{GlobalLock(memory)} as *mut u16;
            if ptr.is_null(){return Err("Could not prepare clipboard text.".into());}
            unsafe{std::ptr::copy_nonoverlapping(wide.as_ptr(),ptr,wide.len());GlobalUnlock(memory);}
            let _clipboard=Clipboard::open(owner)?;
            if unsafe{EmptyClipboard()}==0 || unsafe{SetClipboardData(TEXT,memory as HANDLE)}.is_null(){return Err(format!("Could not copy the selected text: {}",std::io::Error::last_os_error()));}
            Ok(())
        })();
        if result.is_err(){unsafe{windows_sys::Win32::Foundation::GlobalFree(memory);}}
        result
    }
}

#[tauri::command]
pub fn read_clipboard_text(window:tauri::Window)->Result<String,String>{
    #[cfg(windows)]{return native::read(window.hwnd().map_err(|e|e.to_string())?.0 as _);}
    #[cfg(not(windows))]{let _=window;Err("Use the system Paste command on this platform.".into())}
}
#[tauri::command]
pub fn write_clipboard_text(window:tauri::Window,text:String)->Result<(),String>{
    #[cfg(windows)]{return native::write(window.hwnd().map_err(|e|e.to_string())?.0 as _,&text);}
    #[cfg(not(windows))]{let _=(window,text);Err("Use the system Copy command on this platform.".into())}
}
