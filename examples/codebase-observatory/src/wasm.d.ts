declare module "./wasm/trellis_observatory_engine.js" {
  export default function init(): Promise<void>;
  export function initial_state(): string;
  export function dispatch(stateJson: string, actionJson: string): string;
  export function replay(stateJson: string): string;
}
