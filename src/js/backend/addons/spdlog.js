(function() {
    let uuidv4 = window.__uuidv4;
    let levels = ["Trace", "Debug", "Info", "Warn", "Error", "Critical", "Off"];
    let SPDLog = {
        setFlushOn: (level) => {
        },
        Logger: class {
            constructor(loggerType, name, filepath, maxFileSize, maxFiles) {
                this.id = uuidv4();
                this.name = name;
                this.filepath = filepath;
                $e_node.syncAddon_SPDLog_CreateLogger({"id":this.id, "name":name, "filepath":filepath});
                this.log = (function(level, message) {
                    $e_node.asyncAddon_SPDLog_Log({"id":this.id, "level":level, "message":message});
                }).bind(this);
                this.flush = (function() {
                    $e_node.asyncAddon_SPDLog_Flush({"id":this.id});
                }).bind(this);
                this.setLevel = (function(level) {
                    $e_node.asyncAddon_SPDLog_SetLogLevel({"id":this.id, "level":levels[level]});
                }).bind(this);
                this.setPattern = (function(pattern) {
                    //TODO
                }).bind(this);
                this.trace = (function(message) {
                    this.log("Trace", message);
                }).bind(this);
                this.debug = (function(message) {
                    this.log("Debug", message);
                }).bind(this);
                this.info = (function(message) {
                    this.log("Info", message);
                }).bind(this);
                this.warn = (function(message) {
                    this.log("Warn", message);
                }).bind(this);
                this.error = (function(message) {
                    this.log("Error", message);
                }).bind(this);
                this.clearFormatters=(function() {
                    
                }).bind(this);
            }
        }
    }
    window.__electrico.libs["spdlog"] = SPDLog;
})();