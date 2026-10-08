let handler: (() => Promise<void>) | undefined;
export function registerNoteFlush(flush: () => Promise<void>) { handler=flush;return()=>{if(handler===flush)handler=undefined;}; }
export async function flushLocalNote() { await handler?.(); }
