/// <reference types="vite/client" />
/// <reference types="vite-plugin-svgr/client" />

declare module "*.css";
declare module "*.png" {
  const src: string;
  export default src;
}
declare module "*.svg" {
  const src: string;
  export default src;
}
declare module "*.jpg" {
  const src: string;
  export default src;
}

declare const __Meety_VERSION__: string;
