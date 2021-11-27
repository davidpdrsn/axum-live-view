class LiveView {
    constructor() {
        this.socket = new WebSocket("ws://localhost:3000/live")
    }

    connect() {
        this.socket.addEventListener("open", () => {
            this.mountComponents()
            this.addEventListeners(document)
        })

        this.socket.addEventListener("message", (event) => {
            const { topic, data } = JSON.parse(event.data)

            if (topic === "rendered") {
                const element = document.querySelector(`[data-liveview-id="${data.liveview_id}"]`)
                window.morphdom(element, data.html, {
                    onBeforeElUpdated: function (fromEl, toEl) {
                        // if (toEl.tagName === 'INPUT') {
                        //     toEl.value = fromEl.value;
                        // }
                    },
                })
            } else {
                console.error("unknown event", msg)
            }
        })
    }

    send(liveviewId, topic, data) {
        let msg = { "liveview_id": liveviewId, topic: topic, data: data }
        console.log("sending message", msg)
        this.socket.send(JSON.stringify(msg))
    }

    mountComponents() {
        document.querySelectorAll("[data-liveview-id]").forEach((component) => {
            let liveviewId = component.getAttribute("data-liveview-id")
            this.send(liveviewId, "axum/mount-liveview", {})
        })
    }

    addEventListeners(element) {
        element.querySelectorAll("[live-click]").forEach((element) => {
            element.addEventListener("click", (event) => {
                event.preventDefault()
                let liveviewId = element.closest('[data-liveview-id]').getAttribute("data-liveview-id")
                let eventName = element.getAttribute("live-click")
                this.send(liveviewId, "axum/live-click", { "event_name": eventName })
            })
        })

        element.querySelectorAll("[live-input]").forEach((element) => {
            element.addEventListener("input", (event) => {
                let liveviewId = element.closest('[data-liveview-id]').getAttribute("data-liveview-id")
                let eventName = element.getAttribute("live-input")
                this.send(liveviewId, "axum/live-input", { "event_name": eventName, "value": element.value })
            })
        })
    }
}

window.addEventListener("DOMContentLoaded", () => {
    let liveView = new LiveView()
    liveView.connect()
});
