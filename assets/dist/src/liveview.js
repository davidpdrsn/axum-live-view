"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.connectAndRun = void 0;
function connectAndRun(options) {
    const socket = new WebSocket(`ws://${options.host}:${options.port}/live`);
    var firstConnect = true;
    socket.addEventListener("open", () => {
        onOpen(socket);
        if (firstConnect) {
            bindInitialEvents();
        }
    });
}
exports.connectAndRun = connectAndRun;
function onOpen(socket) {
    mountComponents(socket);
}
function socketSend(socket, liveviewId, topic, data) {
    let msg = [liveviewId, topic, data];
    socket.send(JSON.stringify(msg));
}
function mountComponents(socket) {
    const liveviewIdAttr = "data-liveview-id";
    document.querySelectorAll(`[${liveviewIdAttr}]`).forEach((component) => {
        const liveviewId = component.getAttribute(liveviewIdAttr);
        if (liveviewId) {
            socketSend(socket, liveviewId, "axum/mount-liveview", {});
        }
    });
}
function bindInitialEvents() {
    var elements = new Set();
    for (let def of elementLocalAttrs) {
        document.querySelectorAll(`[${def.attr}]`).forEach((el) => {
            if (!elements.has(el)) {
                addEventListeners(el);
            }
            elements.add(el);
        });
    }
}
function addEventListeners(element) {
    const defs = elementLocalAttrs;
    for (let def of elementLocalAttrs) {
        bindLiveEvent(element, def);
    }
}
function bindLiveEvent(element, { attr, eventName, bindEventTo }) {
    var _a;
    var bindEventTo2 = bindEventTo || element;
    if (!((_a = element.getAttribute) === null || _a === void 0 ? void 0 : _a.call(element, attr))) {
        return;
    }
    var f = (event) => {
        var _a;
        let liveviewId = (_a = element.closest("[data-liveview-id]")) === null || _a === void 0 ? void 0 : _a.getAttribute("data-liveview-id");
        if (!liveviewId)
            return;
        let msg = element.getAttribute(attr);
        if (!msg)
            return;
        var data = { e: eventName };
        try {
            data.m = JSON.parse(msg);
        }
        catch (_b) {
            data.m = msg;
        }
        if (element.nodeName === "FORM") {
            data.v = serializeForm(element);
        }
        else {
            const value = inputValue(element);
            if (value) {
                data.v = value;
            }
        }
    };
}
const elementLocalAttrs = [
    { attr: "axm-click", eventName: "click" },
    { attr: "axm-input", eventName: "input" },
    { attr: "axm-blur", eventName: "blur" },
    { attr: "axm-focus", eventName: "focus" },
    { attr: "axm-change", eventName: "change" },
    { attr: "axm-submit", eventName: "submit" },
    { attr: "axm-keydown", eventName: "keydown" },
    { attr: "axm-keyup", eventName: "keyup" },
    { attr: "axm-mouseenter", eventName: "mouseenter" },
    { attr: "axm-mouseover", eventName: "mouseover" },
    { attr: "axm-mouseleave", eventName: "mouseleave" },
    { attr: "axm-mouseout", eventName: "mouseout" },
    { attr: "axm-mousemove", eventName: "mousemove" },
];
const windowAttrs = [
    { attr: "axm-window-keydown", eventName: "keydown", bindEventTo: window },
    { attr: "axm-window-keyup", eventName: "keyup", bindEventTo: window },
    { attr: "axm-window-focus", eventName: "focus", bindEventTo: window },
    { attr: "axm-window-blur", eventName: "blur", bindEventTo: window },
];
function serializeForm(element) {
    var formData = {};
    element.querySelectorAll("textarea").forEach((child) => {
        const name = child.getAttribute("name");
        if (!name) {
            return;
        }
        formData[name] = child.value;
    });
    element.querySelectorAll("input").forEach((child) => {
        const name = child.getAttribute("name");
        if (!name) {
            return;
        }
        if (child.getAttribute("type") === "radio") {
            if (child.checked) {
                formData[name] = child.value;
            }
        }
        else if (child.getAttribute("type") === "checkbox") {
            if (!formData[name]) {
                formData[name] = {};
            }
            formData[name][child.value] = child.checked;
        }
        else {
            formData[name] = child.value;
        }
    });
    element.querySelectorAll("select").forEach((child) => {
        const name = child.getAttribute("name");
        if (!name)
            return;
        if (child.hasAttribute("multiple")) {
            const values = Array.from(child.selectedOptions).map((opt) => opt.value);
            formData[name] = values;
        }
        else {
            formData[name] = child.value;
        }
    });
    return formData;
}
function inputValue(element) {
    if (element instanceof HTMLTextAreaElement) {
        return element.value;
    }
    else if (element instanceof HTMLInputElement) {
        if (element.getAttribute("type") === "radio" || element.getAttribute("type") === "checkbox") {
            return element.checked;
        }
        else {
            return element.value;
        }
    }
    else if (element instanceof HTMLSelectElement) {
        if (element.hasAttribute("multiple")) {
            return Array.from(element.selectedOptions).map((opt) => opt.value);
        }
        else {
            return element.value;
        }
    }
    else {
        return null;
    }
}
function debounce(f, delayMs) {
    var timeout;
    return (...args) => {
        if (timeout) {
            clearTimeout(timeout);
        }
        timeout = setTimeout(() => {
            f(...args);
        }, delayMs);
    };
}
function throttle(f, delayMs) {
    var timeout;
    return (...args) => {
        if (timeout) {
            return;
        }
        else {
            f(...args);
            timeout = setTimeout(() => {
                timeout = null;
            }, delayMs);
        }
    };
}
//# sourceMappingURL=liveview.js.map