class LiveView {
    constructor(host, port) {
        this.socket = new WebSocket(`ws://${host}:${port}/live`)
        this.viewStates = {}
    }

    connect() {
        this.socket.addEventListener("open", () => {
            this.mountComponents()
            this.addEventListeners(document)
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

            } else {
                console.error("unknown event", topic, data)
            }
        })
    }

    updateDom(element, html) {
        window.morphdom(element, html, {
            onNodeAdded: (node) => {
                this.addEventListeners(node)
            },
        })
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

    addEventListeners(element) {
        if (element.querySelectorAll === undefined) {
            return;
        }

        element.querySelectorAll("[live-click]").forEach((element) => {
            element.addEventListener("click", (event) => {
                event.preventDefault()
                let liveviewId = element.closest('[data-liveview-id]').getAttribute("data-liveview-id")
                let eventName = element.getAttribute("live-click")

                var hasAdditionalData = false
                var additionalData = {}
                for (var i = 0; i < element.attributes.length; i++) {
                    var attr = element.attributes[i];
                    if (attr.name.startsWith("live-data-")) {
                        additionalData[attr.name.slice("live-data-".length)] = attr.value
                        hasAdditionalData = true
                    }
                }

                var data = { "e": eventName }
                if (hasAdditionalData) {
                    data["d"] = additionalData;
                }

                this.send(liveviewId, "axum/live-click", data)
            })
        })

        element.querySelectorAll("[live-input]").forEach((element) => {
            element.addEventListener("input", (event) => {
                let liveviewId = element.closest('[data-liveview-id]').getAttribute("data-liveview-id")
                let eventName = element.getAttribute("live-input")

                // TODO: also include `additionalData` here

                this.send(liveviewId, "axum/live-input", { "e": eventName, "v": element.value })
            })
        })
    }
}

const buildHtmlFromState = (variables) => {
    var combined = ""
    var template = variables.s

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
                    console.log(state[key])
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

    if (state["s"].length == Object.keys(state).length - 1) {
        delete state[state["s"].length - 1]
    }
}
