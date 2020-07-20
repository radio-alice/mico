#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#[macro_use]
extern crate diesel;
pub mod models;
pub mod schema;

use anyhow::{anyhow, Result};
use tauri::event;
use web_view::{Handle, WebView};
mod cmd;
use cmd::Cmd::*;
mod db;

fn main() {
  tauri::AppBuilder::new().setup(setup).build().run();
}

fn setup(webview: &mut WebView<()>, _message: String) {
  let handle = webview.handle();

  tauri::event::listen("", move |msg| {
    let msg = match msg {
      Some(msg) => msg,
      None => {
        eprintln!("No message!");
        emit_error_if_necessary(Err(anyhow!("received empty event")), &handle);
        return;
      }
    };
    let msg_result: Result<()> = {
      smol::run(async {
        match serde_json::from_str(&msg)? {
          AddFeed { url } => {
            let connection = db::connect_to_db()?;
            let channel = db::subscribe_to_feed(&url, &connection).await?;
            event::emit(&handle, String::from("feed-added"), Some(channel))?
          }
          GetFeeds {} => {
            let connection = db::connect_to_db()?;
            let feeds = db::send_all_feeds(&connection);
            event::emit(&handle, "get-feeds", Some(feeds?))?
          }
        }
        Ok(())
      })
    };

    emit_error_if_necessary(msg_result, &handle)
  });
}

fn emit_error_if_necessary(possible_err: Result<()>, handle: &Handle<()>) {
  if let Err(e) = possible_err {
    let error_emitted =
      event::emit(handle, String::from("rust-error"), Some(e.to_string()));
    if let Err(e) = error_emitted {
      eprintln!("{}", e)
    };
  }
}
