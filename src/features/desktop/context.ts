import { inject, type InjectionKey, type Ref } from 'vue';
export const desktopCanvasKey: InjectionKey<Ref<boolean>> = Symbol('desktop-canvas');
export function useDesktopCanvas() { return inject(desktopCanvasKey, null); }
