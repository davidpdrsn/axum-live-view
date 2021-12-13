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

            if (topic === "rendered") {
                const element = document.querySelector(`[data-liveview-id="${liveviewId}"]`)
                patchState(this.viewStates[liveviewId], data)
                const html = buildHtmlFromState(this.viewStates[liveviewId])
                this.updateDom(element, html)
            } else if (topic === "initial-render") {
                const element = document.querySelector(`[data-liveview-id="${liveviewId}"]`)
                console.log(data)
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

const buildHtmlFromState = (state) => {
    var combined = ""
    var template = state.s

    for (var i = 0; i < template.length; i++) {
        if (typeof state[i] === "string") {
            combined = combined.concat(template[i], state[i] || "")
        } else if (typeof state[i] === "undefined") {
            combined = combined.concat(template[i])
        } else if (typeof state[i] === "object") {
            combined = combined.concat(template[i], buildHtmlFromState(state[i]))
        } else {
            console.error("buildHtmlFromState", typeof state[i], state[i])
        }
    }

    return combined
}

const patchState = (state, diff) => {
    for (const [key, value] of Object.entries(diff)) {
        if (typeof value === "object") {
            patchState(state[key], value)
        } else if (typeof value === "string") {
            state[key] = value
        } else {
            console.error("patchState", typeof value, value);
        }
    }
}

// (function() {
//     var state = {
//         "0": "0e393823-d873-4c3c-8158-8a91c334366b",
//         "1": {
//             "0": {
//                 "s": [
//                     "its ZERO!"
//                 ]
//             },
//             "s": [
//                 "<div>",
//                 "</div><div><button live-click=\"increment\">+</button><button live-click=\"decrement\">-</button></div>"
//             ]
//         },
//         "s": [
//             "<div class=\"liveview-container\" data-liveview-id=",
//             ">",
//             "</div>"
//         ]
//     };

//     var diff = {
//         "1": {
//             "0": {
//                 "s": [
//                     ""
//                 ]
//             }
//         }
//     };
// })()
