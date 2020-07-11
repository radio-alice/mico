#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use anyhow::{anyhow, Error, Result};
use std::collections::HashSet;
use tauri::event;
use web_view::{Handle, WebView};
mod cmd;
use cmd::Cmd::*;
mod db;
use db::Feed;

fn main() {
  tauri::AppBuilder::new().setup(setup).build().run();
}

fn setup(webview: &mut WebView<()>, _message: String) {
  let handle = webview.handle();
  let db_init = db::setup_db();
  emit_error_if_necessary(db_init, &handle);

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
            let feed_id = db::add_feed(&url, &HashSet::new(), &connection)?;
            event::emit(
              &handle,
              String::from("feed-added"),
              Some(Feed {
                id: feed_id,
                url: url,
                tags: HashSet::new(),
              }),
            )?
          }
          GetFeeds {} => {
            let connection = db::connect_to_db()?;
            let feeds = db::get_feeds(&connection);
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
  match possible_err {
    Err(e) => {
      let error_emitted =
        event::emit(handle, String::from("rust-error"), Some(e.to_string()));
      match error_emitted {
        Err(e) => eprintln!("{}", e),
        Ok(_) => (),
      };
    }
    Ok(_) => (),
  }
}

use rss::Channel;
pub fn format_url(url: &str) -> String {
  if !url.starts_with("http") {
    return format!("https://{}", url);
  }
  url.to_string()
}
pub async fn fetch_channel(url: &str) -> Result<Channel> {
  let feed_xml = surf::get(format_url(url))
    .recv_bytes()
    .await
    .map_err(Error::msg)?;
  let channel = Channel::read_from(&feed_xml[..])?;
  Ok(channel)
}
