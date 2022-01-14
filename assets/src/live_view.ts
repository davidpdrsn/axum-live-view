import morphdom from "morphdom"

export interface LiveViewOptions {}

interface State {
  viewState?: Template;
}

export function connectAndRun(options: LiveViewOptions) {
  // only connect if there is a live view on the page
  if (document.getElementById("live-view-container") === null) {
    return
  }

  const socket = new WebSocket(`ws://${window.location.host}${window.location.pathname}`);

  var state: State = {}

  socket.addEventListener("message", (event) => {
    onMessage(socket, event, state)
  })

  socket.addEventListener("close", () => {
    onClose(options)
  })
}

type MessageFromView = InitialRender | Render | JsCommands

interface Template {
  f: string[],
  d: {
    [index: string]: TemplateDynamic
  },
}

type TemplateDynamic = string | Template | TemplateLoop

interface TemplateLoop {
  f: string[],
  b: {
    [index: string]: { [index: string]: TemplateDynamic }
  }
}

interface TemplateDiff {
  f?: string[],
  d?: {
    [index: string]: TemplateDiffDynamic | null
  }
}

type TemplateDiffDynamic = string | TemplateDiff | TemplateDiffLoop

interface TemplateDiffLoop {
  f: string[],
  b: {
    [index: string]: { [index: string]: TemplateDiffDynamic  } | null
  }
}

type InitialRender = {
  t: "i",
  d: Template,
}

type Render = {
  t: "r",
  d: TemplateDiff | null,
}

type JsCommands = {
  t: "j",
  d: JsCommand[],
}

function onMessage(
  socket: WebSocket,
  event: MessageEvent,
  state: State,
) {
  const msg: MessageFromView = JSON.parse(event.data)

  if (msg.t === "i") {
    state.viewState = msg.d
    updateDomFromState(socket, state)
    bindInitialEvents(socket)

  } else if (msg.t === "r") {
    if (!state.viewState) { return }
    if (!msg.d) { return }
    patchTemplate(state.viewState, msg.d)
    updateDomFromState(socket, state)

  } else if (msg.t === "j") {
    for (const jsCommand of msg.d) {
      handleJsCommand(jsCommand)
    }

  } else {
    const _: never = msg
  }
}

function onClose(options: LiveViewOptions) {
  setTimeout(() => {
    connectAndRun(options)
  }, 500)
}

const axm = {
  click: "axm-click",
  input: "axm-input",
  change: "axm-change",
  submit: "axm-submit",
  focus: "axm-focus",
  blur: "axm-blur",
  keydown: "axm-keydown",
  keyup: "axm-keyup",
  window_keydown: "axm-window-keydown",
  window_keyup: "axm-window-keyup",
  window_focus: "axm-window-focus",
  window_blur: "axm-window-blur",
  mouseenter: "axm-mouseenter",
  mouseover: "axm-mouseover",
  mouseleave: "axm-mouseleave",
  mouseout: "axm-mouseout",
  mousemove: "axm-mousemove",
  scroll: "axm-scroll",
}

function bindInitialEvents(socket: WebSocket) {
  const attrs = Object.values(axm).map((attr) => `[${attr}]`).join(", ")

  document.querySelectorAll(attrs).forEach((element) => {
    addEventListeners(socket, element)
  })
}

