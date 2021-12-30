export interface LiveViewOptions {
  host: string;
  port: number;
  onSocketOpen: (() => void) | undefined;
  onSocketMessage: (() => void) | undefined;
  onSocketClose: (() => void) | undefined;
  onSocketError: (() => void) | undefined;
}

export const connectAndRun = (options: LiveViewOptions) => {
}

interface AttrDef { attr: string; eventName: string }

const elementLocalAttrs: AttrDef[] = [
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
]

const windowAttrs: AttrDef[] = [
  { attr: "axm-window-keydown", eventName: "keydown" },
  { attr: "axm-window-keyup", eventName: "keyup" },
  { attr: "axm-window-focus", eventName: "focus" },
  { attr: "axm-window-blur", eventName: "blur" },
]
