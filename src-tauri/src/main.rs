#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#[macro_use]
extern crate diesel;
mod models;
mod schema;

use anyhow::{anyhow, Result};
use tauri::event;
use webview_official::{Webview, WebviewMut};
mod cmd;
use cmd::Cmd::*;
mod db;

fn main() {
  tauri::AppBuilder::new().setup(setup).build().run();
}

fn setup(webview: &mut Webview, _message: String) {
  let mut webview_mut = webview.as_mut();
  // uncomment to clear db at init
  // smol::run(async {
  //   let connection = db::connect_to_db()?;
  //   db::delete_all_feeds(&connection)?;
  //   Ok(())
  // }) as Result<()>;
  let refresh_result: Result<()> = smol::run(async {
    let connection = db::connect_to_db()?;
    db::refresh_all_feeds(&connection).await?;
    Ok(())
  });
  emit_error_if_necessary(refresh_result, &mut webview_mut);

  tauri::event::listen("", move |msg| {
    let msg = match msg {
      Some(msg) => msg,
      None => {
        eprintln!("No message!");
        emit_error_if_necessary(
          Err(anyhow!("received empty event")),
          &mut webview_mut,
        );
        return;
      }
    };
    let msg_result: Result<()> = {
      smol::run(async {
        let connection = db::connect_to_db()?;
        match serde_json::from_str(&msg)? {
          AddFeed { url } => {
            let channel = db::subscribe_to_feed(&url, &connection).await?;
            event::emit(
              &mut webview_mut,
              String::from("feed-added"),
              Some(channel),
            )?
          }
          GetFeeds {} => {
            let feeds = db::send_all_feeds(&connection)?;
            event::emit(&mut webview_mut, "get-feeds", Some(feeds))?
          }
          GetItemsByFeed { id } => {
            let items = db::send_items_by_feed(id, &connection)?;
            event::emit(&mut webview_mut, "items-by-feed", Some((items, id)))?
          }
        }
        Ok(())
      })
    };

    emit_error_if_necessary(msg_result, &mut webview_mut)
  });
}

fn emit_error_if_necessary(possible_err: Result<()>, webview: &mut WebviewMut) {
  if let Err(e) = possible_err {
    let error_emitted =
      event::emit(webview, String::from("rust-error"), Some(e.to_string()));
    if let Err(e) = error_emitted {
      eprintln!("{}", e)
    };
  }
}
