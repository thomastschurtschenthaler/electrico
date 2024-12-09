#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "api")]
pub enum APICommand {
    Dialog {command: DialogCommand}
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum DialogCommand {
    ShowOpenDialogSync {options:FileDialogOptions},
    ShowOpenDialog {window_id:Option<String>, options:FileDialogOptions},
    ShowSaveDialogSync {options:FileDialogOptions},
    ShowSaveDialog {window_id:Option<String>, options:FileDialogOptions},
    ShowMessageBoxSync {options:ShowMessageBoxOptions},
    ShowMessageBox {window_id:Option<String>, options:ShowMessageBoxOptions},
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileDialogOptions {
  pub title: Option<String>,
  #[serde(rename = "defaultPath")]
  pub default_path: Option<String>,
  #[serde(rename = "buttonLabel")]
  pub button_label: Option<String>,
  pub filters:Option<Vec<FileFilter>>,
  pub properties: Option<Vec<String>>,
  pub message: Option<String>,
  
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ShowMessageBoxOptions {
  pub message: String,
  #[serde(rename = "type")]
  pub msg_type: Option<String>,
  #[serde(rename = "buttonLabel")]
  pub title: Option<String>,
  pub detail: Option<String>,
  pub buttons: Option<Vec<String>>,
  #[serde(rename = "cancelId")]
  pub cancel_id:Option<i32>
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileFilter {
  pub name: String,
  pub extensions: Vec<String>,
}