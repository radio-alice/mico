#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#[macro_use]
extern crate diesel;
mod models;
mod schema;

use anyhow::{anyhow, Result};
use diesel::SqliteConnection;
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
  //   db::delete_all_channels(&connection)?;
  //   Ok(())
  // }) as Result<()>;

  // unwrap reasonable bc we need a db connection to do anything
  let connection = db::connect_to_db().unwrap();
  let refresh_result: Result<()> = smol::run(async {
    db::refresh_all_channels(&connection).await?;
    Ok(())
  });
  emit_error_if_necessary(refresh_result, &mut webview_mut);
  let init_result = init(&mut webview_mut, &connection);
  emit_error_if_necessary(init_result, &mut webview_mut);

  tauri::event::listen("", move |msg| {
    let msg = match msg {
      Some(msg) => msg,
      None => {
        emit_error_if_necessary(
          Err(anyhow!("received empty event")),
          &mut webview_mut,
        );
        return;
      }
    };
    let msg_result: Result<()> = {
      smol::run(async {
        match serde_json::from_str(&msg)? {
          Subscribe { url } => {
            let channel = db::subscribe_to_feed(&url, &connection).await?;
            let new_items = db::send_items_by_feed(channel.id, &connection)?;
            event::emit(
              &mut webview_mut,
              String::from("newChannel"),
              Some(channel),
            )?;
            event::emit(
              &mut webview_mut,
              String::from("newItems"),
              Some(new_items),
            )?;
          }
          ExternalLink { url } => {
            open::that(url)?;
          }
        }
        Ok(())
      })
    };

    emit_error_if_necessary(msg_result, &mut webview_mut)
  });
}

fn init(webview: &mut WebviewMut, connection: &SqliteConnection) -> Result<()> {
  let items = db::send_all_items(&connection)?;
  event::emit(webview, "allItems", Some(items))?;
  let feeds = db::send_all_channels(&connection)?;
  event::emit(webview, "allChannels", Some(feeds))?;
  Ok(())
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
