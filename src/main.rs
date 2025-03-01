use anyhow::{Result, anyhow};
use arboard::Clipboard;
use futures::future;
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use inputbot::KeybdKey::*;
use rustautogui::RustAutoGui;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::spawn_local;
use tokio::time::{Duration, sleep};

const MODEL: &str = "gpt-4o-mini";

/// Returns the key sequence for copying depending on the OS.
fn copy_keys() -> (&'static str, &'static str, &'static str) {
    #[cfg(target_os = "macos")]
    {
        ("command_l", "command_l", "c")
    }
    #[cfg(not(target_os = "macos"))]
    {
        ("control_l", "control_l", "c")
    }
}

/// Returns the key sequence for pasting depending on the OS.
fn paste_keys() -> (&'static str, &'static str, &'static str) {
    #[cfg(target_os = "macos")]
    {
        ("command_l", "command_l", "v")
    }
    #[cfg(not(target_os = "macos"))]
    {
        ("control_l", "control_l", "v")
    }
}

/// Simulates a copy command by sending the appropriate key sequence and waiting briefly.
async fn perform_copy(rustautogui: &RustAutoGui) {
    let (k1, k2, k3) = copy_keys();
    rustautogui.keyboard_multi_key(k1, k2, Some(k3));
    // Wait a bit for the clipboard to update.
    sleep(Duration::from_millis(200)).await;
}

/// Simulates a paste command by sending the appropriate key sequence.
fn perform_paste(rustautogui: &RustAutoGui) {
    let (k1, k2, k3) = paste_keys();
    rustautogui.keyboard_multi_key(k1, k2, Some(k3));
}

async fn enhance_text(initial_text: &str) -> Result<String> {
    dotenvy::dotenv().ok();
    let client = Client::default();
    let chat_req_str = "You are a skilled text editor. When given a piece of text, please correct any grammatical, spelling, or punctuation errors while keeping changes to a minimum. Do not simply return the input verbatim; only make the necessary corrections. If the text is already correct, return it unchanged.";
    let mut chat_req = ChatRequest::default().with_system(chat_req_str);
    chat_req = chat_req.append_message(ChatMessage::user(initial_text));
    let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;
    if let Some(text_result) = chat_res.content_text_as_str() {
        Ok(text_result.to_string())
    } else {
        Err(anyhow!("Issue with the LLM answers"))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Create a LocalSet to run non-Send tasks.
    let local = tokio::task::LocalSet::new();
    // Create a channel for events triggered by CapsLockKey.
    let (tx, mut rx) = unbounded_channel::<()>();

    // Bind a callback that sends an event.
    CapsLockKey.bind({
        let tx = tx.clone();
        move || {
            println!("Key pressed!");
            let _ = tx.send(());
        }
    });

    // Run the blocking input event loop on a separate thread.
    thread::spawn(|| {
        inputbot::handle_input_events(false);
    });

    // Run our async LocalSet tasks on the main thread.
    local
        .run_until(async move {
            // Create persistent instances of rustautogui and clipboard.
            let rustautogui = RustAutoGui::new(false);
            let clipboard = Rc::new(RefCell::new(Clipboard::new().unwrap()));
            // Spawn a local task that processes incoming events.
            spawn_local(async move {
                while let Some(()) = rx.recv().await {
                    // Simulate copy: send the copy shortcut and wait.
                    perform_copy(&rustautogui).await;
                    // Use the persistent clipboard instance.
                    let text_result = clipboard.borrow_mut().get_text();
                    match text_result {
                        Ok(text) => {
                            println!("Clipboard text: {:?}", text);
                            match enhance_text(&text).await {
                                Ok(new_text) => {
                                    clipboard.borrow_mut().set_text(new_text).unwrap();
                                    // Simulate paste: send the paste shortcut.
                                    perform_paste(&rustautogui);
                                }
                                Err(e) => eprintln!("Error enhancing text: {:?}", e),
                            }
                        }
                        Err(e) => println!("Error reading clipboard: {:?}", e),
                    }
                }
            });
            // Keep the LocalSet running indefinitely.
            future::pending::<()>().await;
        })
        .await;

    Ok(())
}
