require("./dialog.js");
require("./safeStorage.js");
window.__electrico.libs["electron"].nativeImage = {
    createFromPath: (path) => {
        return {
            isEmpty: () => {
                return true;
            }
        }
    }
}