function addEventListeners(socket: WebSocket, element: Element) {
  if (element.hasAttribute(axm.click)) {
    on(element, "click", axm.click, (msg) => ({ t: "click", m: msg }))
  }

  if (
    element instanceof HTMLInputElement ||
      element instanceof HTMLTextAreaElement ||
      element instanceof HTMLSelectElement
  ) {
    if (element.hasAttribute(axm.input)) {
      on(element, "input", axm.input, (msg) => {
        const value = inputValue(element)
        return { t: "input", m: msg, d: { v: value } }
      })
    }

    if (element.hasAttribute(axm.change)) {
      on(element, "change", axm.change, (msg) => {
        const value = inputValue(element)
        return { t: "input", m: msg, d: { v: value } }
      })
    }

    if (element.hasAttribute(axm.focus)) {
      on(element, "focus", axm.focus, (msg) => {
        const value = inputValue(element)
        return { t: "input", m: msg, d: { v: value } }
      })
    }

    if (element.hasAttribute(axm.blur)) {
      on(element, "blur", axm.blur, (msg) => {
        const value = inputValue(element)
        return { t: "input", m: msg, d: { v: value } }
      })
    }
  }

  if (element instanceof HTMLFormElement) {
    if (element.hasAttribute(axm.change)) {
      on(element, "change", axm.change, (msg) => {
        // workaround for https://github.com/microsoft/TypeScript/issues/30584
        const form = new FormData(element) as any
        const query = new URLSearchParams(form).toString()
        return { t: "form", m: msg, d: { q: query } }
      })
    }

    if (element.hasAttribute(axm.submit)) {
      on(element, "submit", axm.submit, (msg) => {
        // workaround for https://github.com/microsoft/TypeScript/issues/30584
        const form = new FormData(element) as any
        const query = new URLSearchParams(form).toString()
        return { t: "form", m: msg, d: { q: query } }
      })
    }
  }

  [
    ["mouseenter", axm.mouseenter],
    ["mouseover", axm.mouseover],
    ["mouseleave", axm.mouseleave],
    ["mouseout", axm.mouseout],
    ["mousemove", axm.mousemove],
  ].forEach(([event, axm]) => {
    if (!event) { return }
    if (!axm) { return }

    if (element.hasAttribute(axm)) {
      on(element, event, axm, (msg, event) => {
        if (event instanceof MouseEvent) {
          const data: MouseData = {
            cx: event.clientX,
            cy: event.clientY,
            px: event.pageX,
            py: event.pageY,
            ox: event.offsetX,
            oy: event.offsetY,
            mx: event.movementX,
            my: event.movementY,
            sx: event.screenX,
            sy: event.screenY,
          }
          return { t: "mouse", m: msg, d: data }
        } else {
          return
        }
      })
    }
  });

  [
    ["keydown", axm.keydown],
    ["keyup", axm.keyup],
  ].forEach(([event, axm]) => {
    if (!event) { return }
    if (!axm) { return }

    if (element.hasAttribute(axm)) {
      on(element, event, axm, (msg, event) => {
        if (event instanceof KeyboardEvent) {
          if (
            element.hasAttribute("axm-key") &&
            element?.getAttribute("axm-key")?.toLowerCase() !== event.key.toLowerCase()
          ) {
            return;
          }

          const data: KeyData = {
            k: event.key,
            kc: event.code,
            a: event.altKey,
            c: event.ctrlKey,
            s: event.shiftKey,
            me: event.metaKey,
          }
          return { t: "key", m: msg, d: data }
        } else {
          return
        }
      })
    }
  });

  [
    ["keydown", axm.window_keydown],
    ["keyup", axm.window_keyup],
  ].forEach(([event, axm]) => {
    if (!event) { return }
    if (!axm) { return }

    if (element.hasAttribute(axm)) {
      on(window, event, axm, (msg, event) => {
        if (event instanceof KeyboardEvent) {
          if (
            element.hasAttribute("axm-key") &&
            element?.getAttribute("axm-key")?.toLowerCase() !== event.key.toLowerCase()
          ) {
            return;
          }

          const data: KeyData = {
            k: event.key,
            kc: event.code,
            a: event.altKey,
            c: event.ctrlKey,
            s: event.shiftKey,
            me: event.metaKey,
          }
          return { t: "key", m: msg, d: data }
        } else {
          return
        }
      })
    }
  });

  if (element.hasAttribute(axm.window_focus)) {
    on(window, "focus", axm.window_focus, (msg) => {
      return { t: "window_focus", m: msg }
    })
  }

  if (element.hasAttribute(axm.window_blur)) {
    on(window, "blur", axm.window_blur, (msg) => {
      return { t: "window_blur", m: msg }
    })
  }

  if (element.hasAttribute(axm.scroll)) {
    on(document, "scroll", axm.scroll, (msg) => {
      const data = {
        sx: window.scrollX,
        sy: window.scrollY,
      }
      return { t: "scroll", m: msg, d: data }
    })
  }

  function on(
    element: Element | typeof window | typeof document,
    eventName: string,
    attr: string,
    f: (msg: string | JSON, event: Event) => MessageToView | undefined,
  ) {
    element.addEventListener(eventName, delayOrThrottle((event) => {
      if (!(event instanceof KeyboardEvent)) {
        event.preventDefault()
      }

      const decodeMsg = msgAttr(attr)
      if (!decodeMsg) { return }
      const payload = f(decodeMsg, event)
      if (!payload) { return }
      socket.send(JSON.stringify(payload))
    }))
  }

  function msgAttr(attr: string): string | JSON | undefined {
      const value = element.getAttribute(attr)
      if (!value) { return }
      try {
        return JSON.parse(value)
      } catch {
        return value
      }
  }

  function delayOrThrottle<In extends unknown[]>(f: Fn<In>): Fn<In> {
    var delayMs = numberAttr(element, "axm-debounce")
    if (delayMs) {
      return debounce(f, delayMs)
    }

    var delayMs = numberAttr(element, "axm-throttle")
    if (delayMs) {
      return throttle(f, delayMs)
    }

    return f
  }
}

