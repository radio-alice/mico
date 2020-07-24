use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
  Subscribe { url: String },
  GetChannels {},
  GetItemsByChannel { id: i32 },
}
