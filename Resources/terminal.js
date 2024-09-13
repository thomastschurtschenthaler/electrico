let html = `<h2>Send Shell Command</h2>
            <div>
                <label for="cmd">Command:</label>
                <input type="text" id="cmd" value="${platform=='win32'?'cmd':'ls'}">
                <label for="args">Arguments:</label>
                <input type="text" id="args" value="${platform=='win32'?'dir':'-ltr'}">
                <label for="stdin">stdin:</label>
                <input type="text" id="stdin">
                <button id="send">Send</button>
            </div>
            <div>
                <label for="output">Output:</label>
                <div class="output" id="output"></div>
            </div>`;
document.getElementById("terminal").innerHTML=html;
document.getElementById("send").onclick = (e) => {
    let cmd = document.getElementById("cmd").value.trim();
    let args = document.getElementById("args").value.trim();
    let stdin = document.getElementById("stdin").value.trim();
    args = args=args!=""?args.split(" "):null;
    callshell({cmd:cmd, args:args, stdin:stdin});
};
window.onWriteOutput((event, text, level) => {
    text = text.replaceAll("\n", "<br>").replaceAll("\r", "<br>");
    let html = level!=null?("<span class='"+level+"'>"+text+"</span>"):text;
    document.getElementById("output").innerHTML+=html;
});