(function() {
    let dialog = {
        showOpenDialogSync: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            let {r, e} = $e_electron.syncApi_Dialog_ShowOpenDialogSync({options:options});
            if (e!=null) throw "showOpenDialogSync error: "+e;
            return JSON.parse(r);
        },
        showOpenDialog: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            return new Promise((resolve, reject) => {
                $e_electron.asyncApi_Dialog_ShowOpenDialog({"window_id": win!=null?win._e_id:null, options:options}).then((e, r)=>{
                    if (e!=null) {
                        reject(e);
                        return;
                    } 
                    let res = JSON.parse(r);
                    resolve({"canceled": res.length==0, "filePaths":res});
                });
            });
        },
        showSaveDialogSync: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            let {r, e} = $e_electron.syncApi_Dialog_ShowSaveDialogSync({options:options});
            if (e!=null) throw "showSaveDialogSync error: "+e;
            let res = JSON.parse(r);
            return {"canceled": res.length==0, "filePaths":res};
        },
        showSaveDialog: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            return new Promise((resolve, reject)=> {
                $e_electron.asyncApi_Dialog_ShowSaveDialog({"window_id": win!=null?win._e_id:null, options:options}).then((e, r)=>{
                    if (e!=null) {
                        reject(e);
                        return;
                    }
                    let res = JSON.parse(r);
                    resolve({"canceled": res.length==0, "filePaths":res});
                });
            });
        },
        showMessageBoxSync: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            let {r, e} = $e_electron.syncApi_Dialog_ShowMessageBoxSync({options:options});
            if (e!=null) throw "showMessageBoxSync error: "+e;
            let clicked = null;
            if (r.length>0) {
                clicked = parseInt(r);
            }
            return clicked;
        },
        showMessageBox: (win, options) => {
            if (options==null) {
                options=win;
                win=null;
            }
            return new Promise((resolve, reject)=> {
                $e_electron.asyncApi_Dialog_ShowMessageBox({"window_id": win!=null?win._e_id:null, options:options}).then((e, r)=>{
                    if (e!=null) {
                        reject(e);
                        return;
                    }
                    let clicked = null;
                    if (r.length>0) {
                        clicked = parseInt(r);
                    }
                    resolve({"response":clicked});
                });
            });
        }
    };
    window.__electrico.libs["electron"].dialog = dialog;
})();