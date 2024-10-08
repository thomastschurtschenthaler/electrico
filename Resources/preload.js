window.addEventListener('DOMContentLoaded', () => {
    const replaceText = (selector, text) => {
      const element = document.getElementById(selector)
      if (element) element.innerText = text
    }
  
    for (const type of ['chrome', 'node', 'electron']) {
      replaceText(`${type}-version`, process.versions[type])
    }
});
const { ipcRenderer, contextBridge } = require('electron');
contextBridge.exposeInMainWorld("callshell", (command)=>{
  ipcRenderer.send("shellcommand", command);
});
contextBridge.exposeInMainWorld("onWriteOutput", (callback) => {
  ipcRenderer.removeAllListeners("writeOutput");
  ipcRenderer.on("writeOutput", callback);
});
contextBridge.exposeInMainWorld("platform", process.platform);
contextBridge.exposeInMainWorld("startwatch", (path)=>{
  ipcRenderer.send("startwatch", path);
});
contextBridge.exposeInMainWorld("stopwatch", ()=>{
  ipcRenderer.send("stopwatch");
});