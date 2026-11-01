import { type QueryClient, useMutation, useQuery } from '@tanstack/react-query'
import { ipc } from '../../lib/ipc'
import type { GameStatus, LocationChoice, PickOutcome } from '../../lib/ipc/types'
import { queryKeys } from '../query-keys'

// One scope runs game actions one after another, so a location change and a launch never race.
const GAME_SCOPE = { id: 'game' }

function storeStatus(client: QueryClient, status: GameStatus): void {
  client.setQueryData(queryKeys.gameStatus, status)
}

// A new location means a different Paks folder, so the cached scan no longer applies.
function storeLocationChange(client: QueryClient, status: GameStatus): void {
  storeStatus(client, status)
  void client.invalidateQueries({ queryKey: queryKeys.mods })
}

export function useGameStatus() {
  return useQuery({
    queryKey: queryKeys.gameStatus,
    queryFn: () => ipc.gameStatus(),
    staleTime: Number.POSITIVE_INFINITY,
  })
}

export function useDetectGame() {
  return useMutation({
    scope: GAME_SCOPE,
    mutationFn: () => ipc.gameDetect(),
    onSuccess: (status, _variables, _onMutateResult, { client }) => {
      storeLocationChange(client, status)
    },
  })
}

export function useSetGameLocation() {
  return useMutation({
    scope: GAME_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: (choice: LocationChoice) => ipc.gameSetLocation(choice),
    onSuccess: (status, _choice, _onMutateResult, { client }) => {
      storeLocationChange(client, status)
    },
  })
}

export function usePickGameLocation() {
  return useMutation({
    scope: GAME_SCOPE,
    mutationFn: (title: string) => ipc.gamePickLocation(title),
    onSuccess: (outcome: PickOutcome, _title, _onMutateResult, { client }) => {
      if (outcome.kind === 'selected') storeLocationChange(client, outcome.status)
    },
  })
}

export function useAckUpdate() {
  return useMutation({
    scope: GAME_SCOPE,
    mutationFn: () => ipc.gameAckUpdate(),
    onSuccess: (status, _variables, _onMutateResult, { client }) => {
      storeStatus(client, status)
    },
  })
}

export function useLaunchGame() {
  return useMutation({ scope: GAME_SCOPE, mutationFn: () => ipc.gameLaunch() })
}

export function useRevealPaks() {
  return useMutation({ scope: GAME_SCOPE, mutationFn: () => ipc.gameReveal() })
}
