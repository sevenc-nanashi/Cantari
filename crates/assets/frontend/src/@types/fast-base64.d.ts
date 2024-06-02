declare module 'fast-base64' {
  export function toBase64(base: Uint8Array): Promise<string>;
  export function toBytes(base64: string): Promise<Uint8Array>;
}
