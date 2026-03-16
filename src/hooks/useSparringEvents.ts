import { useEffect } from "react";
import type { RefObject } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useSparringStore } from "../stores/sparringStore";
import type {
  StreamTokenPayload,
  SparringRoundCompletePayload,
  DebateCompletePayload,
  SparringErrorPayload,
  DebateAbortedPayload,
} from "../types";

export function useSparringEvents(
  debateId: number | null,
  aiPanelRef: RefObject<HTMLDivElement | null>,
) {
  useEffect(() => {
    if (debateId === null) return;

    const unlisteners: Promise<UnlistenFn>[] = [];

    const scrollToBottom = (ref: RefObject<HTMLDivElement | null>) => {
      const el = ref.current?.parentElement;
      if (el) {
        el.scrollTop = el.scrollHeight;
      }
    };

    // Stream tokens
    unlisteners.push(
      listen<StreamTokenPayload>("sparring:stream", (event) => {
        if (event.payload.debate_id !== debateId) return;
        useSparringStore.getState().appendAiToken(event.payload.token);
        if (aiPanelRef.current) {
          aiPanelRef.current.textContent += event.payload.token;
          scrollToBottom(aiPanelRef);
        }
      }),
    );

    // Round complete
    unlisteners.push(
      listen<SparringRoundCompletePayload>("sparring:round_complete", (event) => {
        if (event.payload.debate_id !== debateId) return;
        const p = event.payload;
        useSparringStore.getState().completeRound(
          p.round,
          p.phase,
          p.ai_content,
          p.next_phase,
          p.next_word_limit,
          p.is_complete,
        );
        // Clear streaming ref for next round
        if (aiPanelRef.current) {
          aiPanelRef.current.textContent = "";
        }
      }),
    );

    // Complete
    unlisteners.push(
      listen<DebateCompletePayload>("sparring:complete", (event) => {
        if (event.payload.debate_id !== debateId) return;
        useSparringStore.getState().setComplete();
      }),
    );

    // Error
    unlisteners.push(
      listen<SparringErrorPayload>("sparring:error", (event) => {
        if (event.payload.debate_id !== debateId) return;
        useSparringStore.getState().setError(event.payload.message);
      }),
    );

    // Aborted
    unlisteners.push(
      listen<DebateAbortedPayload>("sparring:aborted", (event) => {
        if (event.payload.debate_id !== debateId) return;
        useSparringStore.getState().setAborted();
      }),
    );

    return () => {
      for (const p of unlisteners) {
        void p.then((unlisten) => unlisten());
      }
    };
  }, [debateId, aiPanelRef]);
}
