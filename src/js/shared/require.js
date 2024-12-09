(function() {
    let JS_EXT = [".js", ".cjs", ".json"];
    JS_EXT.isExtension = (path) => {
        return JS_EXT.includes(path.substring(path.lastIndexOf(".")));
    }
    window.__init_require = function (window) {
        window.__dirname="";
        let __module = {
            _load: (request, parent, isMain) => {
                return null;
            },
            _resolveLookupPaths: (request, parent) => {
                return request;
            },
            createRequire: (file) => {
                return require;
            },
            register: (script, path) => {
                //let scr = atob(script.split(",")[1]);
            }
        };
        //console.trace("__init_require call", window);
        function fromCache(expanded_path) {
            return window.__electrico.module_cache[expanded_path];
        }
        function normalize(path) {
            let npath=[];
            for (let p of path.split("/")) {
                if (p==".") continue;
                if (p==".." && npath.length>0) {
                    npath.pop();
                } else {
                    npath.push(p);
                }
            }
            let rpath = npath.join("/");
            //console.log(path, rpath);
            return rpath;
        }
        function resolveNodeModulesPath(path) {
            let ix = path.lastIndexOf("/");
            if (ix<0) return null;
            let spath = path.substring(0, ix);
            let nmpath = spath+"/node_modules";
            let fs = require("fs");
            if (fs.existsSync(nmpath)) {
                return nmpath;
            }
            return resolveNodeModulesPath(spath);
        }
        function circularImport(ctx) {
            let ctxp = ctx;
            while (ctxp.parent!=null) {
                ctxp = ctxp.parent;
                if (ctxp.__filename==ctx.__filename) {
                    ctxp.circular = ctxp.circular || [];
                    ctxp.circular.push(ctx);
                    ctx.lib={};
                    return new Proxy({}, {
                        get(target, prop, receiver) {
                            return ctx.lib[prop];
                        }
                    });
                }
            }
            return null;
        }
        function resolveCircular(ctx, exported) {
            if (ctx.circular!=null) {
                for (let cctx of ctx.circular) {
                    cctx.lib=exported;
                }
            }
        }
        function loadModule(_this, mpath, cache) {
            if (mpath=="module" || mpath=="node:module") {
                return __module;
            }
            if (mpath.endsWith(".node")) {
                mpath=mpath.substring(mpath.lastIndexOf("/")+1, mpath.length);
            }
            let pathparts = mpath.split("/");
            if (pathparts[0].length>0 && !pathparts[0].startsWith(".")) {
                let lib = window.__electrico.getLib(pathparts[0], __electrico_nonce);
                if (lib!=null) {
                    for (let p of pathparts.slice(1, pathparts.length)) {
                        lib=lib[p];
                        if (lib==null) return null;
                    }
                    return lib;
                }
            }
            let fromLoader = __module._load(mpath, {"filename":_this!=null?_this.__filename:null}, false);
            if (fromLoader!=null) {
                return fromLoader;
            }

            var module = {exports:{__default:{}}}; var exports = {__electrico_deferred:[], __default:{}};
            var __e_exports = function(d) {
                if (d=="") return exports;
                return exports.__default;
            };
            let module_path = _this!=null?_this.__import_mpath:"";
            if (mpath.startsWith(".")) {
                expanded_path=module_path.length>0?module_path+"/"+mpath:mpath;
            } else if (mpath.startsWith("/")) {
                expanded_path=mpath;
            } else {
                let node_modules_base = (_this!=null && _this.node_modules_path!=null)?_this.node_modules_path:"node_modules";
                expanded_path=node_modules_base+"/"+mpath;
            }
            expanded_path = normalize(expanded_path);
            let cache_path = expanded_path;
            let cached = fromCache(cache_path);
            if (cached!=null && cached!="" && cache) {
               return cached;
            }
            let expanded_mod_path = expanded_path;
            if (!JS_EXT.isExtension(expanded_path)) expanded_path += ".js";
            
            let script=null; let req={};
            if (cached!="") {
                let jsfilepath = window.__create_protocol_url(window.__create_file_url("electrico-mod/"+expanded_path));
                req = new XMLHttpRequest();
                req.open("GET", jsfilepath, false);
                req.send();
            }
            if (cached=="" || req.status==301) {
                //console.trace("js file not found", expanded_path);
                let package_path = window.__create_protocol_url(window.__create_file_url("electrico-mod/"+expanded_mod_path+"/package.json"));
                let mainjs = null;
                let preq = new XMLHttpRequest();
                preq.open("GET", package_path, false);
                preq.send();
                if (preq.status==301) {
                    expanded_mod_path+=".js";
                    package_path = window.__create_protocol_url(window.__create_file_url("electrico-mod/"+expanded_mod_path+"/package.json"));
                    preq = new XMLHttpRequest();
                    preq.open("GET", package_path, false);
                    preq.send();
                    if (preq.status==301) {
                        console.log("no package.json", package_path);
                    }
                } 
                if (preq.status==200) {
                    let package = JSON.parse(preq.responseText);
                    mainjs = package.main!=null?package.main:(package.exports!=null?(package.exports.default!=null?package.exports.default:package.exports):(package.files!=null?package.files[0]:null));
                }
                mainjs = mainjs || "index.js";
                expanded_path = expanded_mod_path+"/"+mainjs;
                
                if (!JS_EXT.isExtension(expanded_path)) expanded_path+=".js";
                expanded_path = normalize(expanded_path);
                
                const req2 = new XMLHttpRequest();
                let jsfilepath = window.__create_protocol_url(window.__create_file_url("electrico-mod/"+expanded_path));
                req2.open("GET", jsfilepath, false);
                req2.send();
                if (req2.status==301) {
                    if (_this!=null && _this.node_modules_path!=null) {
                        console.log("not found in node_modules_path, trying default node_modules", mpath);
                        delete _this.node_modules_path;
                        return loadModule(_this, mpath, cache);
                    }
                    console.error("js file not found", jsfilepath);
                    return null;
                }
                script=req2.responseText;
            } else {
                script=req.responseText;
            }
            let exported = null;
            if (mpath.endsWith(".json")) {
                exported = JSON.parse(script);
            } else {
                let node_modules_path = null;
                if (_this==null && window.__dirname!="" && expanded_path.startsWith("/")) {
                    node_modules_path = resolveNodeModulesPath(expanded_path);
                }
                
                let __import_mpath = expanded_path.substring(0, expanded_path.lastIndexOf("/"));
                let __dirname = __import_mpath.startsWith(window.__dirname)?__import_mpath:window.__dirname+"/"+__import_mpath;
                let __Import_meta = {url:expanded_path.startsWith(window.__dirname)?expanded_path:window.__dirname+"/"+expanded_path};
                let _this2 = {"parent": _this, "__import_mpath":__import_mpath, "__filename":__Import_meta.url, "node_modules_path": _this!=null?_this.node_modules_path:null};
                if (node_modules_path!=null) _this2.node_modules_path=node_modules_path;

                let circular = circularImport(_this2);
                if (circular!=null) {
                    return circular;
                }

                let sourceURL = "//# sourceURL="+expanded_path+"\n";
                script = window.__replaceImports(script);
                script = sourceURL+"{\nlet __require_this=_this2;"+script+"\n}";
                try {
                    eval(script);
                } catch (e) {
                    window._consolelog("require error", expanded_path, script, e);
                    throw e;
                }
                if (exports.__electrico_deferred!=null) {
                    for (let def of exports.__electrico_deferred) {
                        def();
                    }
                    delete exports.__electrico_deferred;
                }
                exported = (module.exports!=null && ((typeof module.exports=="function") || ((Object.keys(module.exports).length>0 && module.exports.__default==null) || (Object.keys(module.exports).length>1 && module.exports.__default!=null))))?module.exports:exports;
                for (let k in exported.__default) {
                    exported[k] = exported.__default[k];
                }

                resolveCircular(_this2, exported);
            }
            if (cache) {
                window.__electrico.module_cache[cache_path]=exported;
            }
            return exported;
        }
        window.__Import=function(_this, selector, mpath, doExport, exports) {
            //console.log("__import", mpath, selector);
            let mod = loadModule(_this, mpath, true);
            
            let toEval="";
            if (selector!=null) {
                selector = selector.trim();
                let vlnames = false;
                if (selector.startsWith("{") && selector.endsWith("}")) {
                    vlnames = true;
                    selector = selector.substring(1, selector.length-1);
                }
                let parts = selector.split(",");
                for (let i=0; i<parts.length; i++) {
                    let part = parts[i];
                    let vparts = part.split(" as ");
                    let vname = null; let vlname = null;
                    if (vparts.length>1) {
                        vlname =  vparts[0];
                        vname = vparts[1];
                    } else {
                        vname = vparts[0];
                        vlname = vparts[0];
                        if (!vlnames && parts.length==1) {
                            if (mod==null) {
                                console.warn("mod null:", mpath);
                            } else if (mod.__default !=null && Object.keys(mod.__default).length==1) {
                                vlname =  Object.keys(mod.__default)[0];
                                vlnames = true;
                            }
                        }
                    }
                    vlname=vlname.trim();
                    vname=vname.trim();
                    if (vname.length==0) {
                        console.warn("__Import vlname empty", mpath, selector)
                        continue;
                    }
                    if (doExport) {
                        vname = "exports['"+vname+"']";
                    }
                    if (vlnames) {
                        toEval+=((doExport?"":"var ")+vname+"="+"__electrico_import.mod['"+vlname+"'];");
                    } else {
                        toEval+=((doExport?"":"var ")+vname+"="+"__electrico_import.mod;");
                    }
                }
            }
            return {mod:mod, toEval:toEval}; 
        }
        window.__importinline=function(__require_this, mpath) {
            return new Promise((resolve, reject) => {
                let mod = loadModule(__require_this, mpath, true);
                if (mod==null) {
                    reject();
                } else {
                    resolve(mod);
                }
            });
        }
        window.require=function(__require_this, mpath) {
            if (mpath==null) {
                mpath = __require_this;
                __require_this=null;
            }
            return loadModule(__require_this, mpath, true);
        }
        window.__replaceImports = (script) => {
            script = ("\n"+script+"\n").replaceAll(/([;,\r,\n])import (.*) from [',"](.*)[',"][;,\r,\n]/g, "$1var __electrico_import=__Import(__require_this, '$2', '$3');eval(__electrico_import.toEval);");
            script = script.replaceAll(/([;,\r,\n])import [',"](.*)[',"][;,\r,\n]/g, ";$1var __electrico_import=__Import(__require_this, null, '$2');eval(__electrico_import.toEval);");
            script = script.replaceAll("import.meta", "__Import_meta");
            script = script.replaceAll("import(", "__importinline(__require_this, ");
            script = script.replaceAll("require(", "require(__require_this, ");
            script = script.replaceAll(/([;,\r,\n])export (.*) from [',"](.*)[',"][;,\r,\n]/g, ";$1var __electrico_import=__Import(__require_this, '$2', '$3', true, exports);eval(__electrico_import.toEval);");
            
            script = script.replaceAll(/\export +{ *([^{ ,;,\n,}}]*) *} *;/g, "exports['$1']=$1;");
            let export_try_deferred = "var $3={}; try {exports['$3']=$3$4;} catch (e) {}; if (exports.__electrico_deferred!=null) exports.__electrico_deferred.push(function(){if (exports['$3']==null || (Object.keys(exports['$3']).length === 0 && exports['$3'].constructor === Object)) exports['$3']=$3$4;});";

            script = script.replaceAll(/\export +(var ) *(([^{ ,;,\n}]*))(.*);/g, export_try_deferred);
            script = script.replaceAll(/\export +(let ) *(([^{ ,;,\n}]*))(.*);/g, export_try_deferred);
            script = script.replaceAll(/\export +(const ) *(([^{ ,;,\n}]*))(.*);/g, export_try_deferred);

            script = script.replaceAll(/[ ,\r,\n,;]export +(default )?(const )?(var )?(let )? *(([^{ ,;,\n}]*))(.*);/g,"__e_exports('$1')['$6']=$6$7;");

            script = script.replaceAll(/\export +(default)?(const)? *((async +function)?(function)?(function\*)?(async +function\*)?(class)? +([^{ ,(,;,\n}]*))/g, "__e_exports('$1')['$9']=$9=$3");
            script = script.replaceAll('"use strict"', "");
            script = script.replaceAll(/[\r,\n] *}[\r,\n] *\(function/g, "\n};\n(function"); // Color

            let sourcemapspattern = "sourceMappingURL=data:application/json;base64,";
            let smix = script.indexOf(sourcemapspattern);
            if (smix>=0) {
                try {
                    let sourcemaps = JSON.parse(atob(script.substring(smix+sourcemapspattern.length)));
                    if (sourcemaps.sourceRoot!=null && sourcemaps.sourceRoot.startsWith("file://")) {
                        sourcemaps.sourceRoot = window.__create_protocol_url(window.__create_file_url("electrico-mod/"+sourcemaps.sourceRoot.substring(7)));
                        //script = script.substring(0, smix+sourcemapspattern.length)+btoa(JSON.stringify(sourcemaps));
                        script = script.substring(0, smix+sourcemapspattern.length);
                    }
                } catch (e) {}
            }
            return script;
        }
    };
})();