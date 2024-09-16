(function() {
    window.__init_require = function (window) {
        //console.trace("__init_require call", window);
        function fromCache(expanded_path) {
            return window.__electrico.module_cache[expanded_path];
        }
        function loadModule(mpath, cache) {
            let lib = window.__electrico.getLib(mpath, __electrico_nonce);
            if (lib!=null) {
                return lib;
            }
            var module = {}; var exports = {};
            let stack_path="";
            try {__electrico_dummy_error} catch (e) {
                let stack = e.stack.replaceAll("\n", "@").replaceAll("\r", "@").split("@");
                for (let i =0 ; i<stack.length; i++) {
                    if (i>0) {
                        stack_path+=stack[i].split(":")[0];
                    }
                }
            }
            let found_sp=null;
            for (sp in window.__electrico.module_paths) {
                if (stack_path.endsWith(sp) && stack_path!=sp) {
                    if (found_sp==null || sp.length>found_sp.length) {
                        found_sp=sp;
                    }
                } else if (sp.endsWith(stack_path)) {
                    delete window.__electrico.module_paths[sp];
                }
            }

            let module_path = found_sp!=null?window.__electrico.module_paths[found_sp]:window.__create_protocol_url("fil://mod");
            let expanded_path = module_path;
            if (mpath.startsWith(".")) {
                expanded_path+=mpath.substring(1, mpath.length);
            } else {
                expanded_path=window.__create_protocol_url("fil://mod/node_modules/"+mpath);
            }
            
            let cached = fromCache(expanded_path);
            if (cached!=null && cached!="" && cache) {
                window.__electrico.module_paths[stack_path]=expanded_path.substring(0, expanded_path.lastIndexOf("/"));
                return cached;
            }
            let script=null; let req={};
            if (cached!="") {
                let jsfilepath = (expanded_path.lastIndexOf(".")<expanded_path.lastIndexOf("/"))?expanded_path+".js":expanded_path;
                req = new XMLHttpRequest();
                req.open("GET", jsfilepath, false);
                req.send();
            }
            if (cached=="" || req.status==301) {
                //console.trace("js file not found", expanded_path);
                window.__electrico.module_cache[expanded_path]="";
                let package_path = expanded_path+"/package.json";
                const preq = new XMLHttpRequest();
                preq.open("GET", package_path, false);
                preq.send();
                if (preq.status==301) {
                    console.error("js file not found - no package.json", package_path);
                    return null;
                }
                let package = JSON.parse(preq.responseText);
                let mainjs = package.main!=null?package.main:(package.exports!=null?(package.exports.default!=null?package.exports.default:package.exports):package.files[0]);
                expanded_path = expanded_path+"/"+mainjs;
                window.__electrico.module_paths[stack_path]=expanded_path.substring(0, expanded_path.lastIndexOf("/"));

                if (!expanded_path.endsWith("js")) expanded_path+=".js";
                if (cache) {
                    let cached = fromCache(expanded_path);
                    if (cached!=null) {
                        window.__electrico.module_paths[stack_path]=expanded_path.substring(0, expanded_path.lastIndexOf("/"));
                        return cached;
                    }
                }
                const req2 = new XMLHttpRequest();
                req2.open("GET", expanded_path, false);
                req2.send();
                if (req2.status==404) {
                    console.error("js file not found", expanded_path);
                    return null;
                }
                script=req2.responseText;
            } else {
                script=req.responseText;
            }
            window.__electrico.module_paths[stack_path]=expanded_path.substring(0, expanded_path.lastIndexOf("/"));
            script = "//# sourceURL="+expanded_path.substring(11, expanded_path.length) +"\n"+script;
            eval(window.__replaceImports(script));
            let exported = module.exports || exports;
            if (cache) {
                window.__electrico.module_cache[expanded_path]=exported;
            }
            return exported;
        }
        window.__import=function(mpath, selector) {
            //console.log("__import", mpath, selector);
            let mod = loadModule(mpath, false);
            if (selector!=null) {
                //console.log("selector mod", mod);
                /*let modsel = {};
                if (selector=="*") {
                    for (let k in mod)
                }*/
            }
            return mod; 
        }
        window.require=function(mpath) {
            return loadModule(mpath, true);
        }
        window.__replaceImports = (script) => {
            let impr = script.replaceAll(/\import *((.*) +as +)?(.*) *from *([^{ ,;,\r, \n}]*)/g, "var $3 = __import($4, '$2')").replaceAll("import.meta", "__import_meta");
            impr = impr.replaceAll(/\export +(default)?(const)?([^ ]* +([^{ ,(,;,\n}]*))/g, "exports['$4']=$3");
            return impr;
        }
    };
})();