import { screen } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import type { AssetEntry, ContainerSummary, ContentSummary, ModContents } from '../../lib/ipc/types'
import { EMPTY_CONTENT, makeModRow, TEST_CONTENTS } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { ContentSection } from './ContentSection'

function container(relPath: string, overrides: Partial<ContainerSummary> = {}): ContainerSummary {
  return { relPath, live: true, status: 'read', version: 5, files: 2, ...overrides }
}

function renderSection(content: ContentSummary, contents: ModContents = TEST_CONTENTS) {
  installIpcMock(undefined, { contents })
  const row = makeModRow('Alpha', { content })
  renderWithProviders(<ContentSection row={row} />)
}

const READ_CONTENT: ContentSummary = {
  ...EMPTY_CONTENT,
  containers: [container('a.utoc')],
  assets: 2,
  characters: [{ code: 'ker', name: 'Kunimitsu', common: false }],
}

const ENTRY: AssetEntry = {
  path: 'Smoke/Alpha.uasset',
  container: 'a.utoc',
  live: true,
  origin: 'replaces',
  character: null,
  slot: null,
  sharedWith: ['m:beta'],
}

describe('ContentSection', () => {
  beforeEach(() => {
    installIpcMock()
  })

  it('summarises assets and containers and lists the entries', async () => {
    renderSection(READ_CONTENT, { ...TEST_CONTENTS, entries: [ENTRY], total: 1 })

    expect(screen.getByText('2 assets in 1 container(s)')).toBeVisible()
    expect(screen.getByText('Kunimitsu')).toBeVisible()
    expect(await screen.findByText('Smoke/Alpha.uasset')).toBeVisible()
    expect(screen.getByText('Replaces')).toBeVisible()
    expect(screen.getByText('Also in 1 mod')).toBeVisible()
  })

  it('names an unsupported container with its status text', () => {
    renderSection({
      ...EMPTY_CONTENT,
      containers: [container('enc.utoc', { status: 'encrypted', version: null, files: 0 })],
    })

    expect(screen.getByText(/enc\.utoc/)).toBeVisible()
    expect(screen.getByText(/Unsupported: Encrypted: contents cannot be listed/)).toBeVisible()
    expect(screen.queryByText(/assets? in/)).toBeNull()
  })

  it('says so when the mod has no .utoc files', () => {
    renderSection(EMPTY_CONTENT)

    expect(screen.getByText('This mod has no .utoc files.')).toBeVisible()
  })

  it('explains the missing base when it is not complete', async () => {
    renderSection(READ_CONTENT, { ...TEST_CONTENTS, base: 'partial' })

    expect(await screen.findByText(/Whether a file adds or replaces/)).toBeVisible()
  })

  it('says when the list is cut off', async () => {
    renderSection(READ_CONTENT, {
      ...TEST_CONTENTS,
      entries: [ENTRY],
      total: 9000,
      truncated: true,
    })

    expect(await screen.findByText('Only the first 1 of 9000 files are listed.')).toBeVisible()
  })
})
