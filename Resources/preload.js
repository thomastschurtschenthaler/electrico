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

window.onWriteOutput = (callback) => {
  ipcRenderer.removeAllListeners("writeOutput");
  ipcRenderer.on("writeOutput", callback);
};