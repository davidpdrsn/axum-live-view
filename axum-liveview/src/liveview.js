class LiveView {
    constructor(host, port) {
        this.socket = new WebSocket(`ws://${host}:${port}/live`)
    }

    connect() {
        this.socket.addEventListener("open", () => {
            this.mountComponents()
            this.addEventListeners(document)
        })

        this.socket.addEventListener("message", (event) => {
            const [liveviewId, topic, data] = JSON.parse(event.data)

            if (topic === "rendered") {
                // const element = document.querySelector(`[data-liveview-id="${data.liveview_id}"]`)
                // window.morphdom(element, data.html, {
                //     onNodeAdded: (node) => {
                //         this.addEventListeners(node)
                //     },
                //     // this break setting input field values from live views
                //     // onBeforeElUpdated: function (fromEl, toEl) {
                //     //     if (toEl.tagName === 'INPUT') {
                //     //         toEl.value = fromEl.value;
                //     //     }
                //     // },
                // })
            } else if (topic === "initial-render") {
                console.log("RECV: initial-render", data)
            } else {
                console.error("unknown event", topic, data)
            }
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
