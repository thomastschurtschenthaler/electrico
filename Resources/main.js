const { app, ipcMain, BrowserWindow} = require('electron/main')
const path = require('node:path');
const { spawn } = require('node:child_process');
const fs = require('node:fs');

let mainWindow=null;
function writeOutput(text, level) {
  mainWindow.webContents.send("writeOutput", text, level);
}

ipcMain.on('shellcommand', function(event, command) {
  if (command.cmd=="") {
    writeOutput("command empty\n", "error");
    return;
  }
  const child = spawn(command.cmd, command.args);
  writeOutput(`child process started with pid: ${child.pid}\n`, "info");
  child.stdout.on('data', (data) => {
      writeOutput(`${data}`, "info");
  });

  child.stderr.on('data', (data) => {
      writeOutput(`${data}`, "error");
  });

  child.on('close', (code) => {
      writeOutput(`child process exited with code ${code}\n`, code==0?"info":"error");
  });
  if (command.stdin!="") {
      child.stdin.write(command.stdin);
  }
  event.returnValue=child.pid;
});
let watcher = null; let watchpath = null;
ipcMain.on('startwatch', function(event, path) {
  if (path=="") {
    writeOutput("path empty\n", "error");
    return;
  }
  if (watcher!=null) {
    watcher.close();
  }
  watcher = fs.watch(path, {recursive:true});
  watcher.on("change", (type, file) => {
    writeOutput(`file watch event: ${type}; ${file}\n`, "info");
  });
  watchpath=path;
  writeOutput(`file watch started: ${path}\n`, "info");
});
ipcMain.on('stopwatch', function(event) {
  if (watcher!=null) {
    watcher.close();
    writeOutput(`file watch stopped: ${watchpath}\n`, "info");
    watcher=null;
    watchpath=null;
  }
});

function createWindow () {
  mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js')
    }
  })

  mainWindow.loadFile('app.html');
}
app.setName("Electrico Testapp");

app.whenReady().then(() => {
  createWindow()
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    }
  });
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})