type MessageToView =
  Click
  | Form
  | Input
  | Key
  | WindowFocus
  | WindowBlur
  | Mouse
  | Scroll

interface Click { t: "click", m: string | JSON }

interface WindowFocus { t: "window_focus", m: string | JSON }
interface WindowBlur { t: "window_blur", m: string | JSON }

interface Scroll {
  t: "scroll",
  m: string | JSON,
  d: {
    sx: number,
    sy: number,
  }
}

interface Form {
  t: "form",
  m: string | JSON,
  d: {
    q: string
  }
}

interface Input {
  t: "input",
  m: string | JSON,
  d: {
    v: InputValue
  }
}

interface Key {
  t: "key",
  m: string | JSON,
  d: KeyData,
}

interface KeyData {
  k: string,
  kc: string,
  a: boolean,
  c: boolean,
  s: boolean,
  me: boolean,
}

interface Mouse {
  t: "mouse",
  m: string | JSON,
  d: MouseData,
}

interface MouseData {
  cx: number,
  cy: number,
  px: number,
  py: number,
  ox: number,
  oy: number,
  mx: number,
  my: number,
  sx: number,
  sy: number,
}

type InputValue = string | string[] | boolean

function inputValue(element: Element): InputValue {
  if (element instanceof HTMLTextAreaElement) {
    return element.value

  } else if (element instanceof HTMLInputElement) {
    if (element.getAttribute("type") === "radio" || element.getAttribute("type") === "checkbox") {
      return element.checked
    } else {
      return element.value
    }

  } else if (element instanceof HTMLSelectElement) {
    if (element.hasAttribute("multiple")) {
      return Array.from(element.selectedOptions).map((opt) => opt.value)
    } else {
      return element.value
    }

  } else {
    throw "Input has no input value"
  }
}

function numberAttr(element: Element, attr: string): number | null {
  const value = element.getAttribute(attr)
  if (value) {
    const number = parseInt(value, 10)
    if (number) {
      return number
    }
  }
  return null
}

function updateDomFromState(socket: WebSocket, state: State) {
  if (!state.viewState) { return }
  const html = buildHtml(state.viewState)
  const container = document.querySelector("#live-view-container")
  if (!container) { return }
  patchDom(socket, container, html)

  function buildHtml(template: Template): string {
      var combined = ""
      const fixed = template.f

      fixed.forEach((value, i) => {
        combined = combined.concat(value)
        const templateDyn = template.d[i]

        if (templateDyn === undefined || templateDyn === null) {
          return
        }

        if (typeof templateDyn === "string") {
          combined = combined.concat(templateDyn)

        } else if ("d" in templateDyn) {
          combined = combined.concat(buildHtml(templateDyn))

        } else if ("b" in templateDyn) {
          const fixed = templateDyn.f

          // TODO: make sure we loop over the entries in order here
          Object.values(templateDyn.b).forEach((value) => {
            const nestedTemplate = { f: fixed, d: value }
            combined = combined.concat(buildHtml(nestedTemplate))
          })

        } else {
          const _: never = templateDyn
        }
      })

      return combined
  }

  function patchDom(socket: WebSocket, element: Element, html: string) {
    morphdom(element, html, {
        onNodeAdded: (node) => {
          if (node instanceof Element) {
            addEventListeners(socket, node)
          }
          return node
        },
        onBeforeElUpdated: (fromEl, toEl) => {
          if (fromEl instanceof HTMLInputElement && toEl instanceof HTMLInputElement) {
            if (toEl.getAttribute("type") === "radio" || toEl.getAttribute("type") === "checkbox") {
              toEl.checked = fromEl.checked;
            } else {
              toEl.value = fromEl.value;
            }
          }

          if (fromEl instanceof HTMLTextAreaElement && toEl instanceof HTMLTextAreaElement) {
            toEl.value = fromEl.value;
          }

          if (fromEl instanceof HTMLOptionElement && toEl instanceof HTMLOptionElement) {
            if (toEl.closest("select")?.hasAttribute("multiple")) {
              toEl.selected = fromEl.selected
            }
          }

          if (fromEl instanceof HTMLSelectElement && toEl instanceof HTMLSelectElement && !toEl.hasAttribute("multiple")) {
            toEl.value = fromEl.value
          }

          return true
        },
    })
  }
}

