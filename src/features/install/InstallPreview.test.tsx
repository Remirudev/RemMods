import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Inspection, InspectRequest } from '../../lib/ipc/types'
import type { QueuedSource } from '../../shared/types'
import { makeCandidate, makeInspection, makeIssue } from '../../test/install-fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { InstallModal } from './InstallModal'

const SOURCE: QueuedSource = { kind: 'token', id: 'src-1', name: 'cool-mod.zip', sourceKind: 'zip' }

const EXISTING = {
  modRef: 'm:cool',
  key: 'Standard:Cool',
  rel: 'Mods/Cool',
  relation: 'sameName' as const,
  ambiguous: false,
}

type Answer = (request: InspectRequest) => Inspection

function renderPreview(answer: Answer) {
  const ipc = installIpcMock(undefined, {
    handlers: {
      install_inspect: (payload) => answer((payload as { request: InspectRequest }).request),
    },
  })
  renderWithProviders(
    <InstallModal source={SOURCE} remaining={0} onClose={vi.fn()} onStop={vi.fn()} />,
  )
  return ipc
}

function requests(ipc: ReturnType<typeof installIpcMock>): InspectRequest[] {
  return ipc
    .callsOf('install_inspect')
    .map((call) => (call.payload as { request: InspectRequest }).request)
}

const installButton = () => screen.getByRole('button', { name: 'Install' })

