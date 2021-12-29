(() => {
    class LiveView {
        constructor(options) {
            const { host, port } = options
            this.host = host
            this.port = port
            this.viewStates = {}
            this.firstConnect = true
            this.closedForGood = false
        }

        reconnect() {
            if (this.closedForGood) {
                return;
            }

            this.firstConnect = false
            setTimeout(() => {
                this.connect()
            }, 1000)
        }

        connect() {
            this.socket = new WebSocket(`ws://${this.host}:${this.port}/live`)

            this.socket.addEventListener("open", () => {
                this.mountComponents()

                if (this.firstConnect) {
                    this.bindInitialEvents()
                }
            })

            this.socket.addEventListener("close", () => {
                this.reconnect()
            })

            this.socket.addEventListener("message", (event) => {
                const payload = JSON.parse(event.data)

                if (payload.length === 3) {
                    const [liveviewId, topic, data] = payload

                    if (topic === "r") {
                        // rendered
                        const diff = data
                        const element = document.querySelector(`[data-liveview-id="${liveviewId}"]`)

                        patchViewState(this.viewStates[liveviewId], diff)

                        const html = buildHtmlFromState(this.viewStates[liveviewId])
                        this.updateDom(element, html)

                    } else if (topic === "i") {
                        // initial-render
                        const element = document.querySelector(`[data-liveview-id="${liveviewId}"]`)
                        const html = buildHtmlFromState(data)
                        this.updateDom(element, html)
                        this.viewStates[liveviewId] = data

                    } else if (topic === "j") {
                        // js-command
                        this.handleJsCommand(data)

                    } else if (topic === "liveview-gone") {
                        console.error(
                            `Something went wrong on the server and liveview ${liveviewId} is gone`
                        )
                        this.socket.close()
                        this.closedForGood = true

                    } else {
                        console.error("unknown topic", topic, data)
                    }

                } else if (payload.length === 1) {
                    const [topic] = payload
                    if (topic === "h") {
                        // heartbeat
                        this.socket.send(JSON.stringify({ "h": "ok" }))
                    } else {
                        console.error("unknown topic", topic)
                    }

                } else {
                    console.error("unknown socket message", data)
                }
            })
        }

        updateDom(element, html) {
            window.morphdom(element, html, {
                onNodeAdded: (node) => {
                    this.addEventListeners(node)
                },
                onBeforeElUpdated: (fromEl, toEl) => {
                    const tag = toEl.tagName

                    if (tag === 'INPUT') {
                        if (toEl.getAttribute("type") === "radio" || toEl.getAttribute("type") === "checkbox") {
                            toEl.checked = fromEl.checked;
                        } else {
                            toEl.value = fromEl.value;
                        }
                    }

                    if (tag === "TEXTAREA") {
                        toEl.value = fromEl.value;
                    }

                    if (tag === 'OPTION') {
                        if (toEl.closest("select").hasAttribute("multiple")) {
                            toEl.selected = fromEl.selected
                        }
                    }

                    if (tag === "SELECT" && !toEl.hasAttribute("multiple")) {
                        toEl.value = fromEl.value
                    }
                },
            })
        }

        handleJsCommand(commands) {
            for (var i = 0; i < commands.length; i++) {
                const key = Object.keys(commands[i])[0]
                const data = commands[i][key]

                if (key === "ToggleClass") {
                    document.querySelectorAll(data.selector).forEach((element) => {
                        element.classList.toggle(data.class)
                    })

                } else if (key === "AddClass") {
                    document.querySelectorAll(data.selector).forEach((element) => {
                        element.classList.add(data.class)
                    })

                } else if (key === "RemoveClass") {
                    document.querySelectorAll(data.selector).forEach((element) => {
                        element.classList.remove(data.class)
                    })

                } else if (key === "NavigateTo") {
                    if (data.uri.startsWith("http")) {
                        window.location.href = data.uri
                    } else {
                        window.location.pathname = data.uri
                    }

                } else {
                    console.error(`unsupported JS command: ${key}`)
                }
            }
        }

        send(liveviewId, topic, data) {
            let msg = [liveviewId, topic, data]
            this.socket.send(JSON.stringify(msg))
        }

        mountComponents() {
            document.querySelectorAll("[data-liveview-id]").forEach((component) => {
                let liveviewId = component.getAttribute("data-liveview-id")
                this.send(liveviewId, "axum/mount-liveview", {})
            })
        }

        bindInitialEvents() {
            const defs = this.liveBindingDefs()
            var elements = new Set()
            for (var i = 0; i < defs.length; i++) {
                document.querySelectorAll(`[${defs[i].attr}]`).forEach((el) => {
                    if (!elements.has(el)) {
                        this.addEventListeners(el)
                    }
                    elements.add(el)
                })
            }

            document.querySelectorAll("[axm-window-keydown]").forEach((el) => {
                this.bindLiveEvent(
                    el,
                    { attr: "axm-window-keydown", eventName: "keydown", bindEventTo: window }
                )
            })

            document.querySelectorAll("[axm-window-keyup]").forEach((el) => {
                this.bindLiveEvent(
                    el,
                    { attr: "axm-window-keyup", eventName: "keyup", bindEventTo: window }
                )
            })
        }

        liveBindingDefs() {
            return [
                { attr: "axm-click", eventName: "click" },
                { attr: "axm-input", eventName: "input" },
                { attr: "axm-blur", eventName: "blur" },
                { attr: "axm-focus", eventName: "focus" },
                { attr: "axm-change", eventName: "change" },
                { attr: "axm-submit", eventName: "submit" },
                { attr: "axm-keydown", eventName: "keydown" },
                { attr: "axm-keyup", eventName: "keyup" },
            ]
        }

        addEventListeners(element) {
            if (element.querySelectorAll === undefined) {
                return;
            }

            const defs = this.liveBindingDefs()
            for (var i = 0; i < defs.length; i++) {
                this.bindLiveEvent(element, defs[i])
            }
        }

        bindLiveEvent(element, { attr, eventName, bindEventTo }) {
            var bindEventTo = bindEventTo || element

            if (!element.getAttribute?.(attr)) {
                return;
            }

            var f = (event) => {
                let liveviewId = element.closest('[data-liveview-id]').getAttribute("data-liveview-id")
                let msg = element.getAttribute(attr)

                var send = true;

                var data = { "e": eventName };
                try {
                    data.m = JSON.parse(msg);
                } catch {
                    data.m = msg;
                }
                if (element.nodeName === "FORM") {
                    data.v = serializeForm(element)
                } else {
                    data.v = inputValue(element)
                }

                if (event.keyIdentifier) {
                    if (
                        element.hasAttribute("axm-key") &&
                        element.getAttribute("axm-key").toLowerCase() !== event.key.toLowerCase()
                    ) {
                        send = false;
                    }

                    data.k = event.key
                    data.kc = event.code
                    data.a = event.altKey
                    data.c = event.ctrlKey
                    data.s = event.shiftKey
                    data.me = event.metaKey
                }

                if (send) {
                    this.send(liveviewId, `axum/${attr}`, data)
                }
            }

            var delayMs = numberAttr(element, "axm-debounce")
            if (delayMs) {
                f = debounce(f, delayMs)
            }

            var delayMs = numberAttr(element, "axm-throttle")
            if (delayMs) {
                f = throttle(f, delayMs)
            }

            bindEventTo.addEventListener(eventName, (event) => {
                if (!event.keyIdentifier) {
                    event.preventDefault()
                }
                f(event)
            })
        }
    }

    const buildHtmlFromState = (variables) => {
        var combined = ""
        var template = variables[fixed]

        for (var i = 0; i < template.length; i++) {
            const variable = variables[i]

            if (typeof variable === "string") {
                combined = combined.concat(template[i], variable || "")

            } else if (typeof variable === "undefined" || i === template.length - 1) {
                combined = combined.concat(template[i])

            } else if (typeof variable === "object") {
                combined = combined.concat(template[i], buildHtmlFromState(variable))

            } else {
                console.error("buildHtmlFromState", typeof variable, variable)
            }
        }

        return combined
    }

    const patchViewState = (state, diff) => {
        if (typeof state !== 'object') {
            throw "Cannot merge non-object"
        }

        const deepMerge = (state, diff) => {
            for (const [key, val] of Object.entries(diff)) {
                if (val !== null && typeof val === `object` && val.length === undefined) {
                    if (state[key] === undefined) {
                        state[key] = {}
                    }
                    if (typeof state[key] === 'string') {
                        state[key] = val
                    } else {
                        patchViewState(state[key], val)
                    }
                } else {
                    state[key] = val
                }
            }

            return state
        }

        deepMerge(state, diff)

        for (const [key, val] of Object.entries(diff)) {
            if (val === null) {
                delete state[key]
            }
        }

        if (state[fixed].length == Object.keys(state).length - 1) {
            delete state[state[fixed].length - 1]
        }
    }

    const fixed = "f";

    const serializeForm = (element) => {
        var formData = {}

        element.querySelectorAll("textarea").forEach((child) => {
            const name = child.getAttribute("name")
            if (!name) return

            formData[name] = child.value
        })

        element.querySelectorAll("input").forEach((child) => {
            const name = child.getAttribute("name")
            if (!name) return

            if (child.getAttribute("type") === "radio") {
                if (child.checked) {
                    formData[name] = child.value
                }
            } else if (child.getAttribute("type") === "checkbox") {
                if (!formData[name]) {
                    formData[name] = {}
                }
                formData[name][child.value] = child.checked
            } else {
                formData[name] = child.value
            }
        })

        element.querySelectorAll("select").forEach((child) => {
            const name = child.getAttribute("name")
            if (!name) return

            if (child.hasAttribute("multiple")) {
                const values = Array.from(child.selectedOptions).map((opt) => opt.value)
                formData[name] = values
            } else {
                formData[name] = child.value
            }
        })

        return formData
    }

    const inputValue = (element) => {
        if (element.nodeName === "TEXTAREA") {
            return element.value

        } else if (element.nodeName == "INPUT") {
            if (element.getAttribute("type") === "radio" || element.getAttribute("type") === "checkbox") {
                return element.checked
            } else {
                return element.value
            }

        } else if (element.nodeName == "SELECT") {
            if (element.hasAttribute("multiple")) {
                return Array.from(element.selectedOptions).map((opt) => opt.value)
            } else {
                return element.value
            }

        } else {
            return null
        }
    }

    const debounce = (f, delayMs) => {
        var timeout
        return (...args) => {
            if (timeout) {
                clearTimeout(timeout)
            }

            timeout = setTimeout(() => {
                f(...args)
            }, delayMs)
        }
    }

    const throttle = (f, delayMs) => {
        var timeout
        return (...args) => {
            if (timeout) {
                return
            } else {
                f(...args)
                timeout = setTimeout(() => {
                    timeout = null
                }, delayMs)
            }
        }
    }

    const numberAttr = (element, attr) => {
        const value = element.getAttribute(attr)
        if (value) {
            const number = parseInt(value, 10)
            if (!!number) {
                return number
            }
        }
    }

    window.LiveView = LiveView
})()
