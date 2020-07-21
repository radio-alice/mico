use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
  AddFeed { url: String },
  GetFeeds {},
  GetItemsByFeed { id: i32 },
}
