use anyhow::{Result, anyhow};
use arboard::Clipboard;
use futures::future;
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use inputbot::KeybdKey::*;
use rustautogui::RustAutoGui;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::spawn_local;
use tokio::time::{Duration, sleep};

const MODEL: &str = "gpt-4o-mini";

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
    // Create a LocalSet to run !Send futures.
    let local = tokio::task::LocalSet::new();
    // Create a channel for events triggered by CapsLockKey.
    let (tx, mut rx) = unbounded_channel::<()>();

    // Bind a callback that simply sends an event.
    CapsLockKey.bind({
        let tx = tx.clone();
        move || {
            // Send an event. Ignore errors if the receiver is dropped.
            let _ = tx.send(());
        }
    });

    // Run the blocking input event loop on a separate thread.
    std::thread::spawn(|| {
        // This call will block, but that's fine because it runs in its own thread.
        inputbot::handle_input_events(false);
    });

    // Run our async LocalSet tasks on the main thread.
    local
        .run_until(async move {
            // Spawn a local task that processes incoming events.
            spawn_local(async move {
                while let Some(()) = rx.recv().await {
                    let rustautogui = RustAutoGui::new(false);

                    // Simulate the copy command.
                    rustautogui.keyboard_multi_key("control_l", "control_l", Some("c"));

                    // Wait a bit for the clipboard to update.
                    sleep(Duration::from_millis(200)).await;

                    let mut clipboard = Clipboard::new().unwrap();
                    if let Ok(text) = clipboard.get_text() {
                        match enhance_text(&text).await {
                            Ok(new_text) => {
                                clipboard.set_text(new_text).unwrap();
                                rustautogui.keyboard_multi_key("control_l", "control_l", Some("v"));
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
