import { useEffect } from "react";
import type { RefObject } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useDebateStore } from "../stores/debateStore";
import type {
  StreamTokenPayload,
  RoundCompletePayload,
  DebateCompletePayload,
  DebateErrorPayload,
  DebateAbortedPayload,
  DebateModePayload,
} from "../types";

export function useDebateEvents(
  debateId: number | null,
  panelARef: RefObject<HTMLDivElement | null>,
  panelBRef: RefObject<HTMLDivElement | null>,
) {
  useEffect(() => {
    if (debateId === null) return;

    const store = useDebateStore.getState();
    const unlisteners: Promise<UnlistenFn>[] = [];

    const scrollToBottom = (ref: RefObject<HTMLDivElement | null>) => {
      const el = ref.current?.parentElement;
      if (el) {
        el.scrollTop = el.scrollHeight;
      }
    };

    // Stream tokens for model A
    unlisteners.push(
      listen<StreamTokenPayload>("debate:stream:a", (event) => {
        if (event.payload.debate_id !== debateId) return;
        if (panelARef.current) {
          panelARef.current.textContent += event.payload.token;
          scrollToBottom(panelARef);
        }
      }),
    );

    // Stream tokens for model B
    unlisteners.push(
      listen<StreamTokenPayload>("debate:stream:b", (event) => {
        if (event.payload.debate_id !== debateId) return;
        if (panelBRef.current) {
          panelBRef.current.textContent += event.payload.token;
          scrollToBottom(panelBRef);
        }
      }),
    );

    // Round complete
    unlisteners.push(
      listen<RoundCompletePayload>("debate:round_complete", (event) => {
        if (event.payload.debate_id !== debateId) return;
        if (panelARef.current) {
          panelARef.current.textContent += "\n\n";
        }
        if (panelBRef.current) {
          panelBRef.current.textContent += "\n\n";
        }
        useDebateStore.getState().advanceRound(event.payload.round + 1);
      }),
    );

    // Debate complete
    unlisteners.push(
      listen<DebateCompletePayload>("debate:complete", (event) => {
        if (event.payload.debate_id !== debateId) return;
        useDebateStore.getState().complete();
      }),
    );

    // Error
    unlisteners.push(
      listen<DebateErrorPayload>("debate:error", (event) => {
        if (event.payload.debate_id !== debateId) return;
        useDebateStore.getState().setError(event.payload.message);
      }),
    );

    // Aborted
    unlisteners.push(
      listen<DebateAbortedPayload>("debate:aborted", (event) => {
        if (event.payload.debate_id !== debateId) return;
        useDebateStore.getState().abort();
      }),
    );

    // Mode
    unlisteners.push(
      listen<DebateModePayload>("debate:mode", (event) => {
        if (event.payload.debate_id !== debateId) return;
        store.setMode(event.payload.mode);
      }),
    );

    return () => {
      for (const p of unlisteners) {
        void p.then((unlisten) => unlisten());
      }
    };
  }, [debateId, panelARef, panelBRef]);
}
