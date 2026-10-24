import { useMemo } from "react";
import { useQuery, type QueryClient } from "@tanstack/react-query";
import { fetchSuggestions, fetchVocabulary } from "./client";
import type { Suggestion } from "./types";

export const VOCABULARY_KEY = ["vocabulary"] as const;

/** Categories and genre codes. Rarely changes, so it is cached for the session. */
export function useVocabulary() {
  return useQuery({
    queryKey: VOCABULARY_KEY,
    queryFn: fetchVocabulary,
    staleTime: Infinity,
    retry: false,
  }).data;
}

export const SUGGESTIONS_KEY = ["suggestions"] as const;

/** Suggestions for every unclassified work, keyed by game work id. */
export function useSuggestions(): { byWork: Map<string, Suggestion>; loaded: boolean } {
  const query = useQuery({
    queryKey: SUGGESTIONS_KEY,
    queryFn: fetchSuggestions,
    retry: false,
  });
  const data = query.data;
  const byWork = useMemo(() => new Map((data ?? []).map((s) => [s.gameWorkId, s])), [data]);
  return { byWork, loaded: query.isFetched };
}

/** Library rows and suggestions both change whenever a work is edited. */
export function invalidateLibrary(queryClient: QueryClient) {
  return Promise.all([
    queryClient.invalidateQueries({ queryKey: ["library"] }),
    queryClient.invalidateQueries({ queryKey: SUGGESTIONS_KEY }),
  ]);
}
