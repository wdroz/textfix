use anyhow::{anyhow, Result};
use arboard::Clipboard;
use futures::future;
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use inputbot::KeybdKey::*;
use rustautogui::RustAutoGui;
use std::thread;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::spawn_local;
use tokio::time::{sleep, Duration};

const MODEL: &str = "gpt-4o-mini";

/// Returns the key sequence for copying (key1, key2, key3) depending on the OS.
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

/// Returns the key sequence for pasting (key1, key2, key3) depending on the OS.
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
    // Wait for the clipboard to update.
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
    let chat_req_str = "You are used to fix text of the user. You need to keep the changes to minimum, if it's perfect, answer back with the user text, as-is";
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
            // Spawn a local task that processes incoming events.
            spawn_local(async move {
                while let Some(()) = rx.recv().await {
                    // Create a new instance each time (if required by rustautogui).
                    let rustautogui = RustAutoGui::new(false);

                    // Simulate copy: send the copy shortcut and wait.
                    perform_copy(&rustautogui).await;
                    
                    // Create the clipboard and read its content.
                    let mut clipboard = Clipboard::new().unwrap();
                    if let Ok(text) = clipboard.get_text() {
                        println!("Clipboard text: {:?}", text);
                        match enhance_text(&text).await {
                            Ok(new_text) => {
                                clipboard.set_text(new_text).unwrap();
                                // Simulate paste: send the paste shortcut.
                                perform_paste(&rustautogui);
                            }
                            Err(e) => eprintln!("Error enhancing text: {:?}", e),
                        }
                    } else {
                        println!("No text in clipboard");
                    }
                }
            });
            // Keep the LocalSet running indefinitely.
            future::pending::<()>().await;
        })
        .await;

    Ok(())
}