function patchTemplate(template: Template, diff: TemplateDiff) {
  if (diff.f) {
    template.f = diff.f
  }

  if (diff.d && diff.d !== null) {
    patchTemplateDiff(template.d, diff.d)
  }

  function patchTemplateDiff(template: { [index: string]: TemplateDynamic }, diff: { [index: string]: TemplateDiffDynamic | null; }) {
    for (const [key, diffVal] of Object.entries(diff)) {
      if (typeof diffVal === "string") {
        template[key] = diffVal

      } else if (diffVal === null) {
        delete template[key]

      } else if (typeof diffVal === "object") {
        const current = template[key]
        if (current === undefined) { continue }

        if ("d" in diffVal) {
          if (typeof current === "string") {
            template[key] = <TemplateDynamic>diffVal

          } else if ("d" in current) {
            patchTemplate(current, diffVal)

          } else if ("b" in current) {
            console.error("not implemented: b in current")

          } else {
            const _: never = current
          }

        } else if ("b" in diffVal) {
          if (typeof current === "string") {
            template[key] = <TemplateLoop>diffVal

          } else {
            if (!("b" in current)) {
              template[key] = {
                f: current.f,
                b: <{ [index: string]: { [index: string]: TemplateDynamic } }>diffVal.b
              }
            } else {
              patchTemplateLoop(current, diffVal)
            }
          }

        } else {
          console.error("unexpected diff value", diffVal)
        }

      } else {
        const _: never = diffVal
      }
    }
  }

  function patchTemplateLoop(template: TemplateLoop, diff: TemplateDiffLoop) {
    if (diff.f) {
      template.f = diff.f
    }

    if (diff.b) {
      for (const [key, diffVal] of Object.entries(diff.b)) {
        if (diffVal === null) {
          delete template.b[key]

        } else {
          const current = template.b[key]

          if (current === undefined || typeof current === "string") {
            template.b[key] = <{ [index: string]: TemplateDynamic; }>diffVal
          } else {
            patchTemplateDiff(current, diffVal)
          }
        }
      }
    }
  }
}

interface JsCommand {
  delay_ms: number | null,
  kind: JsCommandKind,
}

type JsCommandKind =
  { t: "navigate_to", uri: string }
  | { t: "add_class", selector: string, klass: string }
  | { t: "remove_class", selector: string, klass: string }
  | { t: "toggle_class", selector: string, klass: string }
  | { t: "clear_value", selector: string }
  | { t: "set_title", title: string }
  | { t: "history_push_state", uri: string }

function handleJsCommand(cmd: JsCommand) {
  const run = () => {
    if (cmd.kind.t === "navigate_to") {
      const uri = cmd.kind.uri
      if (uri.startsWith("http")) {
        window.location.href = uri
      } else {
        window.location.pathname = uri
      }

    } else if (cmd.kind.t === "add_class") {
      const { selector, klass } = cmd.kind
      document.querySelectorAll(selector).forEach((element) => {
        element.classList.add(klass)
      })

    } else if (cmd.kind.t === "remove_class") {
      const { selector, klass } = cmd.kind
      document.querySelectorAll(selector).forEach((element) => {
        element.classList.remove(klass)
      })

    } else if (cmd.kind.t === "toggle_class") {
      const { selector, klass } = cmd.kind
      document.querySelectorAll(selector).forEach((element) => {
        element.classList.toggle(klass)
      })

    } else if (cmd.kind.t === "clear_value") {
      const { selector } = cmd.kind
      document.querySelectorAll(selector).forEach((element) => {
        if (element instanceof HTMLInputElement || element instanceof HTMLSelectElement || element instanceof HTMLTextAreaElement) {
          element.value = ""
        }
      })

    } else if (cmd.kind.t === "set_title") {
      document.title = cmd.kind.title

    } else if (cmd.kind.t === "history_push_state") {
      window.history.pushState({}, "", cmd.kind.uri);

    } else {
      const _: never = cmd.kind
    }
  }

  if (cmd.delay_ms) {
    setTimeout(run, cmd.delay_ms)
  } else {
    run()
  }
}

type Fn<
  In extends unknown[],
> = (...args: In) => void;

function debounce<In extends unknown[]>(f: Fn<In>, delayMs: number): Fn<In> {
  var timeout: number
  return (...args) => {
    if (timeout) {
      clearTimeout(timeout)
    }

    timeout = setTimeout(() => {
      f(...args)
    }, delayMs)
  }
}

function throttle<In extends unknown[]>(f: Fn<In>, delayMs: number): Fn<In> {
  var timeout: number | null
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
