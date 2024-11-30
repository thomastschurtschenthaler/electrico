(function() {
    let dialog = {
        showOpenDialogSync: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            let {r, e} = $e_electron.syncApi_Dialog_ShowOpenDialogSync({options:options});
            return JSON.parse(r);
        },
        showOpenDialog: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            return new Promise(resolve => {
                $e_electron.asyncApi_Dialog_ShowOpenDialog({"window_id": win!=null?win._e_id:null, options:options}).then((e, r)=>{
                    if (e!=null) {
                        let res = JSON.parse(r);
                        resolve({"canceled": res==null, "filePaths":res});
                    } else throw "showOpenDialog failed: "+e;
                });
            });
        },
        showSaveDialogSync: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            let {r, e} = $e_electron.syncApi_Dialog_ShowSaveDialogSync({options:options});
            JSON.parse(r);
        },
        showSaveDialog: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            return new Promise(resolve => {
                $e_electron.asyncApi_Dialog_ShowSaveDialog({"window_id": win!=null?win._e_id:null, options:options}).then((e, r)=>{
                    if (e!=null) {
                        let res = JSON.parse(r);
                        resolve({"canceled": res==null, "filePaths":res});
                    } else throw "showOpenDialog failed: "+e;
                });
            });
        },
        showMessageBoxSync: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            let {r, e} = $e_electron.syncApi_Dialog_ShowMessageBoxSync({options:options});
            JSON.parse(r);
        }
    };
    window.__electrico.libs["electron"].dialog = dialog;
})();