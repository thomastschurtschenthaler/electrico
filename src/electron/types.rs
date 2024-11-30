#[derive(serde::Serialize, serde::Deserialize)]
pub struct BrowserWindowLoadFileParamConfigWebPreferences {
  pub preload: Option<String>,
  #[serde(rename = "nodeIntegration")]
  pub node_integration: bool,
  #[serde(rename = "contextIsolation")]
  pub context_isolation: bool,
  #[serde(rename = "additionalArguments")]
  pub additional_arguments: Option<Vec<String>>
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BrowserWindowLoadFileParamConfig {
  pub title: String,
  pub x: Option<i32>,
  pub y: Option<i32>,
  pub width: Option<i32>,
  pub height: Option<i32>,
  pub resizable: bool,
  pub modal: bool,
  pub show: bool,
  pub icon: Option<String>,
  #[serde(rename = "webPreferences")]
  pub web_preferences: BrowserWindowLoadFileParamConfigWebPreferences
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BrowserWindowCreateParam {
  pub id: String,
  pub config: BrowserWindowLoadFileParamConfig
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BrowserWindowLoadFileParam {
  pub id: String,
  pub file: String,
  pub config: BrowserWindowLoadFileParamConfig
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum BrowserWindowDevToolsCall {
  Open,
  Close
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BrowserWindowDevToolsParam {
  pub id: String,
  pub call: BrowserWindowDevToolsCall
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Menu {
  pub id: String,
  pub label: Option<String>,
  pub accelerator: Option<String>,
  pub role: Option<String>,
  #[serde(rename = "type")]
  pub item_type: Option<String>,
  pub submenu: Option<Vec<Menu>>
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppMenu {
  pub id: String,
  pub label: Option<String>,
  pub submenu: Vec<Menu>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Rectangle {
  pub x: i32,
  pub y: i32,
  pub width: u32,
  pub height: u32,
}
impl Rectangle {
  pub fn new(x: i32, y: i32, width: u32, height: u32) -> Rectangle {
    Rectangle { x, y, width, height }
  }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Display {
  pub bounds: Rectangle
}
impl Display {
  pub fn new(bounds: Rectangle) -> Display {
    Display { bounds }
  }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "method")]
pub enum BrowserWindowBoundsAction {
  Set {bounds: Rectangle},
  Get
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "method")]
pub enum BrowserWindowMaximizedAction {
  Set {maximized: bool},
  Get
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "method")]
pub enum BrowserWindowMinimizedAction {
  Set {minimized: bool},
  Get
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "command")]
pub enum ElectronCommand {
    BrowserWindowCreate {params: BrowserWindowCreateParam},
    BrowserWindowLoadfile {params: BrowserWindowLoadFileParam},
    BrowserWindowSetTitle {id: String, title:String},
    BrowserWindowGetTitle {id: String},
    BrowserWindowShow {id: String, shown:bool},
    BrowserWindowClose {id: String},
    BrowserWindowBounds {id: String, params:BrowserWindowBoundsAction},
    BrowserWindowMaximized {id: String, params:BrowserWindowMaximizedAction},
    BrowserWindowMinimized {id: String, params:BrowserWindowMinimizedAction},
    BrowserWindowDevTools {params: BrowserWindowDevToolsParam},
    ChannelSendMessage {id: String, rid: String, channel: String, args: String},
    ExecuteJavascript {id: String, script:String},
    AppQuit {exit: bool},
    AppSetName {name: String},
    GetAppPath {path: Option<String>},
    GetAppVersion,
    SetApplicationMenu {menu:Option<Vec<AppMenu>>},
    GetPrimaryDisplay,
    ShellOpenExternal {url:String},
    PrintToPDF {id: String},
    RegisterFileProtocol {schema: String},
    Api {data: String}
}