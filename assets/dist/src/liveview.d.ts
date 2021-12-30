export interface LiveViewOptions {
    host: string;
    port: number;
    onSocketOpen: (() => void) | undefined;
    onSocketMessage: (() => void) | undefined;
    onSocketClose: (() => void) | undefined;
    onSocketError: (() => void) | undefined;
}
export declare function connectAndRun(options: LiveViewOptions): void;
