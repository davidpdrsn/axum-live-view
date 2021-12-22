(() => {
    class LiveView {
        constructor(options) {
            const { host, port } = options
            this.socket = new WebSocket(`ws://${host}:${port}/live`)
            this.viewStates = {}
        }

        connect() {
            this.socket.addEventListener("open", () => {
                this.mountComponents()
            })

            this.socket.addEventListener("message", (event) => {
                const [liveviewId, topic, data] = JSON.parse(event.data)

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

                } else {
                    console.error("unknown topic", topic, data)
                }
            })
        }

        updateDom(element, html) {
            window.morphdom(element, html, {
                onNodeAdded: (node) => {
                    this.addEventListeners(node)
                },
                onBeforeElUpdated: function(fromEl, toEl) {
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

        handleJsCommand({ type, data }) {
            if (type == "navigate_to") {
                const uri = data.uri
                if (uri.startsWith("http")) {
                    window.location.href = uri
                } else {
                    window.location.pathname = uri
                }

            } else {
                console.error("unknown type", data)
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
        }

        liveBindingDefs() {
            return [
                { attr: "axm-click", bindTo: "click" },
                { attr: "axm-input", bindTo: "input" },
                { attr: "axm-blur", bindTo: "blur" },
                { attr: "axm-focus", bindTo: "focus" },
                { attr: "axm-change", bindTo: "change" },
                { attr: "axm-submit", bindTo: "submit" },
                { attr: "axm-keydown", bindTo: "keydown" },
                { attr: "axm-keyup", bindTo: "keyup" },
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

        bindLiveEvent(element, { attr, bindTo }) {
            if (!element.getAttribute?.(attr)) {
                return;
            }

            var f = (event) => {
                let liveviewId = element.closest('[data-liveview-id]').getAttribute("data-liveview-id")
                let eventName = element.getAttribute(attr)

                var send = true;

                var data;
                if (element.nodeName === "FORM") {
                    data = { "e": eventName, "v": serializeForm(element) }
                } else {
                    data = { "e": eventName, "v": inputValue(element) }
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
                    data.m = event.metaKey
                }

                if (send) {
                    addAdditionalData(element, data)
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

            element.addEventListener(bindTo, (event) => {
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

    const addAdditionalData = (element, data) => {
        var hasAdditionalData = false
        var additionalData = {}
        for (var i = 0; i < element.attributes.length; i++) {
            var attr = element.attributes[i];
            if (attr.name.startsWith("axm-data-")) {
                additionalData[attr.name.slice("axm-data-".length)] = attr.value
                hasAdditionalData = true
            }
        }

        if (hasAdditionalData) {
            data["d"] = additionalData;
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
            console.error("what input element is this?", element)
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
