import morphdom from "morphdom"

export interface LiveViewOptions {
  host: string;
  port: number;
}

interface State {
  viewState?: ViewState;
}

interface ViewState {
  [index: string]: string | string[] | ViewState;
}

interface ViewStateDiff {
  [index: string]: string | string[] | null | ViewStateDiff;
}

export function connectAndRun(options: LiveViewOptions) {
  const socket = new WebSocket(`ws://${window.location.host}${window.location.pathname}`);

  var state: State = {}

  socket.addEventListener("message", (event) => {
    onMessage(socket, event, state, options)
  })

  socket.addEventListener("close", (event) => {
    onClose(socket, state, options)
  })
}

type MessageFromView = InitialRender | Render | JsCommands

type InitialRender = {
  t: "i",
  d: ViewState,
}

type Render = {
  t: "r",
  d: ViewStateDiff,
}

type JsCommands = {
  t: "j",
  d: JsCommand[],
}

function onMessage(
  socket: WebSocket,
  event: MessageEvent,
  state: State,
  options: LiveViewOptions,
) {
  const msg: MessageFromView = JSON.parse(event.data)

  if (msg.t === "i") {
    state.viewState = msg.d
    updateDomFromState(socket, state)
    bindInitialEvents(socket)

  } else if (msg.t === "r") {
    if (!state.viewState) { return }
    patchViewState(state.viewState, msg.d)
    updateDomFromState(socket, state)

  } else if (msg.t === "j") {
    for (const jsCommand of msg.d) {
      handleJsCommand(jsCommand)
    }

  } else {
    const _: never = msg
  }
}

function onClose(socket: WebSocket, state: State, options: LiveViewOptions) {
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
        return { t: "input_change", m: msg, d: { v: value } }
      })
    }

    if (element.hasAttribute(axm.change)) {
      on(element, "change", axm.change, (msg) => {
        const value = inputValue(element)
        return { t: "input_change", m: msg, d: { v: value } }
      })
    }
  }

  if (
    element instanceof HTMLInputElement ||
      element instanceof HTMLTextAreaElement
  ) {
    if (element.hasAttribute(axm.focus)) {
      on(element, "focus", axm.focus, (msg) => {
        const value = inputValue(element)
        if (typeof value === "string") {
          return { t: "input_focus", m: msg, d: { v: value } }
        } else {
          return
        }
      })
    }

    if (element.hasAttribute(axm.blur)) {
      on(element, "blur", axm.blur, (msg) => {
        const value = inputValue(element)
        if (typeof value === "string") {
          return { t: "input_blur", m: msg, d: { v: value } }
        } else {
          return
        }
      })
    }
  }

  if (element instanceof HTMLFormElement) {
    if (element.hasAttribute(axm.change)) {
      on(element, "change", axm.change, (msg) => {
        // workaround for https://github.com/microsoft/TypeScript/issues/30584
        const form = new FormData(element) as any
        const query = new URLSearchParams(form).toString()
        return { t: "form_change", m: msg, d: { q: query } }
      })
    }

    if (element.hasAttribute(axm.submit)) {
      on(element, "submit", axm.submit, (msg) => {
        // workaround for https://github.com/microsoft/TypeScript/issues/30584
        const form = new FormData(element) as any
        const query = new URLSearchParams(form).toString()
        return { t: "form_submit", m: msg, d: { q: query } }
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
    on(window, "focus", axm.window_focus, (msg, event) => {
      return { t: "window_focus", m: msg }
    })
  }

  if (element.hasAttribute(axm.window_blur)) {
    on(window, "blur", axm.window_blur, (msg, event) => {
      return { t: "window_blur", m: msg }
    })
  }

  if (element.hasAttribute(axm.scroll)) {
    on(document, "scroll", axm.scroll, (msg, event) => {
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
  | FormSubmit
  | FormChange
  | InputChange
  | Key
  | InputFocus
  | InputBlur
  | WindowFocus
  | WindowBlur
  | Mouse
  | Scroll

interface Click { t: "click", m: string | JSON }

interface WindowFocus { t: "window_focus", m: string | JSON }
interface WindowBlur { t: "window_blur", m: string | JSON }

interface InputFocus { t: "input_focus", m: string | JSON, d: { v: string } }
interface InputBlur { t: "input_blur", m: string | JSON, d: { v: string } }

interface Scroll {
  t: "scroll",
  m: string | JSON,
  d: {
    sx: number,
    sy: number,
  }
}

interface FormSubmit {
  t: "form_submit",
  m: string | JSON,
  d: {
    q: string
  }
}

interface FormChange {
  t: "form_change",
  m: string | JSON,
  d: {
    q: string
  }
}

interface InputChange {
  t: "input_change",
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

  function buildHtml(state: ViewState): string {
      var combined = ""

      const f = state[fixed]
      if (!Array.isArray(f)) {
        throw "fixed is not an array"
      }

      f.forEach((value, i) => {
        combined = combined.concat(value)
        const variable = state[i]

        if (variable === undefined || variable === null) {
          return
        }

        if (typeof variable === "string") {
          combined = combined.concat(variable)

        } else if (Array.isArray(variable)) {
          throw "wat"

        } else {
          combined = combined.concat(buildHtml(variable))
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
            const tag = toEl.tagName

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

const fixed = "f";

function patchViewState(state: ViewState, diff: ViewStateDiff) {
  for (const [key, val] of Object.entries(diff)) {
    if (typeof val === "string" || Array.isArray(val)) {
      state[key] = val

    } else if (val === null) {
      delete state[key]

    } else if (typeof val === "object") {
      const nestedState = state[key]

      if (typeof nestedState === "object" && !Array.isArray(nestedState)) {
        patchViewState(nestedState, val)

      } else if (typeof nestedState === "string" || nestedState === undefined) {
        state[key] = <ViewState>val

      } else if (Array.isArray(nestedState)) {
        throw "can this be an array?"

      } else {
        const _: never = nestedState
      }

    } else {
      const _: never = val
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
