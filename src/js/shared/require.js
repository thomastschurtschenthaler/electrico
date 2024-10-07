(function() {
    window.__init_require = function (window) {
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
            if (rpath.startsWith("/")) rpath = rpath.substring(1);
            //console.log(path, rpath);
            return rpath;
        }
        function loadModule(_this, mpath, cache) {
            let lib = window.__electrico.getLib(mpath, __electrico_nonce);
            if (lib!=null) {
                return lib;
            }
            var module = {}; var exports = {__electrico_deferred:[]};
            let module_path = _this!=null?_this.__import_mpath:"";
            let expanded_path = module_path;
            if (mpath.startsWith(".")) {
                expanded_path+="/"+mpath;
            } else {
                expanded_path="node_modules/"+mpath;
            }
            expanded_path = normalize(expanded_path);
            let cache_path = expanded_path;
            let cached = fromCache(cache_path);
            if (cached!=null && cached!="" && cache) {
               return cached;
            }
            let script=null; let req={};
            if (cached!="") {
                let jsfilepath = window.__create_protocol_url("fil://mod/"+((expanded_path.lastIndexOf(".")<expanded_path.lastIndexOf("/"))?expanded_path+".js":expanded_path));
                req = new XMLHttpRequest();
                req.open("GET", jsfilepath, false);
                req.send();
            }
            if (cached=="" || req.status==301) {
                //console.trace("js file not found", expanded_path);
                let package_path = window.__create_protocol_url("fil://mod/"+expanded_path+"/package.json");
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
                
                if (!expanded_path.endsWith("js")) expanded_path+=".js";
                expanded_path = normalize(expanded_path);
                
                const req2 = new XMLHttpRequest();
                let jsfilepath = window.__create_protocol_url("fil://mod/"+expanded_path);
                req2.open("GET", jsfilepath, false);
                req2.send();
                if (req2.status==404) {
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
                let _this = {"__import_mpath":expanded_path.substring(0, expanded_path.lastIndexOf("/"))};
                let sourceURL = "//# sourceURL="+expanded_path+"\n";
                script = window.__replaceImports(script);
                script = sourceURL+"{\nlet __require_this=_this;"+script+"\n}";
                try {
                    eval(script);
                } catch (e) {
                    //console.log("require error", expanded_path, script, e);
                    throw e;
                }
                if (exports.__electrico_deferred!=null) {
                    for (let def of exports.__electrico_deferred) {
                        def();
                    }
                    delete exports.__electrico_deferred;
                }
                exported = module.exports || exports;
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
                            } else if (Object.keys(mod).length==1) {
                                vlname =  Object.keys(mod)[0];
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
            let export_try_deferred = "var $3={}; try {exports['$3']=$3$4;} catch (e) {exports.__electrico_deferred.push(function(){exports['$3']=$3$4;});};";

            script = script.replaceAll(/\export +(var ) *(([^{ ,;,\n}]*))(.*);/g, export_try_deferred);
            script = script.replaceAll(/\export +(let ) *(([^{ ,;,\n}]*))(.*);/g, export_try_deferred);
            script = script.replaceAll(/\export +(const ) *(([^{ ,;,\n}]*))(.*);/g, export_try_deferred);
            script = script.replaceAll(/\export +(default )?(const )?(var )?(let )? *(([^{ ,;,\n}]*))(.*);/g, "exports['$6']=$6$7;");

            script = script.replaceAll(/\export +(default)?(const)? *((async +function)?(function)?(function\*)?(async +function\*)?(class)? +([^{ ,(,;,\n}]*))/g, "exports['$9']=$9=$3");
            script = script.replaceAll('"use strict"', "");
            script = script.replaceAll(/[\r,\n] *}[\r,\n] *\(function/g, "\n};\n(function"); // Color

            let sourcemapspattern = "sourceMappingURL=data:application/json;base64,";
            let smix = script.indexOf(sourcemapspattern);
            if (smix>=0) {
                try {
                    let sourcemaps = JSON.parse(atob(script.substring(smix+sourcemapspattern.length)));
                    if (sourcemaps.sourceRoot!=null && sourcemaps.sourceRoot.startsWith("file://")) {
                        sourcemaps.sourceRoot = window.__create_protocol_url("fil://mod/"+sourcemaps.sourceRoot.substring(7));
                        script = script.substring(0, smix+sourcemapspattern.length)+btoa(JSON.stringify(sourcemaps));
                    }
                } catch (e) {}
            }
            return script;
        }
    };
})();