describe('InstallPreview', () => {
  beforeEach(() => vi.restoreAllMocks())

  it('shows the source, the root with its reason and the candidates', async () => {
    renderPreview(() =>
      makeInspection([makeCandidate(0, 'Cool', { inner: 'Cool', shippedOff: 1 })]),
    )
    expect(await screen.findByText('cool-mod.zip')).toBeVisible()
    expect(screen.getByText('ZIP')).toBeVisible()
    expect(screen.getByRole('combobox', { name: 'Install into' })).toHaveValue('Mods')
    expect(screen.getByText('The usual folder for pak mods.')).toBeVisible()
    expect(screen.getByRole('checkbox', { name: 'Install Cool' })).toBeChecked()
    expect(screen.getByText(/Ships switched off: 1/)).toBeVisible()
    expect(screen.queryByRole('combobox', { name: 'File name encoding' })).not.toBeInTheDocument()
    expect(installButton()).toBeEnabled()
  })

  it('offers the encoding only for legacy names and inspects again when it changes', async () => {
    const user = userEvent.setup()
    const ipc = renderPreview((request) =>
      makeInspection([makeCandidate(0, 'Cool')], {
        legacyNames: true,
        encoding: request.encoding,
        issues: [makeIssue('legacyNames', 'info')],
      }),
    )
    await user.click(await screen.findByRole('combobox', { name: 'File name encoding' }))
    await user.click(await screen.findByRole('option', { name: 'Shift_JIS (Japanese)' }))
    await waitFor(() => expect(requests(ipc)).toHaveLength(2))
    expect(requests(ipc)[1]).toMatchObject({ encoding: 'shiftJis', root: null, names: [] })
  })

  it('inspects again with the chosen root', async () => {
    const user = userEvent.setup()
    const ipc = renderPreview((request) =>
      makeInspection([makeCandidate(0, 'Cool')], {
        root: request.root ?? 'standard',
        rootReason: request.root ? 'chosen' : 'default',
      }),
    )
    await user.click(await screen.findByRole('combobox', { name: 'Install into' }))
    await user.click(await screen.findByRole('option', { name: 'LogicMods' }))
    await waitFor(() => expect(requests(ipc)[1]).toMatchObject({ root: 'logic' }))
    expect(await screen.findByText('You chose this folder.')).toBeVisible()
  })

  it('inspects again with the edited name', async () => {
    const user = userEvent.setup()
    const ipc = renderPreview((request) => {
      const name = request.names[0]?.name ?? 'Cool'
      return makeInspection([makeCandidate(0, name)])
    })
    const box = await screen.findByRole('textbox', { name: 'Folder name of Cool' })
    await user.clear(box)
    await user.type(box, 'Better')
    await waitFor(() => expect(requests(ipc)).toHaveLength(2), { timeout: 3000 })
    expect(requests(ipc)[1]?.names).toEqual([{ id: 0, name: 'Better' }])
    expect(await screen.findByRole('textbox', { name: 'Folder name of Better' })).toHaveValue(
      'Better',
    )
  })

  it('offers variants as radios with the first selected', async () => {
    const user = userEvent.setup()
    renderPreview(() =>
      makeInspection(
        [
          makeCandidate(0, 'Cool A', { group: 0 }),
          makeCandidate(1, 'Cool B', { group: 0, selected: false }),
        ],
        { groups: [{ id: 0, candidates: [0, 1] }] },
      ),
    )
    const first = await screen.findByRole('radio', { name: 'Install Cool A' })
    const second = screen.getByRole('radio', { name: 'Install Cool B' })
    expect(first).toBeChecked()
    expect(second).not.toBeChecked()
    await user.click(second)
    expect(second).toBeChecked()
    expect(first).not.toBeChecked()
  })

  it('needs the replace switch before it can start', async () => {
    const user = userEvent.setup()
    renderPreview(() => makeInspection([makeCandidate(0, 'Cool', { existing: EXISTING })]))
    const replace = await screen.findByRole('switch', { name: /Replaces “Mods\/Cool”/ })
    expect(installButton()).toBeDisabled()
    expect(screen.getByText('Confirm which installed mods to replace.')).toBeVisible()
    await user.click(replace)
    expect(installButton()).toBeEnabled()
  })

  it('points an ambiguous match to Repair and stays blocked', async () => {
    renderPreview(() =>
      makeInspection([makeCandidate(0, 'Cool', { existing: { ...EXISTING, ambiguous: true } })]),
    )
    expect(await screen.findByText(/Resolve that in Repair first/)).toBeVisible()
    expect(screen.queryByRole('switch')).not.toBeInTheDocument()
    expect(installButton()).toBeDisabled()
  })

  it('asks which file of a case clash to keep', async () => {
    const user = userEvent.setup()
    renderPreview(() =>
      makeInspection([makeCandidate(0, 'Cool')], {
        clashes: [{ id: 0, paths: ['Cool/A.pak', 'Cool/a.pak'] }],
        issues: [makeIssue('caseClash', 'warn')],
      }),
    )
    const picker = await screen.findByTestId('case-clashes')
    expect(installButton()).toBeDisabled()
    await user.click(within(picker).getByRole('radio', { name: 'Cool/a.pak' }))
    expect(installButton()).toBeEnabled()
  })

  it('lists the issues by severity, blocks first', async () => {
    renderPreview(() =>
      makeInspection([makeCandidate(0, 'Cool')], {
        blocked: true,
        issues: [
          makeIssue('link', 'info', { count: 2 }),
          makeIssue('noPayload', 'block'),
          makeIssue('pathTooLong', 'warn', { count: 3 }),
        ],
      }),
    )
    const list = await screen.findByTestId('install-issues')
    const headings = within(list).getAllByText(/Blocking|Check these|For your information/)
    expect(headings.map((heading) => heading.textContent)).toEqual([
      'Blocking',
      'Check these',
      'For your information',
    ])
    expect(within(list).getByText('Links in the archive are skipped: 2')).toBeVisible()
    expect(installButton()).toBeDisabled()
  })

  it('shows the automatic rename switch only when names need fixing', async () => {
    const user = userEvent.setup()
    renderPreview(() =>
      makeInspection(
        [makeCandidate(0, 'Co:ol', { nameIssue: 'forbidden_char', fixedName: 'Co_ol' })],
        {
          issues: [makeIssue('invalidName', 'warn')],
        },
      ),
    )
    expect(await screen.findByText('Will be renamed to “Co_ol”.')).toBeVisible()
    const rename = screen.getByRole('switch', { name: /Rename automatically/ })
    expect(rename).toBeChecked()
    await user.click(rename)
    expect(installButton()).toBeDisabled()
    expect(
      screen.getByText('Turn on automatic renaming, or rename the mods yourself.'),
    ).toBeVisible()
  })
})
